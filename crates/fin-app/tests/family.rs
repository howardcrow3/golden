use fin_app::household::{ChildRow, Household};
use fin_core::NaiveDate;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}

fn kid(birth: i32, care: i64, college: i64) -> ChildRow {
    ChildRow {
        name: "Sam".into(),
        birth_year: birth,
        childcare_cents: care,
        college_cents: college,
    }
}

#[test]
fn no_children_adds_nothing() {
    let p = Household::sample().plan_input(today()).unwrap();
    assert!(p.one_time_costs.is_empty());
}

#[test]
fn a_child_adds_childcare_college_and_a_credit() {
    let mut h = Household::sample();
    h.children = vec![kid(2024, 1_000_000, 3_000_000)];
    let p = h.plan_input(today()).unwrap();
    let at = |y: i32| {
        p.one_time_costs
            .iter()
            .find(|c| c.year == y)
            .map(|c| c.amount.cents())
    };
    // Age 2: childcare less the credit (sample income is under the phase-out).
    assert_eq!(at(2026), Some(1_000_000 - 220_000));
    // Age 18: college.
    assert_eq!(at(2042), Some(3_000_000));
    // Age 25: nothing.
    assert_eq!(at(2049), None);
}

#[test]
fn bad_children_are_readable_errors() {
    let mut h = Household::sample();
    h.children = vec![kid(2024, -1, 0)];
    assert!(h.plan_input(today()).unwrap_err().contains("negative"));
    h.children = vec![kid(24, 0, 0)];
    assert!(h.plan_input(today()).unwrap_err().contains("birth year"));
}

#[test]
fn old_saved_households_without_children_still_load() {
    let h = Household::sample();
    let mut v = serde_json::to_value(&h).unwrap();
    v.as_object_mut().unwrap().remove("children");
    let back: Household = serde_json::from_value(v).unwrap();
    assert!(back.children.is_empty());
}

#[test]
fn a_city_tax_is_added_to_income_tax_while_earning() {
    use fin_plan::projection::project_constant;
    let mut h = Household::sample();
    h.state = "PA".into();
    let without = project_constant(&h.plan_input(today()).unwrap(), 0.04);
    h.locality = Some("philadelphia".into());
    let p = h.plan_input(today()).unwrap();
    assert_eq!(p.locality.as_deref(), Some("philadelphia"));
    let with = project_constant(&p, 0.04);
    // Philadelphia's 3.735% wage tax on the first year's wages.
    let wages = with.rows[0].earned;
    let extra = with.rows[0].income_tax - without.rows[0].income_tax;
    assert!(wages > fin_core::Money::ZERO);
    assert!(
        (extra.cents() - wages.mul_rate(0.03735).cents()).abs() <= 100,
        "{extra:?}"
    );
}

#[test]
fn a_city_tax_must_match_the_state_and_be_known() {
    let mut h = Household::sample();
    h.locality = Some("philadelphia".into());
    h.state = "TX".into();
    assert!(h.plan_input(today()).unwrap_err().contains("PA"));
    h.locality = Some("atlantis".into());
    assert!(h.plan_input(today()).unwrap_err().contains("Unknown"));
    h.locality = Some(String::new());
    assert!(h.plan_input(today()).is_ok());
}

#[test]
fn a_pension_starts_at_retirement_and_pays_no_payroll_tax() {
    use fin_app::household::StreamRow;
    let mut h = Household::sample();
    h.streams.push(StreamRow {
        label: "Pension".into(),
        person: 0,
        annual_cents: 2_400_000,
        self_employed: false,
        start_year: None,
        end_year: None,
        contribution_cents: 999,
        kind: Some("pension".into()),
        employer_match_cents: 5,
        hsa_cents: 5,
    });
    let p = h.plan_input(today()).unwrap();
    let s = p.streams.last().unwrap();
    assert!(s.payroll_exempt);
    // Savings fields are ignored for a pension.
    assert_eq!(s.traditional_contribution.cents(), 0);
    assert_eq!(s.hsa.cents(), 0);
    // It starts after the first job's last year and runs for decades.
    let job_end = p.streams[0].end_year;
    assert!(s.start_year >= job_end);
    assert!(s.end_year >= s.start_year + 30);
}

#[test]
fn a_government_pension_is_a_pension_that_some_states_exempt() {
    use fin_app::household::StreamRow;
    let mut h = Household::sample();
    h.state = "NY".into();
    h.streams.push(StreamRow {
        label: "State pension".into(),
        person: 0,
        annual_cents: 4_000_000,
        self_employed: false,
        start_year: None,
        end_year: None,
        contribution_cents: 0,
        kind: Some("public_pension".into()),
        employer_match_cents: 0,
        hsa_cents: 0,
    });
    let p = h.plan_input(today()).unwrap();
    let s = p.streams.last().unwrap();
    assert!(s.payroll_exempt && s.pension && s.public_pension);
    assert!(s.start_year >= p.streams[0].end_year);
    // The same income as a private pension pays more New York tax over the plan.
    let tax = |h: &Household| -> i64 {
        let input = h.plan_input(today()).unwrap();
        fin_plan::projection::project(&input, &|_| 0.04)
            .rows
            .iter()
            .map(|r| r.income_tax.cents())
            .sum()
    };
    let public = tax(&h);
    h.streams.last_mut().unwrap().kind = Some("pension".into());
    assert!(tax(&h) > public);
}

#[test]
fn match_and_hsa_carry_through_and_bad_kinds_are_errors() {
    let mut h = Household::sample();
    h.streams[0].employer_match_cents = 300_000;
    h.streams[0].hsa_cents = 400_000;
    let p = h.plan_input(today()).unwrap();
    assert_eq!(p.streams[0].employer_match.cents(), 300_000);
    assert_eq!(p.streams[0].hsa.cents(), 400_000);
    h.streams[0].kind = Some("lottery".into());
    assert!(h.plan_input(today()).unwrap_err().contains("income type"));
    h.streams[0].kind = None;
    h.streams[0].hsa_cents = -1;
    assert!(h.plan_input(today()).unwrap_err().contains("negative"));
}

#[test]
fn military_retired_pay_starts_now_and_some_states_leave_it_out() {
    use fin_app::household::StreamRow;
    let mut h = Household::sample();
    h.state = "MO".into();
    h.streams.push(StreamRow {
        label: "Military retired pay".into(),
        person: 0,
        annual_cents: 3_000_000,
        self_employed: false,
        start_year: None,
        end_year: None,
        contribution_cents: 0,
        kind: Some("military_pension".into()),
        employer_match_cents: 0,
        hsa_cents: 0,
    });
    let p = h.plan_input(today()).unwrap();
    let s = p.streams.last().unwrap();
    assert!(s.payroll_exempt && s.pension && s.military_pension && !s.public_pension);
    assert_eq!(s.start_year, fin_core::Datelike::year(&today()));
    let tax = |h: &Household| -> i64 {
        let input = h.plan_input(today()).unwrap();
        fin_plan::projection::project(&input, &|_| 0.04)
            .rows
            .iter()
            .map(|r| r.income_tax.cents())
            .sum()
    };
    let military = tax(&h);
    // The same money as ordinary taxable income pays more Missouri tax.
    h.streams.last_mut().unwrap().kind = Some("other".into());
    assert!(tax(&h) > military);
}
