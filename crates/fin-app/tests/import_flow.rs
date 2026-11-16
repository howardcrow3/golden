use fin_app::App;
use fin_core::NaiveDate;

const CSV: &str = "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n10/02/2026,POS DEBIT TRADER JOE'S #12,-82.10\n10/05/2026,NETFLIX.COM,-15.49\n10/06/2026,MYSTERY VENDOR,-10.00\n11/02/2026,ACME PAYROLL,5000.00\n";
const OFX: &str = "OFXHEADER:100\nDATA:OFXSGML\n\n<OFX><BANKMSGSRSV1><STMTTRNRS><STMTRS><BANKTRANLIST>\n<STMTTRN><TRNTYPE>DEBIT<DTPOSTED>20261015<TRNAMT>-40.00<FITID>1<NAME>SHELL OIL</STMTTRN>\n</BANKTRANLIST></STMTRS></STMTTRNRS></BANKMSGSRSV1></OFX>";

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[test]
fn import_report_counts_and_dedupes() {
    let mut app = App::in_memory().unwrap();
    let r = app
        .import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    assert_eq!((r.parsed, r.added, r.skipped_duplicates), (5, 5, 0));
    assert_eq!(r.uncategorized, 1);
    assert_eq!(r.total_cents, 1_000_000 - 8_210 - 1_549 - 1_000);
    let again = app
        .import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    assert_eq!((again.added, again.skipped_duplicates), (0, 5));
    assert_eq!(app.accounts().unwrap().len(), 1);
}

#[test]
fn cash_flow_switches_grain_and_fills_gaps() {
    let mut app = App::in_memory().unwrap();
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    let months = app
        .cash_flow(d(2026, 10, 1), d(2026, 11, 30), "month")
        .unwrap();
    assert_eq!(months.len(), 2);
    assert_eq!(months[0].period, "2026-10");
    assert_eq!(months[0].income_cents, 500_000);
    assert_eq!(months[0].spending_cents, 8_210 + 1_549 + 1_000);
    let year = app
        .cash_flow(d(2026, 10, 1), d(2026, 11, 30), "year")
        .unwrap();
    assert_eq!(year.len(), 1);
    assert_eq!(year[0].income_cents, 1_000_000);
    assert!(app
        .cash_flow(d(2026, 1, 1), d(2026, 2, 1), "fortnight")
        .is_err());
}

#[test]
fn multiple_files_and_formats_land_in_separate_accounts() {
    let mut app = App::in_memory().unwrap();
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    app.import_statement(OFX.as_bytes(), "Visa", "credit")
        .unwrap();
    let accts = app.accounts().unwrap();
    assert_eq!(accts.len(), 2);
    assert_eq!(accts[1].balance_cents, -4_000);
    let cats = app.categories(d(2026, 10, 10), "month", false).unwrap();
    assert!(cats
        .iter()
        .any(|c| c.category == "Groceries" && c.cents == 8_210));
    assert!(cats.iter().any(|c| c.category == "Uncategorized"));
}

#[test]
fn bad_input_gives_readable_errors() {
    let mut app = App::in_memory().unwrap();
    assert!(app.import_statement(b"", "X", "checking").is_err());
    assert!(app
        .import_statement(CSV.as_bytes(), "  ", "checking")
        .is_err());
    assert!(app
        .import_statement(b"hello,world\n1,2\n", "X", "checking")
        .is_err());
    assert!(app.accounts().unwrap().is_empty() || app.accounts().unwrap().len() <= 1);
}

#[test]
fn learned_category_applies_to_next_import() {
    let mut app = App::in_memory().unwrap();
    app.learn_category("MYSTERY VENDOR", "Hobbies");
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    let cats = app.categories(d(2026, 10, 10), "month", false).unwrap();
    assert!(cats
        .iter()
        .any(|c| c.category == "Hobbies" && c.cents == 1_000));
}

#[test]
fn string_dates_from_the_ui() {
    let mut app = App::in_memory().unwrap();
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    let rows = app
        .cash_flow_str("2026-10-01", "2026-11-30", "month")
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(app
        .cash_flow_str("10/01/2026", "2026-11-30", "month")
        .is_err());
    assert!(!app
        .categories_str("2026-10-10", "month", true)
        .unwrap()
        .is_empty());
}
