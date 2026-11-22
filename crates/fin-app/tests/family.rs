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
fn a_city_tax_adds_yearly_costs_while_earning() {
    let mut h = Household::sample();
    h.state = "PA".into();
    h.locality = Some("philadelphia".into());
    let p = h.plan_input(today()).unwrap();
    let first = p.one_time_costs.iter().find(|c| c.year == 2026).unwrap();
    assert!(first.amount.cents() > 0);
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
