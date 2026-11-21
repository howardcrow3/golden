//! The in-process model, ready to answer a question with JSON: one worker thread owns the
//! model, loads it on first use and unloads it after a quiet spell.

use std::time::Duration;

use crate::backend::{GenerateRequest, LlmBackend};
use crate::worker::{ModelWorker, Priority};
use crate::{Error, Result};

/// How long the model stays in memory with nothing to do.
pub const IDLE_UNLOAD: Duration = Duration::from_secs(600);

/// A grammar that only lets the model write one JSON object, so a reply always parses.
pub const JSON_OBJECT_GRAMMAR: &str = r#"root ::= ws object ws
object ::= "{" ws ( pair ( ws "," ws pair )* )? ws "}"
pair ::= string ws ":" ws value
array ::= "[" ws ( value ( ws "," ws value )* )? ws "]"
value ::= object | array | string | number | "true" | "false" | "null"
string ::= "\"" ( [^"\\\x00-\x1f] | "\\" ["\\/bfnrt] | "\\u" [0-9a-fA-F]{4} )* "\""
number ::= "-"? ( "0" | [1-9] [0-9]* ) ( "." [0-9]+ )? ( [eE] [-+]? [0-9]+ )?
ws ::= [ \t\n]*
"#;

#[derive(Clone)]
pub struct Engine {
    worker: ModelWorker,
}

impl Engine {
    pub fn new(backend: impl LlmBackend, idle_unload: Duration) -> Self {
        Self {
            worker: ModelWorker::spawn(backend, idle_unload),
        }
    }

    /// Run a harness task (versioned prompt, schema-built grammar, validation, one retry).
    /// Blocks until done; call it off the UI thread.
    pub fn run_task(
        &self,
        spec: &crate::task::TaskSpec,
        vars: &[(&str, &str)],
    ) -> Result<crate::task::TaskOutput> {
        crate::task::run_blocking(&self.worker, spec, vars, Priority::Interactive)
    }

    /// Ask for one JSON object. Blocks until the model answers; call it off the UI thread.
    pub fn complete_json(&self, prompt: &str) -> Result<String> {
        let mut req = GenerateRequest::new(prompt);
        req.grammar = Some(JSON_OBJECT_GRAMMAR.to_owned());
        req.max_tokens = 512;
        req.temperature = 0.0;
        let job = self.worker.submit(Priority::Interactive, req, None);
        job.result
            .blocking_recv()
            .map_err(|_| Error::Other("the model stopped before answering".into()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    struct Echo(bool);
    impl LlmBackend for Echo {
        fn load(&mut self) -> Result<()> {
            self.0 = true;
            Ok(())
        }
        fn unload(&mut self) {
            self.0 = false;
        }
        fn is_loaded(&self) -> bool {
            self.0
        }
        fn generate(
            &mut self,
            req: &GenerateRequest,
            _: &AtomicBool,
            _: &mut dyn FnMut(&str),
        ) -> Result<String> {
            assert_eq!(req.grammar.as_deref(), Some(JSON_OBJECT_GRAMMAR));
            assert_eq!(req.temperature, 0.0);
            Ok(format!("{{\"echo\":{}}}", req.prompt.len()))
        }
    }

    struct Broken;
    impl LlmBackend for Broken {
        fn load(&mut self) -> Result<()> {
            Err(Error::NotFound("model file".into()))
        }
        fn unload(&mut self) {}
        fn is_loaded(&self) -> bool {
            false
        }
        fn generate(
            &mut self,
            _: &GenerateRequest,
            _: &AtomicBool,
            _: &mut dyn FnMut(&str),
        ) -> Result<String> {
            unreachable!()
        }
    }

    #[test]
    fn asks_for_json_with_the_grammar() {
        let e = Engine::new(Echo(false), IDLE_UNLOAD);
        assert_eq!(e.complete_json("hello").unwrap(), "{\"echo\":5}");
        assert_eq!(e.complete_json("hi").unwrap(), "{\"echo\":2}"); // loaded once, reused
    }

    #[test]
    fn a_missing_model_is_an_error_not_a_hang() {
        let e = Engine::new(Broken, IDLE_UNLOAD);
        assert!(e
            .complete_json("x")
            .unwrap_err()
            .to_string()
            .contains("model file"));
    }

    #[test]
    fn the_grammar_names_a_root() {
        assert!(JSON_OBJECT_GRAMMAR.starts_with("root ::="));
    }
}
