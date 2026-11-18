use fin_app::App;
use fin_core::NaiveDate;

fn d(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, day).unwrap()
}
fn app() -> App {
    App::in_memory().unwrap().with_clock(|| d(11, 16))
}
fn month(a: &App) -> fin_app::PeriodRow {
    a.cash_flow(d(11, 1), d(11, 30), "month").unwrap().remove(0)
}

#[test]
fn a_payment_between_two_accounts_is_neither_income_nor_spending() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n11/01/2026,ACME PAYROLL,3000.00\n11/05/2026,TO SAVINGS,-500.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    a.import_statement(
        b"Date,Description,Amount\n11/06/2026,FROM CHECKING,500.00\n",
        "Savings",
        "savings",
    )
    .unwrap();
    let m = month(&a);
    assert_eq!((m.income_cents, m.spending_cents), (300_000, 0));
    // Savings still holds the money: net worth counts every transaction.
    assert_eq!(a.balance_sheet().unwrap().net_worth_cents, 300_000);
    let rows = a.transactions("2026-11-01", "2026-11-30", false).unwrap();
    assert_eq!(
        rows.iter()
            .filter(|r| r.category.as_deref() == Some("Transfer"))
            .count(),
        2
    );
}

#[test]
fn the_same_amount_in_one_account_is_not_a_transfer() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n11/05/2026,REFUND SHOP,500.00\n11/06/2026,NEW PURCHASE,-500.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    let m = month(&a);
    assert_eq!((m.income_cents, m.spending_cents), (50_000, 50_000));
}

#[test]
fn sides_too_far_apart_are_left_alone() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n11/01/2026,ODD OUT,-500.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    a.import_statement(
        b"Date,Description,Amount\n11/20/2026,UNRELATED DEPOSIT,500.00\n",
        "Savings",
        "savings",
    )
    .unwrap();
    let m = month(&a);
    assert_eq!((m.income_cents, m.spending_cents), (50_000, 50_000));
}

#[test]
fn a_category_the_person_chose_is_never_overridden_and_marking_is_repeatable() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n11/05/2026,GIFT TO SIS,-500.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    let id = a.transactions("2026-11-01", "2026-11-30", false).unwrap()[0].id;
    a.set_category(id, "Gifts", false).unwrap();
    a.import_statement(
        b"Date,Description,Amount\n11/06/2026,GIFT RECEIVED,500.00\n",
        "Savings",
        "savings",
    )
    .unwrap();
    assert_eq!(a.mark_transfers().unwrap(), 0);
    assert_eq!(month(&a).spending_cents, 50_000);
}

#[test]
fn spending_by_category_leaves_transfers_out() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n11/05/2026,TO SAVINGS,-500.00\n11/07/2026,GROCERY MART,-80.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    a.import_statement(
        b"Date,Description,Amount\n11/05/2026,FROM CHECKING,500.00\n",
        "Savings",
        "savings",
    )
    .unwrap();
    let cats = a.categories_str("2026-11-10", "month", false).unwrap();
    assert!(cats.iter().all(|c| c.category != "Transfer"));
    assert_eq!(cats.iter().map(|c| c.cents).sum::<i64>(), 8_000);
}
