use fin_app::model::Completer;
use fin_app::App;
use fin_core::NaiveDate;
use std::sync::{Arc, Mutex};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// Answers every task from a script and remembers what it was asked.
struct Script {
    reply: String,
    asked: Asked,
}
impl Completer for Script {
    fn complete(&self, p: &str) -> Result<String, String> {
        self.complete_task("whatif", p)
    }
    fn complete_task(&self, task: &str, prompt: &str) -> Result<String, String> {
        self.asked
            .lock()
            .unwrap()
            .push((task.into(), prompt.into()));
        Ok(self.reply.clone())
    }
}

const CSV: &str = "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n10/06/2026,JOES TAQUERIA 4111111111111111,-18.40\n10/07/2026,MYSTERY CO,-9.99\n";

type Asked = Arc<Mutex<Vec<(String, String)>>>;

fn app_with(reply: &str) -> (App, Asked) {
    let mut app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 16));
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    let asked = Arc::new(Mutex::new(vec![]));
    app.set_embedded_model(Some(Arc::new(Script {
        reply: reply.into(),
        asked: asked.clone(),
    })));
    (app, asked)
}

fn uncategorized(app: &App) -> Vec<(i64, String)> {
    app.transactions("2000-01-01", "2026-12-31", true)
        .unwrap()
        .into_iter()
        .map(|t| (t.id, t.payee))
        .collect()
}

#[test]
fn no_model_says_so_instead_of_guessing() {
    let mut app = App::in_memory().unwrap();
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    assert!(app
        .suggest_categories()
        .unwrap_err()
        .contains("No model is set up"));
}

#[test]
fn suggestions_are_checked_and_nothing_is_saved() {
    let (app, asked) = app_with("{}");
    let todo = uncategorized(&app);
    assert!(!todo.is_empty());
    let taqueria = todo.iter().find(|(_, p)| p.contains("TAQUERIA")).unwrap().0;
    let mystery = todo.iter().find(|(_, p)| p.contains("MYSTERY")).unwrap().0;
    let reply = format!(
        r#"{{"items":[{{"id":{taqueria},"category":"dining OUT"}},{{"id":{mystery},"category":"Made Up Category"}},{{"id":9999,"category":"Dining out"}},{{"id":{taqueria},"category":"Shopping"}}]}}"#
    );
    let (app, asked2) = app_with(&reply);
    let s = app.suggest_categories().unwrap();
    // Unknown category, unknown transaction and a duplicate are all dropped.
    assert_eq!(s.len(), 1);
    assert_eq!((s[0].id, s[0].category.as_str()), (taqueria, "Dining out"));
    // Nothing was saved: both lines are still uncategorized.
    assert_eq!(uncategorized(&app).len(), todo.len());
    // The model was asked as task "categorize".
    assert_eq!(asked2.lock().unwrap()[0].0, "categorize");
    let _ = asked;
}

#[test]
fn the_prompt_holds_descriptions_only() {
    let (app, asked) = app_with(r#"{"items":[]}"#);
    assert!(app.suggest_categories().unwrap().is_empty());
    let prompt = asked.lock().unwrap()[0].1.clone();
    assert!(prompt.contains("TAQUERIA") && prompt.contains("MYSTERY CO"));
    assert!(prompt.contains("#") && !prompt.contains("4111111111111111"));
    assert!(
        !prompt.contains("18.40") && !prompt.contains("2026-10-06") && !prompt.contains("5000")
    );
}

#[test]
fn garbage_replies_give_no_suggestions() {
    for bad in [
        "not json",
        "{}",
        r#"{"items":"x"}"#,
        r#"{"items":[{"id":"a"}]}"#,
    ] {
        let (app, _) = app_with(bad);
        assert!(app.suggest_categories().unwrap().is_empty(), "{bad}");
    }
}

#[test]
fn accepting_a_suggestion_uses_the_normal_edit_path() {
    let (mut app, _) = app_with("{}");
    let id = uncategorized(&app)[0].0;
    app.set_category(id, "Dining out", true).unwrap();
    assert!(uncategorized(&app).iter().all(|(i, _)| *i != id));
}

const OFX: &str = "OFXHEADER:100\nDATA:OFXSGML\n<OFX>\n<FI><ORG>Chase Bank</FI>\n<ACCTID>00123456784821\n<STMTTRN>\n<DTPOSTED>20261006\n<TRNAMT>-18.40\n<NAME>JOES TAQUERIA\n</STMTTRN>\n</OFX>\n";

fn filed(reply: &str, account: &str) -> (String, Asked) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::in_memory()
        .unwrap()
        .with_library(Some(dir.path().to_path_buf()));
    let asked = Arc::new(Mutex::new(vec![]));
    app.set_embedded_model(Some(Arc::new(Script {
        reply: reply.into(),
        asked: asked.clone(),
    })));
    let r = app
        .import_file("download.qfx", OFX.as_bytes(), account, "checking")
        .unwrap();
    (r.filed_as.unwrap(), asked)
}

#[test]
fn a_statement_is_named_for_its_bank_and_last_four() {
    let (name, asked) = filed(r#"{"institution":"Chase Bank","last4":"4821"}"#, "Checking");
    assert_eq!(
        name,
        "2026/Checking/2026-10 Chase Bank Checking 4821 statement.qfx"
    );
    let asked = asked.lock().unwrap();
    assert_eq!(asked[0].0, "name_statement");
    // The model saw the bank and masked account number, not the transaction or the full number.
    assert!(asked[0].1.contains("Chase Bank") && asked[0].1.contains("…4821"));
    assert!(
        !asked[0].1.contains("00123456784821")
            && !asked[0].1.contains("18.40")
            && !asked[0].1.contains("TAQUERIA")
    );
}

#[test]
fn made_up_names_and_digits_are_refused() {
    // A bank the text never mentions, and digits that are not in it, are ignored.
    let (name, _) = filed(
        r#"{"institution":"Wells Fargo","last4":"9999"}"#,
        "Checking",
    );
    assert_eq!(name, "2026/Checking/2026-10 Checking statement.qfx");
    let (name, _) = filed("garbage", "Checking");
    assert_eq!(name, "2026/Checking/2026-10 Checking statement.qfx");
}

#[test]
fn a_name_the_person_typed_is_kept() {
    let (name, _) = filed(
        r#"{"institution":"Chase Bank","last4":"4821"}"#,
        "My Joint Account",
    );
    assert_eq!(
        name,
        "2026/Checking/2026-10 My Joint Account Checking 4821 statement.qfx"
    );
}
