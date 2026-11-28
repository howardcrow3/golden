use fin_app::App;
use fin_core::NaiveDate;
use serde_json::Value;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

const CSV: &str = "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n10/06/2026,\"SMITH, JONES & CO\",-10.50\n09/02/2019,OLD COFFEE,-3.25\n";

fn loaded() -> App {
    let mut app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 16));
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    app
}

fn parse(app: &App) -> Value {
    serde_json::from_str(&app.backup_json().unwrap()).unwrap()
}

#[test]
fn backup_holds_every_transaction_oldest_first_even_old_ones() {
    let v = parse(&loaded());
    assert_eq!(v["app"], "Golden");
    assert_eq!(v["exportedOn"], "2026-11-16");
    let t = v["transactions"].as_array().unwrap();
    assert_eq!(t.len(), 3);
    assert_eq!(t[0]["day"], "2019-09-02");
    assert_eq!(t[0]["amountCents"], -325);
    assert_eq!(t[1]["payee"], "ACME PAYROLL");
    assert_eq!(t[1]["account"], "Checking");
    assert_eq!(t[1]["category"], "Salary");
    assert_eq!(v["accounts"][0]["name"], "Checking");
}

#[test]
fn backup_includes_budgets_and_household_as_real_json() {
    let app = loaded();
    app.set_budget("Food", 40_000).unwrap();
    let v = parse(&app);
    assert_eq!(v["settings"]["budgets"]["monthly"]["Food"], 40_000);
}

#[test]
fn backup_never_includes_licence_or_trial_data() {
    let app = loaded();
    app.db()
        .set_setting("license_token", "SECRET-TOKEN-123")
        .unwrap();
    app.db().set_setting("first_run", "2026-11-01").unwrap();
    let text = app.backup_json().unwrap();
    assert!(!text.contains("SECRET-TOKEN-123"));
    assert!(!text.contains("license"));
    assert!(!text.contains("first_run"));
}

#[test]
fn an_empty_database_still_backs_up() {
    let app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 16));
    let v = parse(&app);
    assert!(v["transactions"].as_array().unwrap().is_empty());
}

#[test]
fn file_is_written_numbered_and_works_after_the_trial() {
    let app = loaded();
    app.db().set_setting("first_run", "2026-01-01").unwrap();
    assert!(!app.license_status(d(2026, 11, 16)).unwrap().can_edit);
    let dir = std::env::temp_dir().join(format!("golden-backup-test-{}", std::process::id()));
    let a = app.export_backup(&dir).unwrap();
    let b = app.export_backup(&dir).unwrap();
    assert_eq!(a.file_name().unwrap(), "golden-backup-2026-11-16.json");
    assert_eq!(b.file_name().unwrap(), "golden-backup-2026-11-16 (2).json");
    let back: Value = serde_json::from_str(&std::fs::read_to_string(&a).unwrap()).unwrap();
    assert_eq!(back["transactions"].as_array().unwrap().len(), 3);
    std::fs::remove_dir_all(&dir).ok();
}

fn blank() -> App {
    App::in_memory().unwrap().with_clock(|| d(2026, 11, 16))
}

#[test]
fn restore_rebuilds_a_fresh_install() {
    let src = loaded();
    src.set_budget("Food", 40_000).unwrap();
    let text = src.backup_json().unwrap();
    let mut fresh = blank();
    let r = fresh.restore_backup(&text).unwrap();
    assert_eq!((r.accounts, r.added, r.skipped_duplicates), (1, 3, 0));
    assert_eq!(r.settings_restored, ["budgets"]);
    assert_eq!(fresh.budgets().unwrap().monthly["Food"], 40_000);
    // Same transactions and categories (row ids are new in a fresh database).
    let key = |app: &App| -> Vec<_> {
        app.transactions("2019-01-01", "2026-12-31", false)
            .unwrap()
            .into_iter()
            .map(|t| (t.day, t.payee, t.amount_cents, t.category, t.account))
            .collect()
    };
    assert_eq!(key(&fresh), key(&src));
}

#[test]
fn restoring_twice_changes_nothing() {
    let text = loaded().backup_json().unwrap();
    let mut app = blank();
    app.restore_backup(&text).unwrap();
    let r = app.restore_backup(&text).unwrap();
    assert_eq!((r.added, r.skipped_duplicates), (0, 3));
    assert!(r.settings_restored.is_empty());
}

#[test]
fn restore_never_overwrites_existing_settings() {
    let src = loaded();
    src.set_budget("Food", 40_000).unwrap();
    let text = src.backup_json().unwrap();
    let mut app = blank();
    app.set_budget("Food", 999).unwrap();
    let r = app.restore_backup(&text).unwrap();
    assert!(r.settings_restored.is_empty());
    assert_eq!(app.budgets().unwrap().monthly["Food"], 999);
}

#[test]
fn restore_round_trips_remembered_category_rules() {
    let mut src = loaded();
    let todo = src.transactions("2019-01-01", "2026-12-31", true).unwrap();
    // The CSV's "SMITH, JONES" line is uncategorized; teach a rule and back up.
    let id = todo.first().map(|t| t.id).unwrap();
    src.set_category(id, "Gifts", true).unwrap();
    let mut fresh = blank();
    let r = fresh.restore_backup(&src.backup_json().unwrap()).unwrap();
    assert!(r.settings_restored.contains(&"user_rules".to_string()));
}

#[test]
fn bad_files_are_refused_with_a_readable_reason() {
    let mut app = blank();
    assert!(app
        .restore_backup("not json")
        .unwrap_err()
        .contains("isn't a Golden backup"));
    assert!(app
        .restore_backup("{\"app\":\"Other\"}")
        .unwrap_err()
        .contains("isn't a Golden backup"));
    let future = r#"{"app":"Golden","backupVersion":9}"#;
    assert!(app
        .restore_backup(future)
        .unwrap_err()
        .contains("version 9"));
    let no_amount = r#"{"app":"Golden","backupVersion":1,"accounts":[{"name":"A","kind":"checking"}],"transactions":[{"account":"A","day":"2026-01-01"}]}"#;
    assert!(app
        .restore_backup(no_amount)
        .unwrap_err()
        .contains("amount"));
}

#[test]
fn restore_is_blocked_after_the_trial() {
    let text = loaded().backup_json().unwrap();
    let mut app = blank();
    app.restore_backup(&text).unwrap();
    app.db().set_setting("first_run", "2026-01-01").unwrap();
    assert!(app.restore_backup(&text).is_err());
}

#[test]
fn restore_keeps_a_refund_cancelling_its_purchase() {
    let mut src = blank();
    src.import_statement(
        b"Date,Description,Amount\n11/01/2026,ACME PAYROLL,3000.00\n11/03/2026,TARGET #1234,-80.00\n11/10/2026,TARGET RETURN,80.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    let nov = |a: &App| {
        let m = a
            .cash_flow(d(2026, 11, 1), d(2026, 11, 30), "month")
            .unwrap()
            .remove(0);
        (m.income_cents, m.spending_cents)
    };
    assert_eq!(nov(&src), (300_000, 0));
    let mut fresh = blank();
    fresh.restore_backup(&src.backup_json().unwrap()).unwrap();
    assert_eq!(nov(&fresh), (300_000, 0));
}

fn smith(app: &App) -> fin_app::transactions::TxnRow {
    app.transactions("2019-01-01", "2026-12-31", false)
        .unwrap()
        .into_iter()
        .find(|t| t.payee == "SMITH, JONES & CO")
        .unwrap()
}

#[test]
fn restore_brings_back_notes_tags_and_splits() {
    use fin_app::transactions::SplitPart;
    let mut src = loaded();
    let id = smith(&src).id;
    src.set_note_tags(id, "lawyer, paid in full", "legal, Tax")
        .unwrap();
    let parts = vec![
        SplitPart {
            category: "Legal".into(),
            cents: -800,
        },
        SplitPart {
            category: "Postage".into(),
            cents: -250,
        },
    ];
    src.set_splits(id, parts.clone()).unwrap();
    let text = src.backup_json().unwrap();
    let mut fresh = blank();
    fresh.restore_backup(&text).unwrap();
    let back = smith(&fresh);
    assert_eq!(back.note, "lawyer, paid in full");
    assert_eq!(back.tags, vec!["legal", "tax"]);
    assert_eq!(back.splits, parts);
    // Restoring again changes nothing.
    fresh.restore_backup(&text).unwrap();
    assert_eq!(smith(&fresh).splits, parts);
}

#[test]
fn backup_carries_every_setting_the_person_made() {
    let src = loaded();
    src.save_watch_lists(vec![fin_app::watchlists::WatchList {
        name: "Coffee".into(),
        query: "coffee".into(),
    }])
    .unwrap();
    src.save_holdings(&[fin_plan::investments::Holding {
        name: "Total market".into(),
        value_cents: 5_000_000,
        class: fin_plan::investments::Class::Stock,
        expense_bp: 3,
        single_stock: false,
    }])
    .unwrap();
    src.close_month("2026-10").unwrap();
    // Tracking that was stopped is stored as an empty value.
    src.stop_progress_tracking().unwrap();
    let mut fresh = blank();
    fresh.restore_backup(&src.backup_json().unwrap()).unwrap();
    assert_eq!(fresh.watch_lists().unwrap(), src.watch_lists().unwrap());
    assert_eq!(fresh.holdings().unwrap(), src.holdings().unwrap());
    assert_eq!(fresh.closed_months().unwrap(), vec!["2026-10".to_string()]);
    assert_eq!(fresh.progress().unwrap(), None);
}
