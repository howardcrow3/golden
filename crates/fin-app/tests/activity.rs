use fin_app::model::Completer;
use fin_app::App;
use fin_core::NaiveDate;
use std::sync::Arc;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}

struct Fake(Result<String, String>);
impl Completer for Fake {
    fn complete(&self, _p: &str) -> Result<String, String> {
        self.0.clone()
    }
}

#[test]
fn every_run_is_logged_with_sizes_but_not_text() {
    let mut app = App::in_memory().unwrap();
    assert!(app.ai_activity().is_empty());
    app.set_embedded_model(Some(Arc::new(Fake(Ok("{\"changes\":[]}".into())))));
    // The what-if chat goes through the same model.
    let _ = app.prepare_whatif("what if inflation is 4%", today());
    let log = app.ai_activity();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].task, "whatif");
    assert!(log[0].ok);
    assert!(log[0].prompt_chars > 0);
    assert_eq!(log[0].reply_chars, 14);
    let json = serde_json::to_string(&log).unwrap();
    assert!(
        !json.contains("inflation"),
        "the prompt text must not be kept"
    );
}

#[test]
fn failures_are_logged_and_the_log_can_be_cleared() {
    let mut app = App::in_memory().unwrap();
    app.set_embedded_model(Some(Arc::new(Fake(Err("model crashed".into())))));
    let _ = app.prepare_whatif("what if I retire at 60", today());
    let log = app.ai_activity();
    assert!(log
        .iter()
        .any(|e| !e.ok && e.error.as_deref() == Some("model crashed")));
    app.clear_ai_activity();
    assert!(app.ai_activity().is_empty());
}

#[test]
fn no_model_means_no_entries() {
    let app = App::in_memory().unwrap();
    let _ = app.prepare_whatif("what if I retire at 60", today());
    assert!(app.ai_activity().is_empty());
}
