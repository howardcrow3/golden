use fin_app::App;
use fin_core::NaiveDate;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}

fn app() -> App {
    let mut app = App::in_memory().unwrap().with_clock(today);
    app.import_statement(
        "Date,Description,Amount\n09/01/2026,ACME PAYROLL,5000.00\n09/03/2026,RENT PAYMENT LANDLORD,-1500.00\n10/01/2026,ACME PAYROLL,5000.00\n10/03/2026,RENT PAYMENT LANDLORD,-1500.00\n11/01/2026,ACME PAYROLL,5000.00\n"
            .as_bytes(),
        "Checking",
        "checking",
    )
    .unwrap();
    app
}

fn values(a: &App, n: u32) -> Vec<(String, i64)> {
    a.net_worth_history(n)
        .unwrap()
        .into_iter()
        .map(|p| (p.month, p.net_worth_cents))
        .collect()
}

#[test]
fn running_total_at_each_month_end_oldest_first() {
    let v = values(&app(), 4);
    assert_eq!(
        v,
        vec![
            ("2026-08".into(), 0),
            ("2026-09".into(), 350_000),
            ("2026-10".into(), 700_000),
            ("2026-11".into(), 1_200_000),
        ]
    );
}

#[test]
fn the_last_point_equals_todays_net_worth() {
    let a = app();
    let last = a.net_worth_history(6).unwrap().pop().unwrap();
    assert_eq!(
        last.net_worth_cents,
        a.balance_sheet().unwrap().net_worth_cents
    );
}

#[test]
fn starting_balances_lift_every_month() {
    let a = app();
    let id = a.accounts().unwrap()[0].id;
    a.set_opening_balance(id, 1_000_000).unwrap();
    let v = values(&a, 4);
    assert_eq!(v[0].1, 1_000_000);
    assert_eq!(v[3].1, 2_200_000);
}

#[test]
fn debts_pull_it_down() {
    let mut a = app();
    a.import_statement(
        "Date,Description,Amount\n10/15/2026,POS DEBIT STORE,-2000.00\n".as_bytes(),
        "Visa",
        "credit",
    )
    .unwrap();
    let v = values(&a, 2);
    assert_eq!(v[0].1, 700_000 - 200_000);
    assert_eq!(v[1].1, 1_200_000 - 200_000);
}

#[test]
fn crosses_year_boundaries_and_checks_bounds() {
    let a = App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2027, 2, 10).unwrap());
    let v = values(&a, 4);
    assert_eq!(
        v.iter().map(|p| p.0.as_str()).collect::<Vec<_>>(),
        ["2026-11", "2026-12", "2027-01", "2027-02"]
    );
    assert!(a.net_worth_history(0).is_err());
    assert!(a.net_worth_history(121).is_err());
    assert_eq!(a.net_worth_history(1).unwrap().len(), 1);
}

#[test]
fn an_empty_ledger_is_all_zeros() {
    assert!(values(&App::in_memory().unwrap().with_clock(today), 3)
        .iter()
        .all(|p| p.1 == 0));
}
