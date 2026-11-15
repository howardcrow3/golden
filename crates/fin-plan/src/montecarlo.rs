//! Monte Carlo over the real-return path. Seeded ChaCha so a given seed reproduces the same
//! result on every platform.
use crate::projection::{project, PlanInput};
use fin_core::Money;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, Normal};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReturnModel {
    /// Mean real annual return, e.g. 0.04.
    pub mean: f64,
    /// Standard deviation of annual real return, e.g. 0.12.
    pub stdev: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonteCarloResult {
    pub runs: usize,
    pub success_rate: f64,
    /// Final balance percentiles across all runs (failed runs end at zero).
    pub p10: Money,
    pub p50: Money,
    pub p90: Money,
    /// Median year of depletion among failed runs.
    pub median_depletion_year: Option<i32>,
}

pub fn run(input: &PlanInput, model: ReturnModel, runs: usize, seed: u64) -> MonteCarloResult {
    assert!(runs > 0);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let normal = Normal::new(model.mean, model.stdev.max(0.0)).expect("valid normal");
    let years = (input.horizon_age
        - input.people[0]
            .age_on(fin_core::NaiveDate::from_ymd_opt(input.start_year, 12, 31).unwrap()))
    .max(0) as usize
        + 1;

    let mut finals = Vec::with_capacity(runs);
    let mut depletion_years = vec![];
    let mut ok = 0usize;
    for _ in 0..runs {
        let path: Vec<f64> = (0..years)
            .map(|_| normal.sample(&mut rng).max(-0.95))
            .collect();
        let p = project(input, &|i| path[i.min(years - 1)]);
        if p.succeeded() {
            ok += 1;
        } else if let Some(y) = p.depleted_year {
            depletion_years.push(y);
        }
        finals.push(p.final_balance());
    }
    finals.sort();
    depletion_years.sort();
    let pct = |q: f64| finals[((finals.len() - 1) as f64 * q).round() as usize];
    MonteCarloResult {
        runs,
        success_rate: ok as f64 / runs as f64,
        p10: pct(0.10),
        p50: pct(0.50),
        p90: pct(0.90),
        median_depletion_year: depletion_years
            .get(depletion_years.len() / 2)
            .copied()
            .filter(|_| !depletion_years.is_empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::fixtures::couple;

    const M: ReturnModel = ReturnModel {
        mean: 0.04,
        stdev: 0.12,
    };

    #[test]
    fn same_seed_same_answer() {
        let a = run(&couple(), M, 200, 42);
        let b = run(&couple(), M, 200, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_differ_but_agree_roughly() {
        let a = run(&couple(), M, 400, 1);
        let b = run(&couple(), M, 400, 2);
        assert_ne!(a.p50, b.p50);
        assert!((a.success_rate - b.success_rate).abs() < 0.1);
    }

    #[test]
    fn zero_volatility_matches_the_deterministic_projection() {
        let r = run(
            &couple(),
            ReturnModel {
                mean: 0.04,
                stdev: 0.0,
            },
            5,
            7,
        );
        let det = crate::projection::project_constant(&couple(), 0.04);
        assert_eq!(r.success_rate, if det.succeeded() { 1.0 } else { 0.0 });
        assert_eq!(r.p50, det.final_balance());
        assert_eq!(r.p10, r.p90);
    }

    #[test]
    fn percentiles_are_ordered() {
        let r = run(&couple(), M, 300, 9);
        assert!(r.p10 <= r.p50 && r.p50 <= r.p90);
        assert!((0.0..=1.0).contains(&r.success_rate));
    }

    #[test]
    fn riskier_markets_lower_success() {
        let calm = run(
            &couple(),
            ReturnModel {
                mean: 0.03,
                stdev: 0.05,
            },
            300,
            3,
        );
        let wild = run(
            &couple(),
            ReturnModel {
                mean: 0.03,
                stdev: 0.25,
            },
            300,
            3,
        );
        assert!(
            wild.success_rate <= calm.success_rate + 0.02,
            "{} vs {}",
            wild.success_rate,
            calm.success_rate
        );
    }

    #[test]
    fn failing_plan_reports_a_depletion_year() {
        let mut i = couple();
        i.annual_spending = Money::from_dollars(300_000);
        let r = run(&i, M, 100, 5);
        assert!(r.success_rate < 0.05);
        assert!(r.median_depletion_year.is_some());
    }
}
