//! The same transaction arriving from two files that describe it differently is stored once.
use chrono::NaiveDate;
use fin_core::Money;
use fin_db::ledger::{AccountId, NewTxn};
use fin_db::Db;

fn d(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

fn row(a: AccountId, day: &str, cents: i64, payee: &str) -> NewTxn {
    NewTxn {
        account: a,
        day: d(day),
        amount: Money::from_cents(cents),
        payee: payee.into(),
        memo: String::new(),
        category: None,
        stream: None,
    }
}

fn setup() -> (Db, AccountId) {
    let db = Db::open_in_memory().unwrap();
    let a = db.add_account("Checking", "checking", None, None).unwrap();
    (db, a)
}

fn count(db: &Db) -> i64 {
    db.txn_count().unwrap()
}

#[test]
fn a_csv_then_an_ofx_of_the_same_month_adds_nothing() {
    let (mut db, a) = setup();
    db.import_batch(&[
        row(a, "2026-10-03", -4_500, "AMAZON MKTPLACE PMTS"),
        row(a, "2026-10-05", -1_200, "Trader Joe's #123"),
        row(a, "2026-10-09", 250_000, "ACME PAYROLL"),
    ])
    .unwrap();
    // Same month, different descriptions, one posting a day later.
    let s = db
        .import_batch(&[
            row(a, "2026-10-03", -4_500, "Amazon.com*2K4TT1"),
            row(a, "2026-10-06", -1_200, "TRADER JOES"),
            row(a, "2026-10-09", 250_000, "Acme Payroll Direct Dep"),
        ])
        .unwrap();
    assert_eq!((s.added, s.skipped), (0, 3));
    assert_eq!(count(&db), 3);
}

#[test]
fn two_real_purchases_both_survive() {
    let (mut db, a) = setup();
    db.import_batch(&[row(a, "2026-10-03", -400, "Starbucks 55")])
        .unwrap();
    // Two coffees on the day: one is the old one described differently, one is new.
    let s = db
        .import_batch(&[
            row(a, "2026-10-03", -400, "STARBUCKS STORE"),
            row(a, "2026-10-03", -400, "STARBUCKS STORE"),
        ])
        .unwrap();
    assert_eq!((s.added, s.skipped), (1, 1));
    assert_eq!(count(&db), 2);
}

#[test]
fn a_purchase_just_past_the_covered_period_is_new() {
    let (mut db, a) = setup();
    db.import_batch(&[
        row(a, "2026-10-01", -100, "Other"),
        row(a, "2026-10-31", -400, "Starbucks"),
    ])
    .unwrap();
    let s = db
        .import_batch(&[row(a, "2026-11-01", -400, "Starbucks 55")])
        .unwrap();
    assert_eq!((s.added, s.skipped), (1, 0));
}

#[test]
fn different_amounts_accounts_or_merchants_are_kept() {
    let (mut db, a) = setup();
    let b = db.add_account("Savings", "savings", None, None).unwrap();
    db.import_batch(&[row(a, "2026-10-03", -4_500, "Amazon")])
        .unwrap();
    let s = db
        .import_batch(&[
            row(a, "2026-10-03", -4_501, "Amazon"),
            row(b, "2026-10-03", -4_500, "Amazon"),
            row(a, "2026-10-03", -4_500, "Shell Oil"),
        ])
        .unwrap();
    assert_eq!((s.added, s.skipped), (3, 0));
}

#[test]
fn a_newer_purchase_listed_first_is_not_taken_for_an_older_one() {
    let (mut db, a) = setup();
    // The first download, taken on the 15th, has Tuesday's coffee.
    db.import_batch(&[
        row(a, "2026-10-01", -100, "Other"),
        row(a, "2026-10-14", -450, "Starbucks"),
        row(a, "2026-10-15", -900, "Lunch"),
    ])
    .unwrap();
    // A later download lists newest first and adds Wednesday's coffee.
    let s = db
        .import_batch(&[
            row(a, "2026-10-15", -450, "Starbucks"),
            row(a, "2026-10-15", -900, "Lunch"),
            row(a, "2026-10-14", -450, "Starbucks"),
            row(a, "2026-10-01", -100, "Other"),
        ])
        .unwrap();
    assert_eq!((s.added, s.skipped), (1, 3));
    assert_eq!(count(&db), 4);
}

#[test]
fn a_same_day_copy_is_matched_before_a_neighbouring_day() {
    let (mut db, a) = setup();
    db.import_batch(&[
        row(a, "2026-10-01", -100, "Other"),
        row(a, "2026-10-14", -450, "STARBUCKS 55"),
        row(a, "2026-10-20", -100, "Other"),
    ])
    .unwrap();
    // Another format of the same account, with a second coffee the day after.
    let s = db
        .import_batch(&[
            row(a, "2026-10-15", -450, "Starbucks Store"),
            row(a, "2026-10-14", -450, "Starbucks Store"),
        ])
        .unwrap();
    assert_eq!((s.added, s.skipped), (1, 1));
    let days: Vec<String> = db
        .txns_between(d("2026-10-14"), d("2026-10-15"))
        .unwrap()
        .iter()
        .map(|t| t.day.to_string())
        .collect();
    assert_eq!(days.len(), 2);
    assert!(days.contains(&"2026-10-15".to_string()), "{days:?}");
}

#[test]
fn a_month_filled_in_later_keeps_purchases_next_to_its_edges() {
    let (mut db, a) = setup();
    // January and March are imported first; February comes later.
    db.import_batch(&[
        row(a, "2026-01-02", -100, "Other"),
        row(a, "2026-01-31", -450, "Starbucks"),
    ])
    .unwrap();
    db.import_batch(&[
        row(a, "2026-03-01", -450, "Starbucks"),
        row(a, "2026-03-30", -100, "Other"),
    ])
    .unwrap();
    // February's own coffees, a day from January's last and March's first.
    let s = db
        .import_batch(&[
            row(a, "2026-02-01", -450, "STARBUCKS STORE"),
            row(a, "2026-02-15", -900, "Lunch"),
            row(a, "2026-02-28", -450, "STARBUCKS STORE"),
        ])
        .unwrap();
    assert_eq!((s.added, s.skipped), (3, 0));
    // An overlapping re-download of January in another format is still caught.
    let s = db
        .import_batch(&[row(a, "2026-01-30", -450, "STARBUCKS STORE")])
        .unwrap();
    assert_eq!((s.added, s.skipped), (0, 1));
}

#[test]
fn reimporting_the_same_file_still_adds_nothing() {
    let (mut db, a) = setup();
    let rows = [
        row(a, "2026-10-03", -400, "Starbucks"),
        row(a, "2026-10-03", -400, "Starbucks"),
    ];
    db.import_batch(&rows).unwrap();
    let s = db.import_batch(&rows).unwrap();
    assert_eq!((s.added, s.skipped), (0, 2));
    assert_eq!(count(&db), 2);
}
