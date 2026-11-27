//! Integration: a household plan through the projection engine, tax engine and Monte Carlo.
use fin_core::{Money, NaiveDate, Person};
use fin_plan::montecarlo::{run, ReturnModel};
use fin_plan::projection::{project_constant, Accounts, IncomeStream, PlanInput, SocialSecurity};
use fin_plan::tax::FilingStatus;

fn household(spending: i64, state: &str) -> PlanInput {
    let person = |n: &str, y: i32| Person {
        id: n.into(),
        name: n.into(),
        birth_date: NaiveDate::from_ymd_opt(y, 3, 1).unwrap(),
        is_dependent: false,
    };
    PlanInput {
        start_year: 2026,
        people: vec![person("Pat", 1970), person("Sam", 1972)],
        filing: FilingStatus::MarriedJoint,
        state: state.into(),
        accounts: Accounts {
            cash: Money::from_dollars(40_000),
            taxable: Money::from_dollars(300_000),
            taxable_gain_share: 0.4,
            traditional: Money::from_dollars(900_000),
            roth: Money::from_dollars(200_000),
        },
        streams: vec![
            IncomeStream {
                label: "Pat salary".into(),
                person: 0,
                annual_gross: Money::from_dollars(160_000),
                start_year: 2026,
                end_year: 2034,
                traditional_contribution: Money::from_dollars(23_500),
                self_employed: false,
                payroll_exempt: false,
                pension: false,
                employer_match: Money::ZERO,
                hsa: Money::ZERO,
            },
            IncomeStream {
                label: "Sam consulting".into(),
                person: 1,
                annual_gross: Money::from_dollars(60_000),
                start_year: 2026,
                end_year: 2036,
                traditional_contribution: Money::ZERO,
                self_employed: false,
                payroll_exempt: false,
                pension: false,
                employer_match: Money::ZERO,
                hsa: Money::ZERO,
            },
        ],
        social_security: vec![
            SocialSecurity {
                person: 0,
                annual_benefit: Money::from_dollars(40_000),
                claim_age: 67,
            },
            SocialSecurity {
                person: 1,
                annual_benefit: Money::from_dollars(24_000),
                claim_age: 67,
            },
        ],
        annual_spending: Money::from_dollars(spending),
        one_time_costs: vec![],
        horizon_age: 95,
        spending_rule: Default::default(),
        withdrawal_order: Default::default(),
        roth_conversion: None,
        medicare: false,
        charity: Money::ZERO,
    }
}

#[test]
fn two_income_streams_both_stop_on_their_own_dates() {
    let p = project_constant(&household(120_000, "TX"), 0.04);
    let y2034 = p.rows.iter().find(|r| r.year == 2034).unwrap().earned;
    let y2035 = p.rows.iter().find(|r| r.year == 2035).unwrap().earned;
    let y2037 = p.rows.iter().find(|r| r.year == 2037).unwrap().earned;
    assert_eq!(y2034, Money::from_dollars(220_000));
    assert_eq!(y2035, Money::from_dollars(60_000));
    assert_eq!(y2037, Money::ZERO);
}

#[test]
fn each_spouse_claims_social_security_at_their_own_age() {
    let p = project_constant(&household(120_000, "TX"), 0.04);
    let by_year = |y: i32| p.rows.iter().find(|r| r.year == y).unwrap().social_security;
    assert_eq!(by_year(2036), Money::ZERO);
    // Everyone is born March 1, so the year of claiming covers April to December (9 of 12).
    assert_eq!(by_year(2037), Money::from_dollars(30_000)); // Pat turns 67 in 2037
    assert_eq!(by_year(2038), Money::from_dollars(40_000));
    assert_eq!(by_year(2039), Money::from_dollars(58_000)); // Sam turns 67 in 2039
    assert_eq!(by_year(2040), Money::from_dollars(64_000));
}

#[test]
fn moving_to_a_no_tax_state_improves_the_outcome() {
    let ca = project_constant(&household(150_000, "CA"), 0.035).final_balance();
    let tx = project_constant(&household(150_000, "TX"), 0.035).final_balance();
    assert!(tx > ca, "tx {tx} ca {ca}");
}

#[test]
fn what_if_spending_cut_raises_success_probability() {
    let model = ReturnModel {
        mean: 0.035,
        stdev: 0.13,
    };
    let high = run(&household(190_000, "TX"), model, 300, 11);
    let low = run(&household(130_000, "TX"), model, 300, 11);
    assert!(low.success_rate >= high.success_rate);
    assert!(low.p50 >= high.p50);
}

#[test]
fn taxes_are_positive_every_working_year_and_balance_never_negative() {
    let p = project_constant(&household(120_000, "NY"), 0.04);
    for r in &p.rows {
        assert!(r.end_balance >= Money::ZERO);
        if r.earned > Money::ZERO {
            assert!(r.taxes > Money::ZERO);
        }
    }
}
