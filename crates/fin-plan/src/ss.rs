//! Social Security claiming: how the monthly benefit changes with the age it starts, and a
//! side-by-side of claiming ages. Figures follow the SSA planner pages
//! (https://www.ssa.gov/benefits/retirement/planner/agereduction.html and
//! https://www.ssa.gov/benefits/retirement/planner/delayret.html): the full retirement age table
//! by birth year, the reduction for claiming early (5/9 of 1% a month for the first 36 months,
//! 5/12 of 1% after that, which reproduces the SSA's 25% to 30% totals at 62) and delayed
//! retirement credits of 2/3 of 1% a month (8% a year) up to age 70.
//!
//! Not modelled: spousal and survivor benefits, the earnings test before full retirement age,
//! and people born on January 1 (the SSA treats them as born the year before).
use crate::params::{Assumptions, ParamError, Picks};
use crate::projection::PlanInput;
use crate::roth::HEIR_TAX_RATE;
use crate::scenario;
use fin_core::Money;

/// Full retirement age in months for someone born in `birth_year`.
pub fn fra_months(birth_year: i32) -> i32 {
    match birth_year {
        ..=1954 => 66 * 12,
        1955 => 66 * 12 + 2,
        1956 => 66 * 12 + 4,
        1957 => 66 * 12 + 6,
        1958 => 66 * 12 + 8,
        1959 => 66 * 12 + 10,
        _ => 67 * 12,
    }
}

/// The share of the full-retirement-age benefit paid when claiming at `claim_months` of age
/// (62 years to 70 years; ages outside that are clamped).
pub fn claim_factor(birth_year: i32, claim_months: i32) -> f64 {
    let fra = fra_months(birth_year);
    let claim = claim_months.clamp(62 * 12, 70 * 12);
    if claim < fra {
        let early = (fra - claim) as f64;
        let first = early.min(36.0);
        let rest = (early - 36.0).max(0.0);
        1.0 - (first * 5.0 / 9.0 + rest * 5.0 / 12.0) / 100.0
    } else {
        // Credits only run from full retirement age to 70.
        let late = (claim.min(70 * 12) - fra) as f64;
        1.0 + late * (2.0 / 3.0) / 100.0
    }
}

/// The age at which the person who claims at `later` has been paid as much in total as the one
/// who claims at `earlier`, given each one's yearly benefit. None if the later claim pays no more.
pub fn break_even_age(
    earlier: i32,
    earlier_annual: Money,
    later: i32,
    later_annual: Money,
) -> Option<f64> {
    let (ea, la) = (earlier_annual.cents() as f64, later_annual.cents() as f64);
    if la <= ea {
        return None;
    }
    Some((la * later as f64 - ea * earlier as f64) / (la - ea))
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClaimOutcome {
    pub claim_age: i32,
    /// Yearly benefit for each person at this claiming age, today's dollars.
    pub annual_benefits: Vec<Money>,
    /// Benefits received over the whole plan (average-return path).
    pub lifetime_benefits: Money,
    pub after_tax_legacy: Money,
    pub success_rate: f64,
    pub depleted_year: Option<i32>,
}

/// Claim ages 62 to 70 for everyone in the plan, one outcome each.
pub fn explore(
    base: &PlanInput,
    a: &Assumptions,
    picks: &Picks,
    runs: usize,
    seed: u64,
) -> Result<Vec<ClaimOutcome>, ParamError> {
    (62..=70)
        .map(|age| {
            let mut plan = base.clone();
            for b in &mut plan.social_security {
                b.claim_age = age;
            }
            let r = scenario::run_keep_claim_ages(&plan, a, picks, runs, seed)?;
            let by_year = |p: &crate::projection::SocialSecurity| {
                let person = &plan.people[p.person];
                let birth = plan.start_year
                    - person.age_on(
                        fin_core::NaiveDate::from_ymd_opt(plan.start_year, 12, 31).unwrap(),
                    );
                p.annual_benefit.mul_rate(claim_factor(birth, age * 12))
            };
            let fa = r.projection.final_accounts;
            Ok(ClaimOutcome {
                claim_age: age,
                annual_benefits: plan.social_security.iter().map(by_year).collect(),
                lifetime_benefits: r.projection.rows.iter().map(|y| y.social_security).sum(),
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

    #[test]
    fn full_retirement_age_follows_the_ssa_table() {
        assert_eq!(fra_months(1950), 66 * 12);
        assert_eq!(fra_months(1955), 66 * 12 + 2);
        assert_eq!(fra_months(1959), 66 * 12 + 10);
        assert_eq!(fra_months(1960), 67 * 12);
        assert_eq!(fra_months(1985), 67 * 12);
    }

    #[test]
    fn claiming_at_62_matches_the_ssa_reduction_table() {
        // (birth year, benefit at 62 out of $1,000 at full retirement age), from the SSA page.
        for (year, pct) in [
            (1950, 25.00),
            (1955, 25.83),
            (1956, 26.67),
            (1957, 27.50),
            (1958, 28.33),
            (1959, 29.17),
            (1960, 30.00),
        ] {
            let got = (1.0 - claim_factor(year, 62 * 12)) * 100.0;
            assert!((got - pct).abs() < 0.01, "{year}: {got} vs {pct}");
        }
    }

    #[test]
    fn delaying_adds_eight_percent_a_year_to_seventy_and_no_more() {
        assert_eq!(claim_factor(1970, 67 * 12), 1.0);
        assert!((claim_factor(1970, 68 * 12) - 1.08).abs() < 1e-9);
        assert!((claim_factor(1970, 70 * 12) - 1.24).abs() < 1e-9);
        assert_eq!(claim_factor(1970, 75 * 12), claim_factor(1970, 70 * 12));
        // Before 62 is treated as 62.
        assert_eq!(claim_factor(1970, 60 * 12), claim_factor(1970, 62 * 12));
        // Someone born in 1950 reaches full retirement age at 66, so 70 is 4 years of credits.
        assert!((claim_factor(1950, 70 * 12) - 1.32).abs() < 1e-9);
    }

    #[test]
    fn break_even_is_where_the_totals_cross() {
        let at62 = Money::from_dollars(28_000);
        let at70 = Money::from_dollars(49_600);
        let t = break_even_age(62, at62, 70, at70).unwrap();
        // 28,000 * (t - 62) == 49,600 * (t - 70)
        assert!((28_000.0 * (t - 62.0) - 49_600.0 * (t - 70.0)).abs() < 1.0);
        assert!(t > 78.0 && t < 84.0, "{t}");
        assert_eq!(break_even_age(70, at70, 62, at62), None);
    }

    #[test]
    fn the_plan_uses_the_claiming_adjustment() {
        let mut early = couple();
        early.social_security[0].claim_age = 62;
        let mut late = couple();
        late.social_security[0].claim_age = 70;
        let e = crate::projection::project_constant(&early, 0.04);
        let l = crate::projection::project_constant(&late, 0.04);
        let first_ss = |p: &crate::projection::Projection| {
            p.rows
                .iter()
                .find(|r| r.social_security > Money::ZERO)
                .unwrap()
                .social_security
        };
        // The fixture's person was born in 1976 (full retirement age 67), benefit 38,000 at 67.
        assert_eq!(first_ss(&e), Money::from_dollars(38_000).mul_rate(0.70));
        assert_eq!(first_ss(&l), Money::from_dollars(38_000).mul_rate(1.24));
    }

    #[test]
    fn explore_lists_each_age_with_rising_benefits() {
        let r = explore(&couple(), &Assumptions::defaults(), &Picks::new(), 80, 3).unwrap();
        assert_eq!(r.len(), 9);
        assert_eq!(r[0].claim_age, 62);
        assert_eq!(r[8].claim_age, 70);
        for w in r.windows(2) {
            assert!(w[1].annual_benefits[0] > w[0].annual_benefits[0]);
        }
        assert!(r.iter().all(|o| (0.0..=1.0).contains(&o.success_rate)));
        // The fixture's person is full-retirement-age 67 (born 1976): index 5 is age 67.
        assert_eq!(r[5].claim_age, 67);
        assert_eq!(r[5].annual_benefits[0], Money::from_dollars(38_000));
    }
}
