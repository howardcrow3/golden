//! "How much can I spend?" Retirees ask for a spending figure, not only a chance of success.
//! This finds the largest yearly spending (in today's dollars) at which the plan still works in
//! at least `target` of the Monte Carlo trials. The same seed is used for every trial set, so the
//! success rate only falls as spending rises and a bisection is sound.
use crate::params::{Assumptions, ParamError, Picks};
use crate::projection::PlanInput;
use crate::scenario;
use fin_core::Money;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SafeSpend {
    /// The success rate the figure was solved for, e.g. 0.85.
    pub target: f64,
    /// Largest yearly spending that still meets `target`, rounded down to $100.
    pub spending: Money,
    /// What the plan currently assumes.
    pub current: Money,
    /// True when even zero spending falls short (nothing can be spent safely).
    pub nothing_safe: bool,
    /// True when the search ceiling was reached, so the real figure is higher.
    pub capped: bool,
}

fn works(
    base: &PlanInput,
    a: &Assumptions,
    picks: &Picks,
    spending: Money,
    runs: usize,
    seed: u64,
) -> Result<f64, ParamError> {
    let mut plan = base.clone();
    plan.annual_spending = spending;
    Ok(scenario::run(&plan, a, picks, runs, seed)?
        .monte_carlo
        .success_rate)
}

pub fn safe_spend(
    base: &PlanInput,
    a: &Assumptions,
    picks: &Picks,
    target: f64,
    runs: usize,
    seed: u64,
) -> Result<SafeSpend, ParamError> {
    let current = base.annual_spending;
    let step = Money::from_dollars(100);
    // The ceiling: three times today's spending, or a tenth of the portfolio if that is more.
    let ceiling = Money::from_cents(current.cents() * 3)
        .max(Money::from_cents(base.accounts.total().cents() / 10));
    let ceiling = if ceiling.cents() <= 0 {
        Money::from_dollars(100_000)
    } else {
        ceiling
    };
    let ok = |s: Money| works(base, a, picks, s, runs, seed).map(|r| r >= target);

    if !ok(Money::ZERO)? {
        return Ok(SafeSpend {
            target,
            spending: Money::ZERO,
            current,
            nothing_safe: true,
            capped: false,
        });
    }
    if ok(ceiling)? {
        return Ok(SafeSpend {
            target,
            spending: ceiling,
            current,
            nothing_safe: false,
            capped: true,
        });
    }
    let (mut lo, mut hi) = (0i64, ceiling.cents() / step.cents() + 1); // in $100 steps
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if ok(Money::from_cents(mid * step.cents()))? {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(SafeSpend {
        target,
        spending: Money::from_cents(lo * step.cents()),
        current,
        nothing_safe: lo == 0,
        capped: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::fixtures::couple;

    fn solve(input: &PlanInput, target: f64) -> SafeSpend {
        safe_spend(
            input,
            &Assumptions::defaults(),
            &Picks::new(),
            target,
            150,
            7,
        )
        .unwrap()
    }

    #[test]
    fn the_answer_meets_the_target_and_a_bit_more_does_not() {
        let input = couple();
        let s = solve(&input, 0.85);
        assert!(!s.nothing_safe);
        let a = Assumptions::defaults();
        let p = Picks::new();
        assert!(works(&input, &a, &p, s.spending, 150, 7).unwrap() >= 0.85);
        let more = s.spending + Money::from_dollars(300);
        if !s.capped {
            assert!(works(&input, &a, &p, more, 150, 7).unwrap() < 0.85);
        }
    }

    #[test]
    fn a_stricter_target_allows_less_spending() {
        let input = couple();
        let loose = solve(&input, 0.70);
        let strict = solve(&input, 0.95);
        assert!(strict.spending <= loose.spending);
    }

    #[test]
    fn more_savings_allow_more_spending() {
        let mut rich = couple();
        rich.accounts.traditional = Money::from_cents(rich.accounts.traditional.cents() * 2);
        assert!(solve(&rich, 0.85).spending >= solve(&couple(), 0.85).spending);
    }

    #[test]
    fn no_money_and_no_income_means_nothing_is_safe() {
        let mut broke = couple();
        broke.accounts.cash = Money::ZERO;
        broke.accounts.taxable = Money::ZERO;
        broke.accounts.traditional = Money::ZERO;
        broke.accounts.roth = Money::ZERO;
        broke.streams.clear();
        broke.social_security.clear();
        let s = solve(&broke, 0.85);
        assert!(s.nothing_safe);
        assert_eq!(s.spending, Money::ZERO);
    }

    #[test]
    fn it_reports_the_current_spending_for_comparison() {
        let input = couple();
        assert_eq!(solve(&input, 0.85).current, input.annual_spending);
    }
}
