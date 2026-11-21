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
    };
    run_plan(&plan, &req.picks, req.runs)
}

/// Run any household plan (any number of people and income streams) with what-if picks.
pub fn run_plan(plan: &PlanInput, picks: &Picks, runs: usize) -> Result<QuickPlanResult, String> {
    let r = scenario::run(
        plan,
        &Assumptions::defaults(),
        picks,
        runs.clamp(10, 5_000),
        20_260_101,
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
            .map(|y| YearPoint {
                year: y.year,
                age: y.age,
                balance_cents: y.end_balance.cents(),
                spending_cents: y.spending.cents(),
                taxes_cents: y.taxes.cents(),
            })
            .collect(),
        real_return: r.inputs.real_return,
        volatility: r.inputs.volatility,
        horizon_age: r.inputs.horizon_age,
        ss_claim_age: r.inputs.ss_claim_age,
    })
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
