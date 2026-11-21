use super::*;
use crate::backend::LlmBackend;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Replies from a script, one per request; records the prompts and grammars it was given.
struct Script {
    replies: Vec<&'static str>,
    seen: Seen,
    loaded: bool,
}

impl LlmBackend for Script {
    fn load(&mut self) -> Result<()> {
        self.loaded = true;
        Ok(())
    }
    fn unload(&mut self) {
        self.loaded = false;
    }
    fn is_loaded(&self) -> bool {
        self.loaded
    }
    fn generate(
        &mut self,
        req: &GenerateRequest,
        _: &AtomicBool,
        _: &mut dyn FnMut(&str),
    ) -> Result<String> {
        self.seen
            .lock()
            .unwrap()
            .push((req.prompt.clone(), req.grammar.is_some()));
        Ok(self.replies.remove(0).to_string())
    }
}

type Seen = Arc<Mutex<Vec<(String, bool)>>>;

fn rig(replies: Vec<&'static str>) -> (ModelWorker, Seen) {
    let seen = Arc::new(Mutex::new(vec![]));
    let w = ModelWorker::spawn(
        Script {
            replies,
            seen: seen.clone(),
            loaded: false,
        },
        Duration::from_secs(60),
    );
    (w, seen)
}

const GOOD: &str = r#"{"changes":[{"kind":"retireAt","person":0,"age":60}]}"#;

#[test]
fn render_fills_placeholders_and_refuses_typos() {
    assert_eq!(
        render("a {{x}} b {{ y }}", &[("x", "1"), ("y", "2")]).unwrap(),
        "a 1 b 2"
    );
    assert!(render("{{missing}}", &[]).is_err());
    assert!(render("oops {{", &[]).is_err());
    // A value that looks like a placeholder is not expanded again.
    assert_eq!(render("{{x}}", &[("x", "{{y}}")]).unwrap(), "{{y}}");
}

#[test]
fn the_whatif_schema_compiles_to_a_grammar() {
    let g = WHATIF.grammar().unwrap();
    assert!(g.contains("root ::="));
    assert!(g.contains("retireAt") && g.contains("setPick"));
    assert_eq!(WHATIF.version, 1);
}

#[test]
fn the_schema_accepts_good_changes_and_refuses_bad_ones() {
    let schema = WHATIF.schema_value().unwrap();
    assert!(check_output(&schema, GOOD).is_ok());
    assert!(check_output(
        &schema,
        r#"{"changes":[{"kind":"spending","cents":8000000}]}"#
    )
    .is_ok());
    for bad in [
        r#"{"changes":[]}"#,
        r#"{"changes":[{"kind":"retireAt","person":0,"age":20}]}"#,
        r#"{"changes":[{"kind":"spending","cents":-5}]}"#,
        r#"{"changes":[{"kind":"fly"}]}"#,
        r#"{"nope":1}"#,
        "not json",
    ] {
        assert!(check_output(&schema, bad).is_err(), "{bad}");
    }
}

#[test]
fn a_good_first_answer_is_used_with_the_grammar_on() {
    let (w, seen) = rig(vec![GOOD]);
    let out = run_blocking(
        &w,
        &WHATIF,
        &[("input", "retire at 60")],
        Priority::Interactive,
    )
    .unwrap();
    assert_eq!(
        (out.raw.as_str(), out.attempts, out.task_id, out.version),
        (GOOD, 1, "whatif", 1)
    );
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert!(seen[0].0 == "retire at 60" && seen[0].1);
}

#[test]
fn a_bad_answer_is_retried_once_with_the_reason() {
    let (w, seen) = rig(vec![
        r#"{"changes":[{"kind":"retireAt","person":0,"age":20}]}"#,
        GOOD,
    ]);
    let out = run_blocking(
        &w,
        &WHATIF,
        &[("input", "retire at 60")],
        Priority::Interactive,
    )
    .unwrap();
    assert_eq!(out.attempts, 2);
    let seen = seen.lock().unwrap();
    assert!(
        seen[1].0.contains("previous answer was rejected")
            && seen[1].0.contains("exactly one shape")
    );
}

#[test]
fn two_bad_answers_fail_the_task() {
    let (w, _) = rig(vec!["nope", "still nope"]);
    let e = run_blocking(&w, &WHATIF, &[("input", "x")], Priority::Interactive).unwrap_err();
    assert!(e.to_string().contains("failed after 2 attempts"));
}
