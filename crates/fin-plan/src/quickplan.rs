//! A single-call retirement check for the app's Retirement screen: JSON in, JSON out.
use crate::params::{Assumptions, Picks};
use crate::projection::{Accounts, IncomeStream, PlanInput, SocialSecurity};
use crate::scenario;
use crate::tax::FilingStatus;
use fin_core::{Money, NaiveDate, Person};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickPlanRequest {
    pub current_year: i32,
    pub age: i32,
    pub retire_age: i32,
    pub filing: FilingStatus,
    pub state: String,
    pub salary_cents: i64,
    pub contribution_cents: i64,
    pub spending_cents: i64,
    pub cash_cents: i64,
    pub taxable_cents: i64,
    pub traditional_cents: i64,
    pub roth_cents: i64,
    pub ss_annual_cents: i64,
    /// What-if picks: parameter key -> option label.
    #[serde(default)]
    pub picks: BTreeMap<String, String>,
    #[serde(default = "default_runs")]
    pub runs: usize,
}

fn default_runs() -> usize {
    500
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YearPoint {
    pub year: i32,
    pub age: i32,
    pub balance_cents: i64,
    pub spending_cents: i64,
    pub taxes_cents: i64,
    /// Balance percentiles across the simulated runs, for the fan chart.
    pub p10_cents: i64,
    pub p50_cents: i64,
    pub p90_cents: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistogramBin {
    pub from_cents: i64,
    pub to_cents: i64,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuickPlanResult {
    pub success_rate: f64,
    pub depleted_year: Option<i32>,
    pub p10_cents: i64,
    pub p50_cents: i64,
    pub p90_cents: i64,
    pub years: Vec<YearPoint>,
    /// Runs by final balance, in ten equal steps from zero to the largest.
    pub histogram: Vec<HistogramBin>,
    /// How many simulated futures and the seed they came from: the same two give the same answer.
    pub runs: usize,
    pub seed: u64,
    pub real_return: f64,
    pub volatility: f64,
    pub horizon_age: i32,
    pub ss_claim_age: i32,
}

pub fn run(req: &QuickPlanRequest) -> Result<QuickPlanResult, String> {
    if req.retire_age < req.age {
        return Err("Retirement age must be at or after your current age.".into());
    }
    let birth = NaiveDate::from_ymd_opt(req.current_year - req.age, 6, 15).ok_or("bad age")?;
    let person = Person {
        id: "you".into(),
        name: "You".into(),
        birth_date: birth,
        is_dependent: false,
    };
    let working_years = req.retire_age - req.age;
    let plan = PlanInput {
        start_year: req.current_year,
        people: vec![person],
        filing: req.filing,
        state: req.state.to_ascii_uppercase(),
        accounts: Accounts {
            cash: Money::from_cents(req.cash_cents),
            taxable: Money::from_cents(req.taxable_cents),
            taxable_gain_share: 0.3,
            traditional: Money::from_cents(req.traditional_cents),
            roth: Money::from_cents(req.roth_cents),
        },
        streams: if working_years > 0 && req.salary_cents > 0 {
            vec![IncomeStream {
                label: "Salary".into(),
                person: 0,
                annual_gross: Money::from_cents(req.salary_cents),
                start_year: req.current_year,
                end_year: req.current_year + working_years - 1,
                traditional_contribution: Money::from_cents(
                    req.contribution_cents.min(req.salary_cents),
                ),
            }]
        } else {
            vec![]
        },
        social_security: if req.ss_annual_cents > 0 {
            vec![SocialSecurity {
                person: 0,
                annual_benefit: Money::from_cents(req.ss_annual_cents),
                claim_age: 67,
            }]
        } else {
            vec![]
        },
        annual_spending: Money::from_cents(req.spending_cents),
        one_time_costs: vec![],
        horizon_age: 95,
        spending_rule: Default::default(),
        withdrawal_order: Default::default(),
        roth_conversion: None,
        medicare: false,
    };
    run_plan(&plan, &req.picks, req.runs)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SafeSpendResult {
    pub target: f64,
    pub spending_cents: i64,
    pub current_cents: i64,
    pub nothing_safe: bool,
    pub capped: bool,
}

/// The most the household can spend a year (today's dollars) while the plan still works in
/// `target` of the trials.
pub fn safe_spend(plan: &PlanInput, picks: &Picks, target: f64) -> Result<SafeSpendResult, String> {
    let target = target.clamp(0.5, 0.99);
    let r = crate::safespend::safe_spend(
        plan,
        &Assumptions::defaults(),
        picks,
        target,
        300,
        20_260_101,
    )
    .map_err(|e| e.to_string())?;
    Ok(SafeSpendResult {
        target: r.target,
        spending_cents: r.spending.cents(),
        current_cents: r.current.cents(),
        nothing_safe: r.nothing_safe,
        capped: r.capped,
    })
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RothRow {
    pub label: String,
    pub converted_cents: i64,
    pub lifetime_tax_cents: i64,
    pub final_balance_cents: i64,
    pub legacy_cents: i64,
    pub success_rate: f64,
    pub depleted_year: Option<i32>,
}

/// Compare "no conversions" with filling the 12%, 22% and 24% brackets until `until_age`.
pub fn roth_explore(
    plan: &PlanInput,
    picks: &Picks,
    until_age: i32,
) -> Result<Vec<RothRow>, String> {
    let until_age = until_age.clamp(50, 90);
    let options = crate::roth::standard_options(plan, until_age);
    let out = crate::roth::explore(
        plan,
        &Assumptions::defaults(),
        picks,
        &options,
        200,
        20_260_101,
    )
    .map_err(|e| e.to_string())?;
    Ok(out
        .into_iter()
        .map(|o| RothRow {
            label: o.label,
            converted_cents: o.converted_total.cents(),
            lifetime_tax_cents: o.lifetime_tax.cents(),
            final_balance_cents: o.final_balance.cents(),
            legacy_cents: o.after_tax_legacy.cents(),
            success_rate: o.success_rate,
            depleted_year: o.depleted_year,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SsRow {
    pub claim_age: i32,
    /// Yearly benefit per person at this claiming age.
    pub annual_cents: Vec<i64>,
    pub lifetime_cents: i64,
    pub legacy_cents: i64,
    pub success_rate: f64,
    pub depleted_year: Option<i32>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BreakEven {
    pub earlier: i32,
    pub later: i32,
    /// Age at which the later claim has paid out as much in total, for the first person.
    pub age: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SsExplore {
    pub rows: Vec<SsRow>,
    pub break_evens: Vec<BreakEven>,
}

/// Compare claiming Social Security at every age from 62 to 70.
pub fn ss_explore(plan: &PlanInput, picks: &Picks) -> Result<SsExplore, String> {
    if plan.social_security.is_empty() {
        return Err("Add a Social Security amount for someone first.".into());
    }
    let out = crate::ss::explore(plan, &Assumptions::defaults(), picks, 200, 20_260_101)
        .map_err(|e| e.to_string())?;
    let first = |age: i32| {
        out.iter()
            .find(|o| o.claim_age == age)
            .map(|o| o.annual_benefits[0])
    };
    let break_evens = [(62, 67), (67, 70), (62, 70)]
        .into_iter()
        .map(|(e, l)| BreakEven {
            earlier: e,
            later: l,
            age: match (first(e), first(l)) {
                (Some(a), Some(b)) => crate::ss::break_even_age(e, a, l, b),
                _ => None,
            },
        })
        .collect();
    Ok(SsExplore {
        rows: out
            .into_iter()
            .map(|o| SsRow {
                claim_age: o.claim_age,
                annual_cents: o.annual_benefits.iter().map(|m| m.cents()).collect(),
                lifetime_cents: o.lifetime_benefits.cents(),
                legacy_cents: o.after_tax_legacy.cents(),
                success_rate: o.success_rate,
                depleted_year: o.depleted_year,
            })
            .collect(),
        break_evens,
    })
}

const SEED: u64 = 20_260_101;

/// Run any household plan (any number of people and income streams) with what-if picks.
pub fn run_plan(plan: &PlanInput, picks: &Picks, runs: usize) -> Result<QuickPlanResult, String> {
    let r = scenario::run(
        plan,
        &Assumptions::defaults(),
        picks,
        runs.clamp(10, 5_000),
        SEED,
    )
    .map_err(|e| e.to_string())?;
    Ok(QuickPlanResult {
        success_rate: r.monte_carlo.success_rate,
        depleted_year: r.projection.depleted_year,
        p10_cents: r.monte_carlo.p10.cents(),
        p50_cents: r.monte_carlo.p50.cents(),
        p90_cents: r.monte_carlo.p90.cents(),
        years: r
            .projection
            .rows
            .iter()
            .enumerate()
            .map(|(i, y)| {
                let band = r.monte_carlo.bands.get(i);
                YearPoint {
                    year: y.year,
                    age: y.age,
                    balance_cents: y.end_balance.cents(),
                    spending_cents: y.spending.cents(),
                    taxes_cents: y.taxes.cents(),
                    p10_cents: band.map_or(y.end_balance.cents(), |b| b.p10.cents()),
                    p50_cents: band.map_or(y.end_balance.cents(), |b| b.p50.cents()),
                    p90_cents: band.map_or(y.end_balance.cents(), |b| b.p90.cents()),
                }
            })
            .collect(),
        histogram: r
            .monte_carlo
            .histogram
            .iter()
            .map(|h| HistogramBin {
                from_cents: h.from.cents(),
                to_cents: h.to.cents(),
                count: h.count,
            })
            .collect(),
        runs: r.monte_carlo.runs,
        seed: SEED,
        real_return: r.inputs.real_return,
        volatility: r.inputs.volatility,
        horizon_age: r.inputs.horizon_age,
        ss_claim_age: r.inputs.ss_claim_age,
    })
}

#[cfg(test)]
mod ss_tests {
    use super::*;
    use crate::projection::fixtures::couple;

    #[test]
    fn explore_returns_nine_ages_and_break_evens() {
        let r = ss_explore(&couple(), &Picks::new()).unwrap();
        assert_eq!(r.rows.len(), 9);
        assert_eq!(r.break_evens.len(), 3);
        let b = r
            .break_evens
            .iter()
            .find(|b| (b.earlier, b.later) == (62, 70))
            .unwrap();
        let age = b.age.unwrap();
        assert!(age > 78.0 && age < 84.0, "{age}");
        let v = serde_json::to_value(&r).unwrap();
        assert!(v["rows"][0]["claimAge"].is_number());
    }

    #[test]
    fn nobody_with_a_benefit_is_a_clear_error() {
        let mut p = couple();
        p.social_security.clear();
        assert!(ss_explore(&p, &Picks::new())
            .unwrap_err()
            .contains("Social Security"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> QuickPlanRequest {
        serde_json::from_str(
            r#"{"currentYear":2026,"age":45,"retireAge":62,"filing":"married_joint","state":"tx",
                "salaryCents":18000000,"contributionCents":2350000,"spendingCents":9000000,
                "cashCents":3000000,"taxableCents":15000000,"traditionalCents":60000000,"rothCents":12000000,
                "ssAnnualCents":3800000,"runs":100}"#,
        )
        .unwrap()
    }

    #[test]
    fn json_request_runs_and_serializes() {
        let r = run(&req()).unwrap();
        assert!(r.success_rate > 0.0);
        assert_eq!(r.years.first().unwrap().year, 2026);
        assert_eq!(r.horizon_age, 95);
        let v = serde_json::to_value(&r).unwrap();
        assert!(v["successRate"].is_number());
        assert!(v["years"][0]["balanceCents"].is_number());
    }

    #[test]
    fn retiring_before_now_is_an_error() {
        let mut q = req();
        q.retire_age = 40;
        assert!(run(&q).unwrap_err().contains("Retirement age"));
    }

    #[test]
    fn retiring_later_helps() {
        let mut early = req();
        early.retire_age = 55;
        let mut late = req();
        late.retire_age = 67;
        assert!(run(&late).unwrap().success_rate >= run(&early).unwrap().success_rate);
        assert!(run(&late).unwrap().p50_cents >= run(&early).unwrap().p50_cents);
    }

    #[test]
    fn what_if_pick_changes_resolved_inputs() {
        let mut q = req();
        q.picks.insert("real_return".into(), "Cautious".into());
        let r = run(&q).unwrap();
        assert_eq!(r.real_return, 0.02);
        assert!(r.success_rate <= run(&req()).unwrap().success_rate);
    }

    #[test]
    fn already_retired_household_has_no_salary() {
        let mut q = req();
        q.age = 67;
        q.retire_age = 67;
        let r = run(&q).unwrap();
        assert!(r.years.iter().all(|y| y.age >= 67));
    }

    #[test]
    fn same_request_same_answer() {
        assert_eq!(run(&req()).unwrap(), run(&req()).unwrap());
    }

    #[test]
    fn unknown_pick_surfaces_a_message() {
        let mut q = req();
        q.picks.insert("real_return".into(), "Moonshot".into());
        assert!(run(&q).unwrap_err().contains("Moonshot"));
    }
}
