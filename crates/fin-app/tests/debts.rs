use fin_app::App;
use fin_core::NaiveDate;

fn app() -> App {
    let mut app = App::in_memory().unwrap();
    app.import_statement(
        "Date,Description,Amount\n11/01/2026,ACME PAYROLL,5000.00\n".as_bytes(),
        "Checking",
        "checking",
    )
    .unwrap();
    app.import_statement(
        "Date,Description,Amount\n11/02/2026,POS DEBIT STORE,-3000.00\n".as_bytes(),
        "Visa",
        "credit",
    )
    .unwrap();
    app.import_statement(
        "Date,Description,Amount\n11/03/2026,LOAN BALANCE,-9000.00\n".as_bytes(),
        "Car loan",
        "loan",
    )
    .unwrap();
    app
}

#[test]
fn debts_without_terms_are_listed_but_not_planned() {
    let p = app().debt_plan(0).unwrap();
    assert_eq!(p.debts.len(), 2);
    assert!(p.debts.iter().all(|d| !d.has_terms));
    assert!(p.avalanche.is_none() && p.snowball.is_none() && p.minimums_only.is_none());
    // The checking account is not a debt.
    assert!(p.debts.iter().all(|d| d.name != "Checking"));
}

#[test]
fn terms_make_a_plan_and_extra_money_saves_interest() {
    let a = app();
    a.set_debt_terms("Visa", 2299, 9_000).unwrap();
    a.set_debt_terms("Car loan", 699, 25_000).unwrap();
    let p = a.debt_plan(30_000).unwrap();
    assert!(p.debts.iter().all(|d| d.has_terms));
    let av = p.avalanche.as_ref().unwrap();
    let base = p.minimums_only.as_ref().unwrap();
    assert!(av.feasible && base.feasible);
    assert!(av.months < base.months);
    assert!(av.total_interest_cents < base.total_interest_cents);
    assert!(av.total_interest_cents <= p.snowball.as_ref().unwrap().total_interest_cents);
}

#[test]
fn only_debts_with_terms_are_used() {
    let a = app();
    a.set_debt_terms("Visa", 2000, 10_000).unwrap();
    let p = a.debt_plan(0).unwrap();
    let order = &p.avalanche.unwrap().order;
    assert_eq!(order.len(), 1);
    assert_eq!(order[0].name, "Visa");
    assert!(p.debts.iter().any(|d| d.name == "Car loan" && !d.has_terms));
}

#[test]
fn terms_are_remembered_and_can_be_changed() {
    let a = app();
    a.set_debt_terms("Visa", 2000, 10_000).unwrap();
    a.set_debt_terms("Visa", 1500, 12_000).unwrap();
    let row = a
        .debt_plan(0)
        .unwrap()
        .debts
        .into_iter()
        .find(|d| d.name == "Visa")
        .unwrap();
    assert_eq!((row.apr_bp, row.min_cents), (1500, 12_000));
}

#[test]
fn bad_input_is_refused_with_a_reason() {
    let a = app();
    assert!(a
        .set_debt_terms("Visa", -1, 100)
        .unwrap_err()
        .contains("rate"));
    assert!(a
        .set_debt_terms("Visa", 10_001, 100)
        .unwrap_err()
        .contains("rate"));
    assert!(a
        .set_debt_terms("Visa", 1000, -5)
        .unwrap_err()
        .contains("minimum"));
    assert!(a
        .set_debt_terms("Checking", 1000, 100)
        .unwrap_err()
        .contains("isn't one of the debts"));
    assert!(a.set_debt_terms("Nope", 1000, 100).is_err());
    assert!(a.debt_plan(-1).is_err());
}

#[test]
fn a_minimum_that_cannot_cover_interest_says_so() {
    let a = app();
    a.set_debt_terms("Visa", 9000, 100).unwrap();
    let p = a.debt_plan(0).unwrap();
    assert!(!p.minimums_only.unwrap().feasible);
}

#[test]
fn editing_is_locked_after_the_trial_but_reading_works() {
    let a = app();
    a.set_debt_terms("Visa", 2000, 10_000).unwrap();
    a.db().set_setting("first_run", "2026-01-01").unwrap();
    let a = a.with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap());
    assert!(a.set_debt_terms("Visa", 100, 100).is_err());
    assert!(a.debt_plan(0).unwrap().avalanche.is_some());
}
