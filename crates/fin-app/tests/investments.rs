use fin_app::App;
use fin_plan::investments::{Class, Holding, Risk};

fn holding(name: &str, dollars: i64, class: Class, bp: i64) -> Holding {
    Holding {
        name: name.into(),
        value_cents: dollars * 100,
        class,
        expense_bp: bp,
        single_stock: false,
    }
}

#[test]
fn holdings_are_saved_and_read_back() {
    let a = App::in_memory().unwrap();
    assert!(a.holdings().unwrap().is_empty());
    let v = vec![
        holding("Total market", 60_000, Class::Stock, 3),
        holding("Bonds", 40_000, Class::Bond, 5),
    ];
    a.save_holdings(&v).unwrap();
    assert_eq!(a.holdings().unwrap(), v);
}

#[test]
fn the_checkup_uses_the_saved_holdings_and_household_age() {
    let a = App::in_memory().unwrap();
    assert!(a
        .investment_checkup(Risk::Moderate)
        .unwrap_err()
        .contains("at least one"));
    a.save_holdings(&[holding("Total market", 100_000, Class::Stock, 3)])
        .unwrap();
    let c = a.investment_checkup(Risk::Conservative).unwrap();
    assert_eq!(c.total_cents, 10_000_000);
    assert!(c.stock_gap > 0.3);
}

#[test]
fn unnamed_holdings_are_refused() {
    let a = App::in_memory().unwrap();
    assert!(a
        .save_holdings(&[holding(" ", 1, Class::Cash, 0)])
        .unwrap_err()
        .contains("name"));
}
