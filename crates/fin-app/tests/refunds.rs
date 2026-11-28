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
fn a_full_refund_cancels_the_purchase_in_the_totals() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n11/01/2026,ACME PAYROLL,3000.00\n11/03/2026,TARGET #1234,-80.00\n11/10/2026,TARGET RETURN,80.00\n11/11/2026,SHELL OIL,-40.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    let m = month(&a);
    // Income is the pay only and spending only the fuel: the return is not income.
    assert_eq!((m.income_cents, m.spending_cents), (300_000, 4_000));
    // The money is still counted in the balance, and both lines are still listed.
    assert_eq!(a.balance_sheet().unwrap().net_worth_cents, 300_000 - 4_000);
    let rows = a.transactions("2026-11-01", "2026-11-30", false).unwrap();
    assert_eq!(rows.len(), 4);
}

#[test]
fn a_refund_without_a_matching_purchase_stays_income() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n11/03/2026,TARGET #1234,-80.00\n11/10/2026,TARGET RETURN,35.00\n11/12/2026,WALMART REFUND,80.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    let m = month(&a);
    // A partial refund and a refund from another shop are left alone.
    assert_eq!((m.income_cents, m.spending_cents), (11_500, 8_000));
}

#[test]
fn each_purchase_is_cancelled_once() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n11/03/2026,TARGET,-20.00\n11/05/2026,TARGET,-20.00\n11/10/2026,TARGET RETURN,20.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    let m = month(&a);
    // One purchase is cancelled by the one refund; the other still counts.
    assert_eq!((m.income_cents, m.spending_cents), (0, 2_000));
}

#[test]
fn a_refund_long_after_the_purchase_is_not_paired() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n01/03/2026,TARGET,-20.00\n11/10/2026,TARGET RETURN,20.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    assert_eq!(month(&a).income_cents, 2_000);
}
