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
    /// Income on the return, income tax on it and the federal rate on the next dollar.
    pub agi_cents: i64,
    pub income_tax_cents: i64,
    pub marginal_rate: f64,
    /// Where the year's money came from: wages, Social Security and withdrawals (required
    /// distributions included), plus the Medicare premiums inside spending.
    pub earned_cents: i64,
    pub social_security_cents: i64,
    pub withdrawals_cents: i64,
    pub rmd_cents: i64,
    pub medicare_cents: i64,
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
    /// Dates worth knowing along the way, in order.
    pub milestones: Vec<crate::milestones::Milestone>,
    /// What heirs would receive from the accounts left at the end of the plan.
    pub legacy: crate::estate::LegacyView,
    /// Heads-ups, worst first.
    pub alerts: Vec<crate::alerts::Alert>,
    /// What each self-employed income could put into a Solo 401(k) or SEP IRA this year.
    pub self_employed_limits: Vec<SelfEmployedLimit>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SelfEmployedLimit {
    pub label: String,
    pub net_earnings_cents: i64,
    pub solo_401k_cents: i64,
    pub sep_ira_cents: i64,
    /// What the plan already counts as saved from this income.
    pub saving_cents: i64,
}

/// Contribution room for each self-employed income in the plan's first year.
pub fn self_employed_limits(plan: &PlanInput) -> Vec<SelfEmployedLimit> {
    let fed = crate::tax::FederalTaxYear::y2026();
    plan.streams
        .iter()
        .filter(|s| {
            s.self_employed && s.start_year <= plan.start_year && plan.start_year <= s.end_year
        })
        .map(|s| {
            let age = plan.people.get(s.person).map_or(40, |p| {
                plan.start_year
                    - p.birth_date
                        .format("%Y")
                        .to_string()
                        .parse::<i32>()
                        .unwrap_or(0)
            });
            let se = fed.self_employment_tax(plan.filing, s.annual_gross, fin_core::Money::ZERO);
            SelfEmployedLimit {
                label: s.label.clone(),
                net_earnings_cents: s.annual_gross.cents(),
                solo_401k_cents: crate::tax::federal::solo_401k_limit(
                    age,
                    s.annual_gross,
                    se.deduction,
                )
                .cents(),
                sep_ira_cents: crate::tax::federal::sep_ira_limit(s.annual_gross, se.deduction)
                    .cents(),
                saving_cents: s.traditional_contribution.cents(),
            }
        })
        .collect()
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
                self_employed: false,
                payroll_exempt: false,
                pension: false,
                public_pension: false,
                military_pension: false,
                employer_match: Money::ZERO,
                hsa: Money::ZERO,
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
        charity: Money::ZERO,
        traditional_split: vec![],
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

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BacktestWindow {
    pub start_year: i32,
    pub succeeded: bool,
    pub final_cents: i64,
    pub depleted_year: Option<i32>,
    pub annualised_return: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BacktestResult {
    pub stock_share: f64,
    pub years: usize,
    pub success_rate: f64,
    pub first_year: i32,
    pub last_year: i32,
    pub windows: Vec<BacktestWindow>,
    /// Start years of the three weakest windows, worst first (by money left, failures first).
    pub worst: Vec<i32>,
}

/// Replay the plan from every start year in US market history since 1928.
pub fn backtest(
    plan: &PlanInput,
    picks: &Picks,
    stock_share: f64,
) -> Result<BacktestResult, String> {
    let b = crate::backtest::run(plan, picks, stock_share)?;
    let windows: Vec<BacktestWindow> = b
        .windows
        .iter()
        .map(|w| BacktestWindow {
            start_year: w.start_year,
            succeeded: w.succeeded,
            final_cents: w.final_balance_cents,
            depleted_year: w.depleted_year,
            annualised_return: w.annualised_return,
        })
        .collect();
    let mut order: Vec<&BacktestWindow> = windows.iter().collect();
    order.sort_by_key(|w| (w.succeeded, w.final_cents, w.start_year));
    Ok(BacktestResult {
        stock_share: b.stock_share,
        years: b.years,
        success_rate: b.success_rate,
        first_year: b.first_year,
        last_year: b.last_year,
        worst: order.iter().take(3).map(|w| w.start_year).collect(),
        windows,
    })
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MixRow {
    pub stock_share: f64,
    pub success_rate: f64,
    /// Start year of the weakest window for this mix.
    pub worst_year: i32,
    /// Money left in the weakest window (zero when it ran out).
    pub worst_final_cents: i64,
    /// Median money left across every start year.
    pub median_final_cents: i64,
}

/// Replay history for several stock/bond mixes so they can be compared side by side.
pub fn compare_mixes(
    plan: &PlanInput,
    picks: &Picks,
    shares: &[f64],
) -> Result<Vec<MixRow>, String> {
    let mut out = Vec::new();
    for &share in shares.iter().take(8) {
        let b = backtest(plan, picks, share)?;
        let worst = b
            .windows
            .iter()
            .find(|w| Some(&w.start_year) == b.worst.first())
            .ok_or("No history windows to compare.")?;
        let mut finals: Vec<i64> = b.windows.iter().map(|w| w.final_cents).collect();
        finals.sort_unstable();
        out.push(MixRow {
            stock_share: share,
            success_rate: b.success_rate,
            worst_year: worst.start_year,
            worst_final_cents: worst.final_cents.max(0),
            median_final_cents: finals[finals.len() / 2],
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapResult {
    pub success_rate: f64,
    pub p10_cents: i64,
    pub p50_cents: i64,
    pub p90_cents: i64,
    pub runs: usize,
    pub block_years: usize,
    pub stock_share: f64,
}

/// Monte Carlo that redraws blocks of real history instead of assuming a bell curve.
pub fn bootstrap(
    plan: &PlanInput,
    picks: &Picks,
    stock_share: f64,
    block_years: usize,
    runs: usize,
) -> Result<BootstrapResult, String> {
    let (plan, _) =
        scenario::resolve(plan, &Assumptions::defaults(), picks).map_err(|e| e.to_string())?;
    let history: Vec<f64> = crate::backtest::history()
        .iter()
        .map(|y| crate::backtest::real_return(y, stock_share))
        .collect();
    let runs = runs.clamp(10, 5_000);
    let r = crate::montecarlo::run_bootstrap(&plan, &history, block_years, runs, SEED);
    Ok(BootstrapResult {
        success_rate: r.success_rate,
        p10_cents: r.p10.cents(),
        p50_cents: r.p50.cents(),
        p90_cents: r.p90.cents(),
        runs,
        block_years: block_years.clamp(1, history.len()),
        stock_share,
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
    let milestones = crate::milestones::find(plan, &r.projection, r.inputs.real_return);
    let legacy: crate::estate::LegacyView = (&crate::estate::settle(
        plan.filing,
        &r.projection.final_accounts,
        fin_core::Money::ZERO,
        crate::estate::DEFAULT_HEIR_RATE,
    ))
        .into();
    let alerts = crate::alerts::find(&crate::alerts::Facts {
        success_rate: r.monte_carlo.success_rate,
        depleted_year: r.projection.depleted_year,
        this_year: plan.start_year,
        milestones: &milestones,
        estate_tax_cents: legacy.estate_tax_cents,
    });
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
                    agi_cents: y.agi.cents(),
                    income_tax_cents: y.income_tax.cents(),
                    marginal_rate: y.marginal_rate,
                    earned_cents: y.earned.cents(),
                    social_security_cents: y.social_security.cents(),
                    withdrawals_cents: y.withdrawals.cents(),
                    rmd_cents: y.rmd.cents(),
                    medicare_cents: y.medicare.cents(),
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
        milestones,
        legacy,
        alerts,
        self_employed_limits: self_employed_limits(plan),
    })
}

#[cfg(test)]
mod backtest_tests {
    use super::*;
    use crate::projection::fixtures::couple;

    #[test]
    fn bootstrap_reports_a_success_rate_and_clamps_runs() {
        let r = bootstrap(&couple(), &Picks::new(), 0.6, 5, 1).unwrap();
        assert_eq!(r.runs, 10);
        assert!((0.0..=1.0).contains(&r.success_rate));
        let again = bootstrap(&couple(), &Picks::new(), 0.6, 5, 1).unwrap();
        assert_eq!(r, again);
    }

    #[test]
    fn mixes_compare_side_by_side() {
        let rows = compare_mixes(&couple(), &Picks::new(), &[0.4, 0.6, 1.0]).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1].stock_share, 0.6);
        let single = backtest(&couple(), &Picks::new(), 0.6).unwrap();
        assert_eq!(rows[1].success_rate, single.success_rate);
        assert_eq!(rows[1].worst_year, single.worst[0]);
        assert!(rows.iter().all(|r| r.worst_final_cents >= 0));
        assert!(compare_mixes(&couple(), &Picks::new(), &[])
            .unwrap()
            .is_empty());
    }

    #[test]
    fn backtest_reports_windows_and_the_worst_starts() {
        let r = backtest(&couple(), &Picks::new(), 0.6).unwrap();
        assert_eq!(r.windows.len(), 53);
        assert_eq!(r.worst.len(), 3);
        let v = serde_json::to_value(&r).unwrap();
        assert!(v["windows"][0]["startYear"].is_number() && v["successRate"].is_number());
        // The worst start is no better than the best.
        let worst = r
            .windows
            .iter()
            .find(|w| w.start_year == r.worst[0])
            .unwrap();
        assert!(r
            .windows
            .iter()
            .all(|w| (w.succeeded, w.final_cents) >= (worst.succeeded, worst.final_cents)));
    }
}

#[cfg(test)]
mod flow_tests {
    use super::*;
    use crate::projection::fixtures::couple;

    #[test]
    fn each_year_reports_where_the_money_came_from() {
        let r = run_plan(&couple(), &Picks::new(), 20).unwrap();
        let first = &r.years[0];
        assert!(first.earned_cents > 0);
        assert_eq!(first.social_security_cents, 0);
        let last = r.years.last().unwrap();
        assert_eq!(last.earned_cents, 0);
        assert!(last.social_security_cents > 0);
        let v = serde_json::to_value(first).unwrap();
        assert!(v["earnedCents"].is_number() && v["withdrawalsCents"].is_number());
    }

    #[test]
    fn money_in_covers_spending_and_tax_while_the_plan_holds() {
        let r = run_plan(&couple(), &Picks::new(), 20).unwrap();
        for y in r
            .years
            .iter()
            .filter(|y| r.depleted_year.is_none() && y.balance_cents > 0)
        {
            let inflow =
                y.earned_cents + y.social_security_cents + y.rmd_cents + y.withdrawals_cents;
            assert!(
                inflow + 100 >= y.spending_cents,
                "{} {} {}",
                y.year,
                inflow,
                y.spending_cents
            );
        }
    }
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

#[cfg(test)]
mod legacy_tests {
    use super::*;
    use crate::projection::fixtures::couple;

    #[test]
    fn the_result_carries_what_is_left_for_heirs() {
        let r = run_plan(&couple(), &Picks::new(), 10).unwrap();
        let l = &r.legacy;
        assert!(l.gross_cents >= 0);
        assert!(l.net_to_heirs_cents <= l.gross_cents);
        assert_eq!(
            l.net_to_heirs_cents,
            l.gross_cents - l.estate_tax_cents - l.heir_income_tax_cents
        );
        assert!(l.exemption_cents >= 1_500_000_000);
    }
}

#[cfg(test)]
mod limit_tests {
    use super::*;
    use crate::projection::fixtures::couple;

    #[test]
    fn self_employed_income_shows_contribution_room() {
        let mut plan = couple();
        assert!(self_employed_limits(&plan).is_empty());
        plan.streams[0].self_employed = true;
        plan.streams[0].annual_gross = Money::from_dollars(100_000);
        let l = self_employed_limits(&plan);
        assert_eq!(l.len(), 1);
        // Employee deferral plus a fifth of earnings after half the SE tax.
        assert!(l[0].solo_401k_cents > l[0].sep_ira_cents);
        assert!(l[0].sep_ira_cents > 0 && l[0].sep_ira_cents < 2_000_000);
        assert_eq!(l[0].net_earnings_cents, 10_000_000);
    }
}
