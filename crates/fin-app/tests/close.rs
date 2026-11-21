use fin_app::App;
use fin_core::NaiveDate;

fn app() -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap())
}

const OCT: &str = "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n10/06/2026,MYSTERY VENDOR,-18.40\n10/09/2026,GROCER,-40.00\n10/10/2026,GROCER,-42.00\n10/12/2026,GROCER,-38.00\n10/15/2026,GROCER,-41.00\n10/20/2026,BIG SPLURGE,-900.00\n";

#[test]
fn reports_issues_and_a_plain_summary() {
    let mut a = app();
    a.import_statement(OCT.as_bytes(), "Checking", "checking")
        .unwrap();
    let r = a.month_close_check("2026-10").unwrap();
    assert!(r.finished && !r.closed);
    assert_eq!(r.income_cents, 500_000);
    assert!(r.summary.contains("October 2026"));
    assert!(r.summary.contains("$5,000"));
    assert!(r.issues.iter().any(|i| i.kind == "uncategorized"));
    assert!(
        r.issues
            .iter()
            .any(|i| i.kind == "unusual_amount" && i.text.contains("BIG SPLURGE")),
        "{:?}",
        r.issues
    );
}

#[test]
fn flags_an_account_that_went_quiet() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n09/05/2026,CAFE,-5.00\n",
        "Card",
        "credit",
    )
    .unwrap();
    a.import_statement(OCT.as_bytes(), "Checking", "checking")
        .unwrap();
    let r = a.month_close_check("2026-10").unwrap();
    assert!(r
        .issues
        .iter()
        .any(|i| i.kind == "quiet_account" && i.text.starts_with("Card")));
}

#[test]
fn closing_locks_the_month_and_reopening_unlocks_it() {
    let mut a = app();
    a.import_statement(OCT.as_bytes(), "Checking", "checking")
        .unwrap();
    assert!(a.close_month("2026-11").is_err(), "this month isn't over");
    let r = a.close_month("2026-10").unwrap();
    assert!(r.closed);
    assert_eq!(a.closed_months().unwrap(), vec!["2026-10".to_string()]);
    // Recategorizing and importing into the closed month are refused.
    let id = a.transactions("2026-10-01", "2026-10-31", true).unwrap()[0].id;
    assert!(a
        .set_category(id, "Shopping", false)
        .unwrap_err()
        .contains("closed"));
    let more = b"Date,Description,Amount\n10/25/2026,LATE ONE,-9.00\n";
    assert!(a
        .import_statement(more, "Checking", "checking")
        .unwrap_err()
        .contains("October 2026"));
    // Other months are unaffected.
    a.import_statement(
        b"Date,Description,Amount\n11/02/2026,CAFE,-4.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    // Reopen and the change goes through.
    assert!(!a.reopen_month("2026-10").unwrap().closed);
    a.set_category(id, "Shopping", false).unwrap();
    a.import_statement(more, "Checking", "checking").unwrap();
}

#[test]
fn rejects_a_bad_month() {
    let a = app();
    assert!(a.month_close_check("October").is_err());
    assert!(a.month_close_check("2026-13").is_err());
}
