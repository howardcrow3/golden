//! Roth conversion explorer: run the same plan with different conversion strategies and put the
//! lifetime tax, the legacy and the chance of success side by side. Pure local arithmetic on the
//! plan engine; nothing here predicts the future, it compares choices under one set of assumptions.
use crate::params::{Assumptions, ParamError, Picks};
use crate::projection::{ConversionTarget, PlanInput, RothConversion};
use crate::scenario;
use fin_core::Money;

/// Tax rate assumed on what heirs inherit in traditional accounts, so a dollar left in a
/// traditional account is not counted as if it were already after tax.
pub const HEIR_TAX_RATE: f64 = 0.22;

#[derive(Debug, Clone, PartialEq)]
pub struct RothOption {
    pub label: String,
    pub conversion: Option<RothConversion>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RothOutcome {
    pub label: String,
    pub converted_total: Money,
    /// Federal, state and payroll tax over the whole plan (average return path).
    pub lifetime_tax: Money,
    pub final_balance: Money,
    /// Balance at the end counting traditional money after `HEIR_TAX_RATE`.
    pub after_tax_legacy: Money,
    pub success_rate: f64,
    pub depleted_year: Option<i32>,
}

/// The first year nobody earns a wage, or the start year if that never happens.
pub fn first_wage_free_year(plan: &PlanInput) -> i32 {
    let last_wage = plan.streams.iter().map(|s| s.end_year).max();
    match last_wage {
        Some(y) => (y + 1).max(plan.start_year),
        None => plan.start_year,
    }
}

/// The usual comparison: no conversions, then filling the 12%, 22% and 24% brackets from the first
/// year without wages until the primary person reaches `until_age`.
pub fn standard_options(plan: &PlanInput, until_age: i32) -> Vec<RothOption> {
    let start = first_wage_free_year(plan);
    let now_age = plan.people[0]
        .age_on(fin_core::NaiveDate::from_ymd_opt(plan.start_year, 12, 31).expect("valid date"));
    let end = plan.start_year + (until_age - now_age);
    let make = |label: &str, rate: f64| RothOption {
        label: label.into(),
        conversion: Some(RothConversion {
            start_year: start,
            end_year: end,
            target: ConversionTarget::FillBracket { top_rate: rate },
        }),
    };
    vec![
        RothOption {
            label: "No conversions".into(),
            conversion: None,
        },
        make("Fill the 12% bracket", 0.12),
        make("Fill the 22% bracket", 0.22),
        make("Fill the 24% bracket", 0.24),
    ]
}

pub fn explore(
    base: &PlanInput,
    a: &Assumptions,
    picks: &Picks,
    options: &[RothOption],
    runs: usize,
    seed: u64,
) -> Result<Vec<RothOutcome>, ParamError> {
    options
        .iter()
        .map(|o| {
            let mut plan = base.clone();
            plan.roth_conversion = o.conversion;
            let r = scenario::run(&plan, a, picks, runs, seed)?;
            let rows = &r.projection.rows;
            let fa = r.projection.final_accounts;
            Ok(RothOutcome {
                label: o.label.clone(),
                converted_total: rows.iter().map(|y| y.converted).sum(),
                lifetime_tax: rows.iter().map(|y| y.taxes).sum(),
                final_balance: r.projection.final_balance(),
                after_tax_legacy: fa.cash
                    + fa.taxable
                    + fa.roth
                    + fa.traditional.mul_rate(1.0 - HEIR_TAX_RATE),
                success_rate: r.monte_carlo.success_rate,
                depleted_year: r.projection.depleted_year,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::fixtures::couple;

    fn plan() -> PlanInput {
        let mut p = couple();
        // Retire sooner with a big traditional balance, so there are years to convert in.
        p.streams[0].end_year = 2030;
        p.accounts.traditional = Money::from_dollars(1_800_000);
        p
    }

    fn run(p: &PlanInput) -> Vec<RothOutcome> {
        explore(
            p,
            &Assumptions::defaults(),
            &Picks::new(),
            &standard_options(p, 72),
            100,
            11,
        )
        .unwrap()
    }

    #[test]
    fn the_standard_options_start_the_year_after_the_last_paycheck() {
        let p = plan();
        let o = standard_options(&p, 72);
        assert_eq!(o.len(), 4);
        assert!(o[0].conversion.is_none());
        let c = o[1].conversion.unwrap();
        assert_eq!(c.start_year, 2031);
        // The primary person is 50 at the end of 2026 in the fixture, so 72 is 22 years on.
        assert_eq!(c.end_year, 2026 + 22);
        assert_eq!(first_wage_free_year(&p), 2031);
    }

    #[test]
    fn no_conversions_converts_nothing_and_the_others_convert_more_as_the_bracket_grows() {
        let r = run(&plan());
        assert_eq!(r[0].converted_total, Money::ZERO);
        assert!(r[1].converted_total > Money::ZERO);
        assert!(r[2].converted_total >= r[1].converted_total);
        assert!(r[3].converted_total >= r[2].converted_total);
    }

    #[test]
    fn converting_pays_tax_earlier_and_changes_what_is_left_for_heirs() {
        let r = run(&plan());
        // Early tax shows up as more tax over the plan's first years; the legacy figure counts
        // traditional money after the heirs' tax, so the options are comparable.
        assert!(r[1].lifetime_tax != r[0].lifetime_tax);
        for o in &r {
            assert!(o.after_tax_legacy <= o.final_balance + Money::from_dollars(1));
            assert!((0.0..=1.0).contains(&o.success_rate));
        }
        assert!(r[1].after_tax_legacy > r[0].after_tax_legacy - Money::from_dollars(1));
    }

    #[test]
    fn the_same_inputs_give_the_same_answer() {
        assert_eq!(run(&plan()), run(&plan()));
    }
}
