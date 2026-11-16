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
