use fin_core::Money;
use fin_ingest::holdings::{classify, parse, AssetKind};

#[test]
fn reads_a_typical_positions_file_and_skips_totals() {
    let csv = "Account Name,Brokerage\nAs of,2026-11-20\n\nSymbol,Description,Quantity,Market Value,Asset Class,Expense Ratio\n\
VTI,Vanguard Total Stock Market ETF,10,\"$12,345.67\",Equity,0.03%\n\
BND,Vanguard Total Bond Market ETF,5,\"$4,000.00\",Fixed Income,0.03%\n\
SPAXX,Fidelity Government Money Market,1,$500.00,Cash,\n\
Total,,,\"$16,845.67\",,\n";
    let h = parse(csv).unwrap();
    // "Description" is ranked above "Symbol" so the fund names show, not the tickers.
    assert_eq!(h.len(), 3);
    assert_eq!(h[0].name, "Vanguard Total Stock Market ETF");
    assert_eq!(h[0].value, Money::parse("12345.67").unwrap());
    assert_eq!(h[0].kind, AssetKind::Stock);
    assert_eq!(h[0].expense_bp, 3);
    assert!(!h[0].single_stock);
    assert_eq!(h[1].kind, AssetKind::Bond);
    assert_eq!(h[2].kind, AssetKind::Cash);
    assert_eq!(h[2].expense_bp, 0);
}

#[test]
fn an_individual_stock_is_flagged() {
    let csv =
        "Name,Value,Type\nAcme Corp,\"$1,000\",Common Stock\nIndex Fund,\"$1,000\",Stock Fund\n";
    let h = parse(csv).unwrap();
    assert!(h[0].single_stock);
    assert!(!h[1].single_stock);
}

#[test]
fn skips_empty_negative_and_zero_positions() {
    let csv = "Name,Value\nA,100\nB,\nC,0\nD,-50\n,25\n";
    let h = parse(csv).unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].name, "A");
}

#[test]
fn a_bad_value_names_the_row() {
    let err = parse("Name,Value\nA,100\nB,lots\n").unwrap_err();
    assert!(err.to_string().contains("row 3"), "{err}");
}

#[test]
fn files_without_the_columns_are_refused() {
    assert!(parse("").is_err());
    assert!(parse("Date,Amount\n2026-01-01,5\n").is_err());
}

#[test]
fn classify_falls_back_to_the_name() {
    assert_eq!(classify("", "Total Bond Market Index"), AssetKind::Bond);
    assert_eq!(classify("", "Prime Money Market"), AssetKind::Cash);
    assert_eq!(classify("", "Mystery Thing"), AssetKind::Other);
    assert_eq!(classify("Treasury", "Anything"), AssetKind::Bond);
}
