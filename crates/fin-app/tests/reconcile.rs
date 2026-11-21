use fin_app::App;

fn ofx(balance: &str) -> String {
    format!(
        "OFXHEADER:100\n<OFX><STMTRS><BANKTRANLIST>\
<STMTTRN><DTPOSTED>20261001<TRNAMT>5000.00<NAME>PAYROLL</STMTTRN>\
<STMTTRN><DTPOSTED>20261003<TRNAMT>-1500.00<NAME>LANDLORD</STMTTRN>\
</BANKTRANLIST><LEDGERBAL><BALAMT>{balance}<DTASOF>20261031</LEDGERBAL></STMTRS></OFX>"
    )
}

#[test]
fn matching_balance_reconciles() {
    let mut app = App::in_memory().unwrap();
    let r = app
        .import_statement(ofx("3500.00").as_bytes(), "Chase", "checking")
        .unwrap();
    assert_eq!(r.statement_balance_cents, Some(350_000));
    assert_eq!(r.balance_after_cents, Some(350_000));
    assert_eq!(r.reconciled, Some(true));
    assert_eq!(r.difference_cents, Some(0));
}

#[test]
fn mismatch_reports_the_gap_and_opening_balance_fixes_it() {
    let mut app = App::in_memory().unwrap();
    let r = app
        .import_statement(ofx("4000.00").as_bytes(), "Chase", "checking")
        .unwrap();
    assert_eq!(r.reconciled, Some(false));
    assert_eq!(r.difference_cents, Some(50_000));
    let id = app.accounts().unwrap()[0].id;
    app.set_opening_balance(id, 50_000).unwrap();
    let again = app
        .import_statement(ofx("4000.00").as_bytes(), "Chase", "checking")
        .unwrap();
    assert_eq!(again.added, 0);
    assert_eq!(again.reconciled, Some(true));
}

#[test]
fn csv_has_no_printed_balance() {
    let mut app = App::in_memory().unwrap();
    let r = app
        .import_statement(
            b"Date,Amount,Description\n2026-10-01,10.00,X\n",
            "A",
            "checking",
        )
        .unwrap();
    assert_eq!(r.reconciled, None);
    assert_eq!(r.statement_balance_cents, None);
}
