use fin_app::App;
use fin_core::NaiveDate;

fn app() -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap())
}

#[test]
fn holdings_count_toward_net_worth_and_can_be_updated() {
    let app = app();
    app.add_holding("2019 Honda Civic", "vehicle", 1_200_000)
        .unwrap();
    app.add_holding("Wallet", "Crypto", 350_000).unwrap();
    let sheet = app.balance_sheet().unwrap();
    assert_eq!(sheet.assets_cents, 1_550_000);
    assert_eq!(sheet.net_worth_cents, 1_550_000);
    assert!(sheet
        .assets
        .iter()
        .any(|l| l.name == "2019 Honda Civic" && l.kind == "Vehicle"));
    let id = app
        .accounts()
        .unwrap()
        .into_iter()
        .find(|a| a.name == "Wallet")
        .unwrap()
        .id;
    app.set_opening_balance(id, 500_000).unwrap();
    assert_eq!(app.balance_sheet().unwrap().assets_cents, 1_700_000);
}

#[test]
fn bad_holdings_are_readable_errors() {
    let app = app();
    assert!(app.add_holding("  ", "Vehicle", 1).is_err());
    assert!(app
        .add_holding("Boat", "Spaceship", 1)
        .unwrap_err()
        .contains("Choose one of"));
    assert!(app.add_holding("Boat", "Other asset", -5).is_err());
    app.add_holding("Boat", "Other asset", 500_000).unwrap();
    assert!(app
        .add_holding("boat", "Vehicle", 1)
        .unwrap_err()
        .contains("already"));
}
