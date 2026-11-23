use chrono::NaiveDate;
use fin_core::Money;
use fin_db::bank::BankTxn;
use fin_db::ledger::NewTxn;
use fin_db::Db;

fn d(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

fn bank(
    id: &str,
    acct: fin_db::ledger::AccountId,
    day: &str,
    cents: i64,
    pending: bool,
) -> BankTxn {
    BankTxn {
        external_id: id.into(),
        account: acct,
        day: d(day),
        amount: Money::from_cents(cents),
        payee: "Shell".into(),
        memo: String::new(),
        pending,
        category: None,
    }
}

fn setup() -> (Db, fin_db::ledger::AccountId) {
    let db = Db::open_in_memory().unwrap();
    let a = db.add_account("Checking", "checking", None, None).unwrap();
    (db, a)
}

#[test]
fn new_lines_are_added_and_resyncing_updates_not_duplicates() {
    let (mut db, a) = setup();
    let s = db
        .apply_bank_changes(&[bank("t1", a, "2026-11-02", -1250, true)], &[])
        .unwrap();
    assert_eq!((s.added, s.updated), (1, 0));
    let s = db
        .apply_bank_changes(&[bank("t1", a, "2026-11-03", -1260, false)], &[])
        .unwrap();
    assert_eq!((s.added, s.updated), (0, 1));
    assert_eq!(db.txn_count().unwrap(), 1);
    let t = &db.txns_between(d("2026-11-01"), d("2026-11-30")).unwrap()[0];
    assert_eq!(t.amount.cents(), -1260);
    assert_eq!(t.day, d("2026-11-03"));
}

#[test]
fn an_update_keeps_the_category_the_person_chose() {
    let (mut db, a) = setup();
    let mut t = bank("t1", a, "2026-11-02", -500, false);
    t.category = Some("Fuel".into());
    db.apply_bank_changes(&[t.clone()], &[]).unwrap();
    let id = db.txns_between(d("2026-11-01"), d("2026-11-30")).unwrap()[0].id;
    db.set_txn_category(id, Some("Gas")).unwrap();
    t.category = Some("Fuel".into());
    db.apply_bank_changes(&[t], &[]).unwrap();
    let got = db.get_txn(id).unwrap();
    assert_eq!(got.category.as_deref(), Some("Gas"));
}

#[test]
fn removed_lines_are_deleted_and_unknown_ids_ignored() {
    let (mut db, a) = setup();
    db.apply_bank_changes(&[bank("t1", a, "2026-11-02", -500, true)], &[])
        .unwrap();
    let s = db
        .apply_bank_changes(&[], &["t1".into(), "never".into()])
        .unwrap();
    assert_eq!(s.removed, 1);
    assert_eq!(db.txn_count().unwrap(), 0);
}

#[test]
fn account_links_round_trip_and_unlink_removes_synced_lines_only() {
    let (mut db, a) = setup();
    db.link_bank_account("bank-acc", "item-1", a).unwrap();
    assert_eq!(db.bank_account("bank-acc").unwrap(), Some(a));
    assert_eq!(db.bank_account("other").unwrap(), None);
    db.add_txn(&NewTxn {
        account: a,
        day: d("2026-10-01"),
        amount: Money::from_cents(-900),
        payee: "Statement line".into(),
        memo: String::new(),
        category: None,
        stream: None,
    })
    .unwrap();
    db.apply_bank_changes(&[bank("t1", a, "2026-11-02", -500, false)], &[])
        .unwrap();
    assert_eq!(db.unlink_bank_item("item-1").unwrap(), 1);
    assert_eq!(db.txn_count().unwrap(), 1);
    assert_eq!(db.bank_account("bank-acc").unwrap(), None);
}

#[test]
fn a_statement_replaces_the_bank_lines_it_covers() {
    let (mut db, a) = setup();
    db.apply_bank_changes(
        &[
            bank("t1", a, "2026-11-02", -1250, false),
            bank("t2", a, "2026-11-20", -999, true),
        ],
        &[],
    )
    .unwrap();
    // The statement posts the first one a day later and does not yet have the second.
    db.add_txn(&NewTxn {
        account: a,
        day: d("2026-11-03"),
        amount: Money::from_cents(-1250),
        payee: "SHELL OIL".into(),
        memo: String::new(),
        category: None,
        stream: None,
    })
    .unwrap();
    assert_eq!(db.reconcile_bank(a).unwrap(), 1);
    assert_eq!(db.txn_count().unwrap(), 2);
    // Running it again changes nothing.
    assert_eq!(db.reconcile_bank(a).unwrap(), 0);
}

#[test]
fn one_statement_line_covers_only_one_bank_line() {
    let (mut db, a) = setup();
    db.apply_bank_changes(
        &[
            bank("t1", a, "2026-11-02", -400, false),
            bank("t2", a, "2026-11-02", -400, false),
        ],
        &[],
    )
    .unwrap();
    db.add_txn(&NewTxn {
        account: a,
        day: d("2026-11-02"),
        amount: Money::from_cents(-400),
        payee: "Coffee".into(),
        memo: String::new(),
        category: None,
        stream: None,
    })
    .unwrap();
    assert_eq!(db.reconcile_bank(a).unwrap(), 1);
    assert_eq!(db.txn_count().unwrap(), 2);
}

#[test]
fn lines_too_far_apart_or_different_amounts_are_kept() {
    let (mut db, a) = setup();
    db.apply_bank_changes(&[bank("t1", a, "2026-11-02", -400, false)], &[])
        .unwrap();
    for (day, cents) in [("2026-11-09", -400), ("2026-11-02", -401)] {
        db.add_txn(&NewTxn {
            account: a,
            day: d(day),
            amount: Money::from_cents(cents),
            payee: "x".into(),
            memo: String::new(),
            category: None,
            stream: None,
        })
        .unwrap();
    }
    assert_eq!(db.reconcile_bank(a).unwrap(), 0);
    assert_eq!(db.txn_count().unwrap(), 3);
}

#[test]
fn erasing_personal_data_clears_bank_links() {
    let (mut db, a) = setup();
    db.link_bank_account("b", "i", a).unwrap();
    db.apply_bank_changes(&[bank("t1", a, "2026-11-02", -400, false)], &[])
        .unwrap();
    db.erase_personal_data(&[]).unwrap();
    assert_eq!(db.txn_count().unwrap(), 0);
    assert_eq!(db.bank_account("b").unwrap(), None);
}
