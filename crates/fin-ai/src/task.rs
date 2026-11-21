//! The task harness: a versioned prompt, a JSON Schema for the answer, and a runner that
//! renders the prompt, constrains the model with a grammar compiled from the schema, parses and
//! validates the output, and retries once with the error when the model still gets it wrong.
//! Ported from CeeVee's `ceevee-ai` task framework (blocking, since Golden's chat is).
//!
//! Prompt text is compiled in. Change a prompt's text, bump its `version`.

use serde_json::Value;

use crate::backend::GenerateRequest;
use crate::grammar::json_schema_to_gbnf;
use crate::validate::validate;
use crate::worker::{ModelWorker, Priority};
use crate::{Error, Result};

/// How many times the model is asked before the task fails.
pub const MAX_ATTEMPTS: u32 = 2;

pub struct TaskSpec {
    pub id: &'static str,
    pub version: u32,
    /// Prompt text with `{{name}}` placeholders.
    pub prompt: &'static str,
    /// JSON Schema for the answer, as JSON text.
    pub schema: &'static str,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskOutput {
    /// The model's raw text for the accepted attempt (valid against the schema).
    pub raw: String,
    pub attempts: u32,
    pub task_id: &'static str,
    pub version: u32,
}

/// Fill `{{name}}` placeholders. A missing variable or an unclosed placeholder is an error, so a
/// typo never reaches the model. Values are inserted as is and are not scanned again.
pub fn render(template: &str, vars: &[(&str, &str)]) -> Result<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .ok_or_else(|| Error::Invalid("unclosed {{ in prompt template".into()))?;
        let name = after[..end].trim();
        let value = vars
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| *v)
            .ok_or_else(|| Error::Invalid(format!("prompt variable \"{name}\" not provided")))?;
        out.push_str(value);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

impl TaskSpec {
    pub fn schema_value(&self) -> Result<Value> {
        serde_json::from_str(self.schema)
            .map_err(|e| Error::Invalid(format!("task {} schema is not JSON: {e}", self.id)))
    }

    pub fn grammar(&self) -> Result<String> {
        json_schema_to_gbnf(&self.schema_value()?)
    }
}

/// Parse model text and check it against the schema. The error is worded for a retry prompt.
pub fn check_output(schema: &Value, raw: &str) -> std::result::Result<Value, String> {
    let value: Value =
        serde_json::from_str(raw.trim()).map_err(|e| format!("not valid JSON: {e}"))?;
    validate(schema, &value)?;
    Ok(value)
}

/// Run a task on the model worker, blocking until it is done.
pub fn run_blocking(
    worker: &ModelWorker,
    spec: &TaskSpec,
    vars: &[(&str, &str)],
    priority: Priority,
) -> Result<TaskOutput> {
    let schema = spec.schema_value()?;
    let grammar = spec.grammar()?;
    let base = render(spec.prompt, vars)?;
    let mut prompt = base.clone();
    let mut last_problem = String::new();
    for attempt in 1..=MAX_ATTEMPTS {
        let mut req = GenerateRequest::new(prompt.clone());
        req.grammar = Some(grammar.clone());
        req.max_tokens = spec.max_tokens;
        req.temperature = spec.temperature;
        let raw = worker
            .submit(priority, req, None)
            .result
            .blocking_recv()
            .map_err(|_| Error::Other("model worker stopped".into()))??;
        match check_output(&schema, &raw) {
            Ok(_) => {
                return Ok(TaskOutput {
                    raw,
                    attempts: attempt,
                    task_id: spec.id,
                    version: spec.version,
                })
            }
            Err(problem) => {
                prompt = format!(
                    "{base}\n\nYour previous answer was rejected: {problem}\nAnswer again with JSON that fixes this."
                );
                last_problem = problem;
            }
        }
    }
    Err(Error::Invalid(format!(
        "task {} failed after {MAX_ATTEMPTS} attempts: {last_problem}",
        spec.id
    )))
}

/// Turn a what-if question into plan changes. The caller (fin-app) builds the full prompt,
/// which holds only first names, retirement ages and the allowed choices, and still re-checks
/// every change before it reaches the plan.
pub const WHATIF: TaskSpec = TaskSpec {
    id: "whatif",
    version: 1,
    prompt: "{{input}}",
    schema: include_str!("../prompts/whatif.schema.json"),
    max_tokens: 400,
    temperature: 0.0,
};

#[cfg(test)]
mod tests;
