//! End to end: bank file bytes -> parsed lines -> ledger -> week/month totals.
use fin_core::{Grain, Money, NaiveDate};
use fin_db::ledger::NewTxn;
use fin_db::Db;
use fin_ingest::{read_statement, total, ParsedTxn};

const CSV: &str = "Account: Checking 4821\nDate,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n10/03/2026,LANDLORD,-1500.00\n10/09/2026,GROCER,-45.00\n11/14/2026,ANNUAL BONUS,20000.00\n";

const OFX: &str = "OFXHEADER:100\n<OFX><STMTTRN><DTPOSTED>20261001<TRNAMT>5000.00<NAME>ACME PAYROLL</STMTTRN><STMTTRN><DTPOSTED>20261003<TRNAMT>-1500.00<NAME>LANDLORD</STMTTRN></OFX>";

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn to_rows(acct: fin_db::ledger::AccountId, p: Vec<ParsedTxn>) -> Vec<NewTxn> {
    p.into_iter()
        .map(|t| NewTxn {
            account: acct,
            day: t.day,
            amount: t.amount,
            payee: t.payee,
            memo: t.memo,
            category: None,
            stream: None,
        })
        .collect()
}

#[test]
fn csv_file_lands_in_the_ledger_and_totals_match() {
    let parsed = read_statement(CSV.as_bytes()).unwrap();
    assert_eq!(total(&parsed), Money::from_cents(2_345_500));
    let mut db = Db::open_in_memory().unwrap();
    let a = db
        .add_account("Checking", "checking", Some("Chase"), Some("4821"))
        .unwrap();
    let s = db.import_batch(&to_rows(a, parsed)).unwrap();
    assert_eq!(s.added, 4);
    assert_eq!(db.balance(a).unwrap(), Money::from_cents(2_345_500));
    let months = db
        .totals(d(2026, 10, 1), d(2026, 11, 30), Grain::Month)
        .unwrap();
    assert_eq!(months[0].net, Money::from_cents(345_500));
    assert_eq!(months[1].income, Money::from_dollars(20_000));
}

#[test]
fn same_file_twice_adds_nothing() {
    let mut db = Db::open_in_memory().unwrap();
    let a = db.add_account("Checking", "checking", None, None).unwrap();
    for expect_added in [4, 0] {
        let rows = to_rows(a, read_statement(CSV.as_bytes()).unwrap());
        assert_eq!(db.import_batch(&rows).unwrap().added, expect_added);
    }
}

#[test]
fn ofx_and_csv_of_the_same_activity_dedupe_against_each_other() {
    let mut db = Db::open_in_memory().unwrap();
    let a = db.add_account("Checking", "checking", None, None).unwrap();
    db.import_batch(&to_rows(a, read_statement(CSV.as_bytes()).unwrap()))
        .unwrap();
    // Payee text matches, so the OFX download of the same two lines is recognized as duplicates.
    let s = db
        .import_batch(&to_rows(a, read_statement(OFX.as_bytes()).unwrap()))
        .unwrap();
    assert_eq!((s.added, s.skipped), (0, 2));
    assert_eq!(db.txn_count().unwrap(), 4);
}

#[test]
fn format_detection_picks_the_right_reader() {
    assert_eq!(read_statement(OFX.as_bytes()).unwrap().len(), 2);
    assert_eq!(read_statement(CSV.as_bytes()).unwrap().len(), 4);
    assert!(read_statement(b"").is_err());
}
