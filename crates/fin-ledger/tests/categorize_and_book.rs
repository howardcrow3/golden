//! Integration: parse a bank CSV, categorize it with rules, store it, and read category totals.
use fin_core::{Grain, Money, NaiveDate, PeriodKey};
use fin_db::ledger::{Metric, NewTxn};
use fin_db::Db;
use fin_ingest::read_statement;
use fin_ledger::rules::RuleSet;

const CSV: &str = "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n10/02/2026,POS DEBIT TRADER JOE'S #12,-82.10\n10/04/2026,CHECKCARD STARBUCKS 5531,-6.45\n10/05/2026,NETFLIX.COM,-15.49\n10/06/2026,MYSTERY VENDOR,-10.00\n";

#[test]
fn categorized_import_produces_category_totals_and_flags_unknowns() {
    let rules = RuleSet::starter();
    let parsed = read_statement(CSV.as_bytes()).unwrap();
    let mut db = Db::open_in_memory().unwrap();
    let a = db.add_account("Checking", "checking", None, None).unwrap();
    let rows: Vec<NewTxn> = parsed
        .into_iter()
        .map(|t| NewTxn {
            account: a,
            day: t.day,
            amount: t.amount,
            category: rules.categorize(&t.payee, t.amount).map(String::from),
            payee: t.payee,
            memo: t.memo,
            stream: None,
        })
        .collect();
    assert_eq!(db.import_batch(&rows).unwrap().added, 5);

    let oct = PeriodKey::containing(Grain::Month, NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());
    let spend = db.category_totals(oct, Metric::Spending).unwrap();
    assert_eq!(
        spend[0],
        ("Groceries".to_string(), Money::from_cents(8_210))
    );
    assert!(spend.contains(&("Uncategorized".to_string(), Money::from_dollars(10))));
    assert!(spend.iter().any(|(c, _)| c == "Subscriptions"));
    let income = db.category_totals(oct, Metric::Income).unwrap();
    assert_eq!(
        income,
        vec![("Salary".to_string(), Money::from_dollars(5000))]
    );
}

#[test]
fn teaching_the_app_once_fixes_the_next_import() {
    let mut rules = RuleSet::starter();
    assert_eq!(
        rules.categorize("MYSTERY VENDOR 22", Money::from_dollars(-10)),
        None
    );
    rules.learn("MYSTERY VENDOR 22", "Hobbies");
    assert_eq!(
        rules.categorize("MYSTERY VENDOR 31 PORTLAND OR", Money::from_dollars(-4)),
        Some("Hobbies")
    );
}
