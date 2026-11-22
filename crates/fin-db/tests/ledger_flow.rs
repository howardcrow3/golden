use chrono::NaiveDate;
use fin_core::{Grain, Money, PeriodKey};
use fin_db::ledger::{Metric, NewTxn};
use fin_db::Db;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn row(
    acct: fin_db::ledger::AccountId,
    day: NaiveDate,
    cents: i64,
    payee: &str,
    cat: Option<&str>,
) -> NewTxn {
    NewTxn {
        account: acct,
        day,
        amount: Money::from_cents(cents),
        payee: payee.into(),
        memo: String::new(),
        category: cat.map(String::from),
        stream: None,
    }
}

fn seeded() -> (Db, fin_db::ledger::AccountId) {
    let mut db = Db::open_in_memory().unwrap();
    let a = db
        .add_account("Checking", "checking", Some("Chase"), Some("4821"))
        .unwrap();
    let rows = vec![
        row(a, d(2026, 10, 1), 500_000, "Acme Payroll", Some("Salary")),
        row(a, d(2026, 10, 3), -150_000, "Landlord", Some("Rent")),
        row(a, d(2026, 10, 9), -4_500, "Grocer", Some("Groceries")),
        row(a, d(2026, 10, 17), -6_000, "Grocer", Some("Groceries")),
        row(a, d(2026, 11, 1), 500_000, "Acme Payroll", Some("Salary")),
        row(a, d(2026, 11, 2), -150_000, "Landlord", Some("Rent")),
        row(a, d(2026, 11, 14), 2_000_000, "Bonus", Some("Salary")),
    ];
    let s = db.import_batch(&rows).unwrap();
    assert_eq!((s.added, s.skipped), (7, 0));
    (db, a)
}

#[test]
fn reimporting_the_same_file_adds_nothing() {
    let (mut db, a) = seeded();
    let again = vec![row(
        a,
        d(2026, 10, 1),
        500_000,
        "Acme Payroll",
        Some("Salary"),
    )];
    let s = db.import_batch(&again).unwrap();
    assert_eq!((s.added, s.skipped), (0, 1));
    assert_eq!(db.txn_count().unwrap(), 7);
}

#[test]
fn identical_purchases_in_one_file_are_both_kept() {
    let mut db = Db::open_in_memory().unwrap();
    let a = db.add_account("Card", "credit", None, None).unwrap();
    let two = vec![
        row(a, d(2026, 5, 5), -400, "Cafe", None),
        row(a, d(2026, 5, 5), -400, "Cafe", None),
    ];
    assert_eq!(db.import_batch(&two).unwrap().added, 2);
    assert_eq!(db.import_batch(&two).unwrap().added, 0);
    assert_eq!(db.balance(a).unwrap(), Money::from_cents(-800));
}

#[test]
fn month_totals_show_the_lumpy_bonus_in_its_own_month() {
    let (db, _) = seeded();
    let t = db
        .totals(d(2026, 10, 1), d(2026, 12, 31), Grain::Month)
        .unwrap();
    assert_eq!(
        t.iter().map(|p| p.period.as_str()).collect::<Vec<_>>(),
        ["2026-10", "2026-11", "2026-12"]
    );
    assert_eq!(t[0].income, Money::from_dollars(5_000));
    assert_eq!(t[1].income, Money::from_dollars(25_000));
    assert_eq!(t[2].count, 0); // empty periods are filled with zeros
    assert_eq!(
        t[0].net,
        Money::from_cents(500_000 - 150_000 - 4_500 - 6_000)
    );
}

#[test]
fn every_grain_sums_to_the_same_grand_total() {
    let (db, _) = seeded();
    let mut totals = vec![];
    for g in Grain::ALL {
        let t = db.totals(d(2026, 10, 1), d(2026, 11, 30), g).unwrap();
        totals.push(t.iter().map(|p| p.net).sum::<Money>());
    }
    assert!(totals.windows(2).all(|w| w[0] == w[1]), "{totals:?}");
}

#[test]
fn quarter_and_year_rollups() {
    let (db, _) = seeded();
    let q = db
        .totals(d(2026, 10, 1), d(2026, 12, 31), Grain::Quarter)
        .unwrap();
    assert_eq!(q.len(), 1);
    assert_eq!(q[0].period, "2026-Q4");
    let y = db
        .totals(d(2026, 1, 1), d(2026, 12, 31), Grain::Year)
        .unwrap();
    assert_eq!(y[0].period, "2026");
    assert_eq!(y[0].income, Money::from_dollars(30_000));
}

#[test]
fn weeks_use_iso_boundaries() {
    let (db, _) = seeded();
    // Oct 9 (Fri) and Oct 17 (Sat) are in different ISO weeks.
    let w = db
        .totals(d(2026, 10, 5), d(2026, 10, 18), Grain::Week)
        .unwrap();
    assert_eq!(w.len(), 2);
    assert_eq!(w[0].spending, Money::from_cents(4_500));
    assert_eq!(w[1].spending, Money::from_cents(6_000));
}

#[test]
fn category_breakdown_sorted_by_size() {
    let (db, _) = seeded();
    let p = PeriodKey::containing(Grain::Month, d(2026, 10, 1));
    let spend = db.category_totals(p, Metric::Spending).unwrap();
    assert_eq!(spend[0], ("Rent".into(), Money::from_dollars(1_500)));
    assert_eq!(spend[1], ("Groceries".into(), Money::from_cents(10_500)));
    let inc = db.category_totals(p, Metric::Income).unwrap();
    assert_eq!(
        inc,
        vec![("Salary".to_string(), Money::from_dollars(5_000))]
    );
}

#[test]
fn people_roundtrip_with_birthdates() {
    let db = Db::open_in_memory().unwrap();
    db.add_person("Pat", d(1976, 4, 2), false).unwrap();
    db.add_person("Sam", d(2015, 9, 30), true).unwrap();
    let ps = db.people().unwrap();
    assert_eq!(ps.len(), 2);
    assert_eq!(ps[0].age_on(d(2026, 11, 15)), 50);
    assert!(ps[1].is_dependent);
    assert!(db.add_person(" ", d(2000, 1, 1), false).is_err());
}

#[test]
fn file_database_persists_across_opens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("golden.db");
    {
        let db = Db::open(&path).unwrap();
        let a = db.add_account("Savings", "savings", None, None).unwrap();
        db.add_txn(&row(a, d(2026, 1, 2), 1_000, "Interest", None))
            .unwrap();
    }
    let db = Db::open(&path).unwrap();
    assert_eq!(db.txn_count().unwrap(), 1);
    assert_eq!(
        db.schema_version().unwrap(),
        fin_db::migrations::latest_version()
    );
}

#[test]
fn documents_round_trip_and_erasing_personal_data_removes_them() {
    use fin_db::documents::NewDoc;
    let db = fin_db::Db::open_in_memory().unwrap();
    let d = db
        .add_document(&NewDoc {
            name: "a.txt",
            kind: "Document",
            year: None,
            added: "2026-11-22",
            fields: "[]",
            body: "hello",
            content: b"hello",
        })
        .unwrap();
    assert_eq!(d.size, 5);
    assert_eq!(db.get_document(d.id).unwrap().unwrap().1, b"hello");
    assert_eq!(db.list_documents("ell").unwrap().len(), 1);
    db.erase_personal_data(&[]).unwrap();
    assert!(db.list_documents("").unwrap().is_empty());
    assert!(!db.delete_document(d.id).unwrap());
}
