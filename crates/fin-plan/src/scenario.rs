//! Glue between assumptions with options, what-if picks and the projection engine.
use crate::montecarlo::{self, MonteCarloResult, ReturnModel};
use crate::params::{Assumptions, ParamError, Picks};
use crate::projection::{project_constant, PlanInput, Projection};
use fin_core::Money;

#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub projection: Projection,
    pub monte_carlo: MonteCarloResult,
    pub inputs: ResolvedInputs,
}

/// The concrete numbers a run used, shown beside every result so nothing is hidden.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedInputs {
    pub real_return: f64,
    pub volatility: f64,
    pub horizon_age: i32,
    pub ss_claim_age: i32,
    pub spending_change: f64,
}

/// Apply `assumptions` (plus what-if `picks`) to `base` and return the resolved plan.
pub fn resolve(
    base: &PlanInput,
    a: &Assumptions,
    picks: &Picks,
) -> Result<(PlanInput, ResolvedInputs), ParamError> {
    resolve_with(base, a, picks, true)
}

/// Like `resolve`, but `set_claim_age` false keeps each benefit's own claiming age.
pub fn resolve_with(
    base: &PlanInput,
    a: &Assumptions,
    picks: &Picks,
    set_claim_age: bool,
) -> Result<(PlanInput, ResolvedInputs), ParamError> {
    let r = ResolvedInputs {
        real_return: a.value("real_return", picks)?,
        volatility: a.value("return_volatility", picks)?,
        horizon_age: a.value("horizon_age", picks)?.round() as i32,
        ss_claim_age: a.value("ss_claim_age", picks)?.round() as i32,
        spending_change: a.value("spending_change", picks)?,
    };
    let mut plan = base.clone();
    plan.horizon_age = r.horizon_age;
    if set_claim_age {
        for b in &mut plan.social_security {
            b.claim_age = r.ss_claim_age;
        }
    }
    // Real spending drifts by `spending_change` a year after the first year; the engine takes one
    // flat figure, so use the average over the horizon.
    let age_now = plan.people.first().map_or(50, |p| {
        p.age_on(fin_core::NaiveDate::from_ymd_opt(plan.start_year, 12, 31).unwrap())
    });
    let years = (r.horizon_age - age_now).max(1) as f64;
    let avg = (1.0 + r.spending_change).powf(years / 2.0);
    plan.annual_spending = plan.annual_spending.mul_rate(avg);
    Ok((plan, r))
}

pub fn run(
    base: &PlanInput,
    a: &Assumptions,
    picks: &Picks,
    runs: usize,
    seed: u64,
) -> Result<ScenarioResult, ParamError> {
    run_inner(base, a, picks, runs, seed, true)
}

/// Run with the claiming ages already set on the plan (the Social Security explorer).
pub fn run_keep_claim_ages(
    base: &PlanInput,
    a: &Assumptions,
    picks: &Picks,
    runs: usize,
    seed: u64,
) -> Result<ScenarioResult, ParamError> {
    run_inner(base, a, picks, runs, seed, false)
}

fn run_inner(
    base: &PlanInput,
    a: &Assumptions,
    picks: &Picks,
    runs: usize,
    seed: u64,
    set_claim_age: bool,
) -> Result<ScenarioResult, ParamError> {
    let (plan, inputs) = resolve_with(base, a, picks, set_claim_age)?;
    let projection = project_constant(&plan, inputs.real_return);
    let monte_carlo = montecarlo::run(
        &plan,
        ReturnModel {
            mean: inputs.real_return,
            stdev: inputs.volatility,
        },
        runs,
        seed,
    );
    Ok(ScenarioResult {
        projection,
        monte_carlo,
        inputs,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensitivityRow {
    pub key: String,
    pub low_label: String,
    pub high_label: String,
    pub low_final: Money,
    pub high_final: Money,
}

impl SensitivityRow {
    pub fn swing(&self) -> Money {
        (self.high_final - self.low_final).abs()
    }
}

/// For each parameter, run the deterministic projection at its lowest and highest option and
/// report the swing in ending balance, biggest first ("what moves the answer most").
pub fn sensitivity(base: &PlanInput, a: &Assumptions) -> Result<Vec<SensitivityRow>, ParamError> {
    let mut rows = vec![];
    let keys: Vec<String> = a.keys().map(String::from).collect();
    for key in keys {
        let p = a.get(&key)?;
        if p.options.len() < 2 {
            continue;
        }
        let low = p
            .options
            .iter()
            .min_by(|x, y| x.value.total_cmp(&y.value))
            .unwrap();
        let high = p
            .options
            .iter()
            .max_by(|x, y| x.value.total_cmp(&y.value))
            .unwrap();
        let fin = |label: &str| -> Result<Money, ParamError> {
            let mut picks = Picks::new();
            picks.insert(key.clone(), label.to_string());
            let (plan, r) = resolve(base, a, &picks)?;
            Ok(project_constant(&plan, r.real_return).final_balance())
        };
        rows.push(SensitivityRow {
            key: key.clone(),
            low_label: low.label.clone(),
            high_label: high.label.clone(),
            low_final: fin(&low.label)?,
            high_final: fin(&high.label)?,
        });
    }
    rows.sort_by(|x, y| y.swing().cmp(&x.swing()).then(x.key.cmp(&y.key)));
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::fixtures::couple;

    #[test]
    fn resolves_defaults() {
        let (plan, r) = resolve(&couple(), &Assumptions::defaults(), &Picks::new()).unwrap();
        assert_eq!(r.real_return, 0.04);
        assert_eq!(plan.horizon_age, 95);
        assert_eq!(plan.social_security[0].claim_age, 67);
        assert_eq!(plan.annual_spending, couple().annual_spending);
    }

    #[test]
    fn picks_change_the_resolved_plan_only() {
        let a = Assumptions::defaults();
        let mut picks = Picks::new();
        picks.insert("ss_claim_age".into(), "Age 70".into());
        picks.insert("real_return".into(), "Cautious".into());
        let (plan, r) = resolve(&couple(), &a, &picks).unwrap();
        assert_eq!(plan.social_security[0].claim_age, 70);
        assert_eq!(r.real_return, 0.02);
        assert_eq!(a.get("ss_claim_age").unwrap().value(), 67.0);
    }

    #[test]
    fn what_if_return_moves_the_outcome() {
        let a = Assumptions::defaults();
        let mut cautious = Picks::new();
        cautious.insert("real_return".into(), "Cautious".into());
        let mut optimistic = Picks::new();
        optimistic.insert("real_return".into(), "Optimistic".into());
        let lo = run(&couple(), &a, &cautious, 100, 1).unwrap();
        let hi = run(&couple(), &a, &optimistic, 100, 1).unwrap();
        assert!(hi.projection.final_balance() > lo.projection.final_balance());
        assert!(hi.monte_carlo.success_rate >= lo.monte_carlo.success_rate);
    }

    #[test]
    fn sensitivity_ranks_by_swing() {
        let rows = sensitivity(&couple(), &Assumptions::defaults()).unwrap();
        assert_eq!(rows.len(), 5);
        assert!(rows.windows(2).all(|w| w[0].swing() >= w[1].swing()));
        let ret = rows.iter().find(|r| r.key == "real_return").unwrap();
        assert!(ret.high_final > ret.low_final);
        assert_eq!(rows[0].key, "real_return"); // return assumptions dominate a long horizon
    }

    #[test]
    fn spending_drift_averages_over_the_years_actually_left() {
        // Primary is 50 at the end of 2026 in the fixture; make them 70. Horizon 95 leaves 25
        // years of drift after the first, so the average sits 12.5 years in.
        let mut base = couple();
        base.people[0].birth_date = fin_core::NaiveDate::from_ymd_opt(1956, 6, 15).unwrap();
        let mut picks = Picks::new();
        picks.insert("spending_change".into(), "Declines".into());
        let (plan, _) = resolve(&base, &Assumptions::defaults(), &picks).unwrap();
        assert_eq!(
            plan.annual_spending,
            base.annual_spending.mul_rate(0.99f64.powf(12.5))
        );
    }

    #[test]
    fn unknown_pick_is_an_error_not_a_panic() {
        let mut picks = Picks::new();
        picks.insert("real_return".into(), "Moonshot".into());
        assert!(run(&couple(), &Assumptions::defaults(), &picks, 10, 1).is_err());
    }
}
