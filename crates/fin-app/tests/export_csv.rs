use fin_app::App;
use fin_core::NaiveDate;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

const CSV: &str = "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n10/06/2026,\"SMITH, JONES & CO\",-10.50\n10/09/2026,=HYPERLINK(\"evil\"),-4.00\n10/10/2026,NETFLIX.COM,-15.49\n";

fn loaded() -> App {
    let mut app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 16));
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    app
}

#[test]
fn header_rows_oldest_first_and_plain_decimal_amounts() {
    let csv = loaded()
        .transactions_csv("2026-10-01", "2026-10-31")
        .unwrap();
    let lines: Vec<&str> = csv.split("\r\n").collect();
    assert_eq!(lines[0], "Date,Account,Payee,Memo,Amount,Category");
    assert!(lines[1].starts_with("2026-10-01,Checking,ACME PAYROLL,,5000.00,Salary"));
    assert!(lines[4].starts_with("2026-10-10,Checking,NETFLIX.COM,,-15.49,Subscriptions"));
    assert_eq!(lines.len(), 6); // header + 4 rows + trailing empty
}

#[test]
fn commas_and_quotes_are_escaped() {
    let csv = loaded()
        .transactions_csv("2026-10-01", "2026-10-31")
        .unwrap();
    assert!(csv.contains("\"SMITH, JONES & CO\""));
}

#[test]
fn formula_like_payees_are_defused() {
    let csv = loaded()
        .transactions_csv("2026-10-01", "2026-10-31")
        .unwrap();
    assert!(csv.contains("'=HYPERLINK"), "{csv}");
    assert!(!csv.contains(",=HYPERLINK"));
}

#[test]
fn only_the_requested_dates_are_included() {
    let csv = loaded()
        .transactions_csv("2026-10-06", "2026-10-09")
        .unwrap();
    assert_eq!(csv.matches("\r\n").count(), 3);
    assert!(!csv.contains("ACME"));
}

#[test]
fn export_works_after_the_trial_has_ended_and_files_are_not_overwritten() {
    let app = loaded();
    app.db().set_setting("first_run", "2026-01-01").unwrap();
    assert!(!app.license_status(d(2026, 11, 16)).unwrap().can_edit);
    let dir = tempfile::tempdir().unwrap();
    let a = app
        .export_transactions(dir.path(), "2026-10-01", "2026-10-31")
        .unwrap();
    let b = app
        .export_transactions(dir.path(), "2026-10-01", "2026-10-31")
        .unwrap();
    assert_ne!(a, b);
    assert!(b.to_string_lossy().ends_with("(2).csv"));
    let text = std::fs::read_to_string(a).unwrap();
    assert!(text.starts_with("Date,Account"));
}

#[test]
fn bad_dates_are_an_error() {
    assert!(loaded()
        .transactions_csv("10/01/2026", "2026-10-31")
        .is_err());
}

#[test]
fn the_excel_export_has_number_amounts_and_safe_text() {
    let app = loaded();
    let book = app.transactions_xlsx("2026-10-01", "2026-10-31").unwrap();
    assert_eq!(&book[..2], b"PK");
    // Parts are stored uncompressed, so the sheet's XML can be read straight from the bytes.
    let text = String::from_utf8_lossy(&book);
    assert!(text.contains("<sheet name=\"Transactions\""));
    assert!(text.contains("<v>5000</v>"), "amounts are numbers");
    assert!(text.contains("<v>-15.49</v>"));
    assert!(
        text.contains(">=HYPERLINK(&quot;evil&quot;)</t>"),
        "kept as text"
    );
    assert!(!text.contains("<f>"));
    assert!(text.contains(">SMITH, JONES &amp; CO</t>"));
    // Written to a new file each time, even after the trial.
    app.db().set_setting("first_run", "2026-01-01").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let a = app
        .export_transactions_xlsx(dir.path(), "2026-10-01", "2026-10-31")
        .unwrap();
    let b = app
        .export_transactions_xlsx(dir.path(), "2026-10-01", "2026-10-31")
        .unwrap();
    assert_ne!(a, b);
    assert!(a.to_string_lossy().ends_with(".xlsx"));
    assert!(b.to_string_lossy().ends_with("(2).xlsx"));
}
