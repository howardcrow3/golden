use fin_app::balance::is_debt_kind;
use fin_app::App;

fn app() -> App {
    let mut app = App::in_memory().unwrap();
    app.import_statement(
        "Date,Description,Amount\n01/01/2026,PAYROLL,3000.00\n01/05/2026,RENT,-1000.00\n"
            .as_bytes(),
        "Checking",
        "checking",
    )
    .unwrap();
    app.import_statement(
        "Date,Description,Amount\n01/06/2026,POS DEBIT STORE,-400.00\n01/20/2026,PAYMENT,100.00\n"
            .as_bytes(),
        "Visa",
        "credit",
    )
    .unwrap();
    app
}

#[test]
fn kinds_are_classified() {
    assert!(is_debt_kind("Credit Card"));
    assert!(is_debt_kind("mortgage"));
    assert!(is_debt_kind("Auto Loan"));
    assert!(!is_debt_kind("checking"));
    assert!(!is_debt_kind("brokerage"));
}

#[test]
fn net_worth_is_assets_minus_debts() {
    let s = app().balance_sheet().unwrap();
    assert_eq!(s.assets.len(), 1);
    assert_eq!(s.assets[0].cents, 200_000);
    assert_eq!(s.debts.len(), 1);
    assert_eq!(s.debts[0].name, "Visa");
    assert_eq!(s.debts[0].cents, 30_000);
    assert_eq!(s.assets_cents, 200_000);
    assert_eq!(s.debts_cents, 30_000);
    assert_eq!(s.net_worth_cents, 170_000);
}

#[test]
fn overpaid_card_counts_as_an_asset() {
    let mut app = App::in_memory().unwrap();
    app.import_statement(
        "Date,Description,Amount\n01/06/2026,PAYMENT,50.00\n".as_bytes(),
        "Visa",
        "credit",
    )
    .unwrap();
    let s = app.balance_sheet().unwrap();
    assert!(s.debts.is_empty());
    assert_eq!(s.net_worth_cents, 5_000);
}

#[test]
fn overdrawn_checking_is_a_debt() {
    let mut app = App::in_memory().unwrap();
    app.import_statement(
        "Date,Description,Amount\n01/06/2026,RENT,-50.00\n".as_bytes(),
        "Checking",
        "checking",
    )
    .unwrap();
    let s = app.balance_sheet().unwrap();
    assert_eq!(s.debts_cents, 5_000);
    assert_eq!(s.net_worth_cents, -5_000);
}

#[test]
fn empty_is_zero() {
    let s = App::in_memory().unwrap().balance_sheet().unwrap();
    assert_eq!(s.net_worth_cents, 0);
    assert!(s.assets.is_empty() && s.debts.is_empty());
}
