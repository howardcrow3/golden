use fin_app::App;
use fin_core::NaiveDate;

fn app() -> App {
    let mut app = App::in_memory().unwrap();
    app.import_statement(
        "Date,Description,Amount\n11/01/2026,ACME PAYROLL,5000.00\n11/03/2026,RENT PAYMENT LANDLORD,-1500.00\n"
            .as_bytes(),
        "Checking",
        "checking",
    )
    .unwrap();
    app
}

#[test]
fn opening_balance_is_part_of_the_balance_and_net_worth() {
    let app = app();
    let id = app.accounts().unwrap()[0].id;
    assert_eq!(app.accounts().unwrap()[0].opening_cents, 0);
    app.set_opening_balance(id, 1_000_000).unwrap();
    let a = &app.accounts().unwrap()[0];
    assert_eq!(a.balance_cents, 1_350_000);
    assert_eq!(a.opening_cents, 1_000_000);
    assert_eq!(app.balance_sheet().unwrap().net_worth_cents, 1_350_000);
}

#[test]
fn opening_balance_is_not_income() {
    let app = app();
    let id = app.accounts().unwrap()[0].id;
    let before = app
        .cash_flow(
            NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11, 30).unwrap(),
            "month",
        )
        .unwrap();
    app.set_opening_balance(id, 9_999_900).unwrap();
    let after = app
        .cash_flow(
            NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11, 30).unwrap(),
            "month",
        )
        .unwrap();
    assert_eq!(before, after);
}

#[test]
fn changing_it_replaces_the_old_value_and_negative_is_allowed() {
    let app = app();
    let id = app.accounts().unwrap()[0].id;
    app.set_opening_balance(id, 500_000).unwrap();
    app.set_opening_balance(id, -200_000).unwrap();
    assert_eq!(app.accounts().unwrap()[0].balance_cents, 150_000);
    assert_eq!(app.balance_sheet().unwrap().net_worth_cents, 150_000);
}

#[test]
fn unknown_account_and_absurd_amounts_are_refused() {
    let app = app();
    assert!(app
        .set_opening_balance(999, 100)
        .unwrap_err()
        .contains("No such account"));
    let id = app.accounts().unwrap()[0].id;
    assert!(app
        .set_opening_balance(id, i64::MAX)
        .unwrap_err()
        .contains("too large"));
}

#[test]
fn locked_after_the_trial() {
    let app = app();
    let id = app.accounts().unwrap()[0].id;
    app.db().set_setting("first_run", "2026-01-01").unwrap();
    let app = app.with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap());
    assert!(app.set_opening_balance(id, 100).is_err());
}

#[test]
fn backup_carries_it_to_a_fresh_copy_without_overwriting_existing() {
    let src = app();
    let id = src.accounts().unwrap()[0].id;
    src.set_opening_balance(id, 700_000).unwrap();
    let json = src.backup_json().unwrap();

    let mut fresh = App::in_memory().unwrap();
    fresh.restore_backup(&json).unwrap();
    assert_eq!(fresh.accounts().unwrap()[0].opening_cents, 700_000);

    // A copy that already has the account keeps what it has.
    let mut other = app();
    let oid = other.accounts().unwrap()[0].id;
    other.set_opening_balance(oid, 1).unwrap();
    other.restore_backup(&json).unwrap();
    assert_eq!(other.accounts().unwrap()[0].opening_cents, 1);
}

#[test]
fn the_forecast_starts_from_the_balance_including_it() {
    let mut app = App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap());
    app.import_statement(
        "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n10/03/2026,RENT PAYMENT LANDLORD,-1500.00\n"
            .as_bytes(),
        "Checking",
        "checking",
    )
    .unwrap();
    let id = app.accounts().unwrap()[0].id;
    app.set_opening_balance(id, 2_000_000).unwrap();
    assert_eq!(app.forecast(1).unwrap().start_balance_cents, 2_350_000);
}
