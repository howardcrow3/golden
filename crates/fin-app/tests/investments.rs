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

#[test]
fn a_positions_file_becomes_holdings_without_saving() {
    let a = App::in_memory().unwrap();
    let csv = "Description,Market Value,Asset Class,Expense Ratio\nTotal Stock Index,\"$60,000.00\",Equity,0.04%\nBond Index,\"$40,000.00\",Fixed Income,0.05%\nTotal,\"$100,000.00\",,\n";
    let h = a.parse_holdings(csv).unwrap();
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].value_cents, 6_000_000);
    assert_eq!(h[0].class, Class::Stock);
    assert_eq!(h[0].expense_bp, 4);
    assert_eq!(h[1].class, Class::Bond);
    assert!(a.holdings().unwrap().is_empty());
}

#[test]
fn an_unreadable_positions_file_says_why() {
    let a = App::in_memory().unwrap();
    assert!(a.parse_holdings("").is_err());
    assert!(a.parse_holdings("Date,Amount\n2026-01-01,5\n").is_err());
    let err = a.parse_holdings("Name,Value\nA,0\n").unwrap_err();
    assert!(err.contains("No holdings"), "{err}");
}
