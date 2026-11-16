use fin_app::App;
use fin_core::NaiveDate;

const STUB: &str = "Employer: Acme Corp
Pay Period Beginning: 10/01/2026
Pay Period Ending: 10/15/2026
Pay Date: 10/20/2026
Gross Pay 4,000.00 80,000.00
Federal Income Tax 420.00
State Income Tax 180.00
Social Security 248.00
Medicare 58.00
401(k) 320.00
Health Insurance 110.00
Net Pay 2,664.00
";

fn biweekly() -> String {
    STUB.replace("10/01/2026", "10/05/2026")
        .replace("10/15/2026", "10/18/2026")
}

#[test]
fn summary_annualizes_by_frequency() {
    let app = App::in_memory().unwrap();
    let s = app.read_paystub(STUB).unwrap();
    assert_eq!(s.frequency, "semimonthly");
    assert_eq!(s.periods_per_year, 24);
    assert_eq!(s.annual_gross_cents, 400_000 * 24);
    assert_eq!(s.annual_retirement_cents, 32_000 * 24);
    // (420 + 180) / 4000 = 15.00%
    assert_eq!(s.income_tax_rate_bp, 1500);
    assert_eq!(s.pay_date.as_deref(), Some("2026-10-20"));
}

#[test]
fn frequency_is_found_from_the_period_length() {
    let app = App::in_memory().unwrap();
    let f = |a: &str, b: &str| {
        app.read_paystub(&STUB.replace("10/01/2026", a).replace("10/15/2026", b))
            .unwrap()
            .frequency
    };
    assert_eq!(f("10/05/2026", "10/11/2026"), "weekly");
    assert_eq!(f("10/05/2026", "10/18/2026"), "biweekly");
    assert_eq!(f("10/16/2026", "10/31/2026"), "semimonthly");
    assert_eq!(f("10/01/2026", "10/31/2026"), "monthly");
    assert_eq!(f("10/01/2026", "12/31/2026"), "");
}

#[test]
fn reading_changes_nothing() {
    let app = App::in_memory().unwrap();
    let before = app.household().unwrap();
    app.read_paystub(STUB).unwrap();
    assert_eq!(app.household().unwrap(), before);
}

#[test]
fn applying_adds_then_updates_the_same_income_source() {
    let app = App::in_memory().unwrap();
    let n = app.household().unwrap().streams.len();
    let row = app.apply_paystub(&biweekly(), 0).unwrap();
    assert_eq!(row.label, "Acme Corp salary");
    assert_eq!(row.annual_cents, 400_000 * 26);
    assert_eq!(app.household().unwrap().streams.len(), n + 1);
    // A raise: same employer, bigger stub; the source is replaced, not duplicated.
    let raise = biweekly()
        .replace("4,000.00", "5,000.00")
        .replace("2,664.00", "3,500.00");
    let row = app.apply_paystub(&raise, 0).unwrap();
    assert_eq!(row.annual_cents, 500_000 * 26);
    assert_eq!(app.household().unwrap().streams.len(), n + 1);
}

#[test]
fn unknown_pay_period_or_person_is_refused() {
    let app = App::in_memory().unwrap();
    let no_period = "Gross Pay 1,000.00\nNet Pay 800.00\n";
    assert!(app
        .apply_paystub(no_period, 0)
        .unwrap_err()
        .contains("pay period"));
    assert!(app.apply_paystub(STUB, 9).unwrap_err().contains("people"));
}

#[test]
fn garbage_is_a_readable_error() {
    let app = App::in_memory().unwrap();
    assert!(app.read_paystub("hello").unwrap_err().contains("gross pay"));
}

#[test]
fn applying_is_locked_after_the_trial_but_reading_works() {
    let app = App::in_memory().unwrap();
    app.db().set_setting("first_run", "2026-01-01").unwrap();
    let app = app.with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap());
    assert!(app.read_paystub(STUB).is_ok());
    assert!(app.apply_paystub(STUB, 0).is_err());
}
