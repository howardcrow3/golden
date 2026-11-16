use fin_plan::debt::*;

fn d(name: &str, bal: i64, apr: i64, min: i64) -> Debt {
    Debt {
        name: name.into(),
        balance_cents: bal,
        apr_bp: apr,
        min_cents: min,
    }
}

#[test]
fn one_debt_matches_the_textbook_answer() {
    // $1,000 at 12% a year (1% a month), $100 a month: 11 payments, total interest about $59.
    let p = payoff(&[d("Card", 100_000, 1200, 10_000)], 0, Strategy::Avalanche).unwrap();
    assert!(p.feasible);
    assert_eq!(p.months, 11);
    assert!(
        (5_800..=6_000).contains(&p.total_interest_cents),
        "{}",
        p.total_interest_cents
    );
    assert_eq!(p.total_paid_cents, 100_000 + p.total_interest_cents);
}

#[test]
fn interest_free_debt_is_balance_over_payment() {
    let p = payoff(&[d("Loan", 100_000, 0, 25_000)], 0, Strategy::Avalanche).unwrap();
    assert_eq!((p.months, p.total_interest_cents), (4, 0));
}

#[test]
fn extra_money_shortens_and_cheapens() {
    let debts = [
        d("Card", 500_000, 2400, 15_000),
        d("Car", 800_000, 600, 20_000),
    ];
    let base = payoff(&debts, 0, Strategy::Avalanche).unwrap();
    let more = payoff(&debts, 30_000, Strategy::Avalanche).unwrap();
    assert!(more.months < base.months);
    assert!(more.total_interest_cents < base.total_interest_cents);
}

#[test]
fn avalanche_never_costs_more_interest_than_snowball() {
    let debts = [
        d("Small low rate", 100_000, 500, 5_000),
        d("Big high rate", 900_000, 2800, 20_000),
        d("Medium", 400_000, 1500, 10_000),
    ];
    let a = payoff(&debts, 25_000, Strategy::Avalanche).unwrap();
    let s = payoff(&debts, 25_000, Strategy::Snowball).unwrap();
    assert!(a.total_interest_cents <= s.total_interest_cents);
}

#[test]
fn snowball_clears_the_smallest_balance_first() {
    let debts = [
        d("Small", 100_000, 500, 5_000),
        d("Big", 900_000, 2800, 20_000),
    ];
    let s = payoff(&debts, 50_000, Strategy::Snowball).unwrap();
    assert_eq!(s.order[0].name, "Small");
    assert!(s.order[0].month < s.order[1].month);
}

#[test]
fn money_is_conserved_exactly() {
    let debts = [d("A", 123_457, 1899, 4_000), d("B", 987_654, 799, 12_345)];
    let p = payoff(&debts, 7_777, Strategy::Snowball).unwrap();
    assert!(p.feasible);
    assert_eq!(
        p.total_paid_cents,
        123_457 + 987_654 + p.total_interest_cents
    );
}

#[test]
fn a_payment_that_never_covers_interest_is_infeasible() {
    // Interest is $200 a month; paying $100 only digs deeper.
    let p = payoff(
        &[d("Trap", 1_000_000, 2400, 10_000)],
        0,
        Strategy::Avalanche,
    )
    .unwrap();
    assert!(!p.feasible);
    assert_eq!(p.months, MAX_MONTHS);
}

#[test]
fn nothing_owed_is_done_immediately_and_bad_input_is_refused() {
    let p = payoff(&[], 0, Strategy::Avalanche).unwrap();
    assert!(p.feasible && p.months == 0 && p.order.is_empty());
    let zero = payoff(&[d("Paid", 0, 1999, 0)], 0, Strategy::Snowball).unwrap();
    assert!(zero.feasible && zero.months == 0);
    assert_eq!(
        payoff(&[d("Bad", -1, 0, 0)], 0, Strategy::Avalanche),
        Err(DebtError::Negative("Bad".into()))
    );
    assert_eq!(
        payoff(&[], -1, Strategy::Avalanche),
        Err(DebtError::NegativeExtra)
    );
}

#[test]
fn freed_minimums_roll_to_the_next_debt() {
    // $100 + $1,000 at $50 each: A clears in 2 months, then its $50 joins B's, so B finishes
    // in 11 months instead of 20.
    let debts = [d("A", 10_000, 0, 5_000), d("B", 100_000, 0, 5_000)];
    let p = payoff(&debts, 0, Strategy::Snowball).unwrap();
    assert_eq!(p.months, 11);
}

#[test]
fn a_runaway_balance_is_infeasible_not_a_crash() {
    // 100% a year with a $1 payment on $1 million: it balloons, and must not overflow.
    let p = payoff(
        &[d("Runaway", 100_000_000, 10_000, 100)],
        0,
        Strategy::Avalanche,
    )
    .unwrap();
    assert!(!p.feasible);
}
