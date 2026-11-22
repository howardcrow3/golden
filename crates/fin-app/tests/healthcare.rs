use fin_app::household::Household;
use fin_core::NaiveDate;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}

#[test]
fn healthcare_estimates_are_off_until_asked_for() {
    let h = Household::sample();
    assert!(!h.health);
    assert!(h.plan_input(today()).unwrap().one_time_costs.is_empty());
}

#[test]
fn turning_them_on_adds_a_cost_every_year() {
    let mut h = Household::sample();
    h.health = true;
    let plan = h.plan_input(today()).unwrap();
    let years: Vec<i32> = plan.one_time_costs.iter().map(|c| c.year).collect();
    assert!(years.contains(&2026));
    assert!(years.windows(2).all(|w| w[1] == w[0] + 1));
    assert!(plan.one_time_costs.iter().all(|c| c.amount.cents() > 0));
}

#[test]
fn they_sit_alongside_planned_events() {
    let mut h = Household::sample();
    h.health = true;
    h.events.push(fin_app::household::EventRow {
        label: "Roof".into(),
        year: 2030,
        cents: 2_000_000,
    });
    let plan = h.plan_input(today()).unwrap();
    assert!(plan
        .one_time_costs
        .iter()
        .any(|c| c.year == 2030 && c.amount.cents() == 2_000_000));
    assert!(plan.one_time_costs.len() > 40);
}

#[test]
fn healthcare_lowers_the_final_balance() {
    let base = Household::sample();
    let mut with = base.clone();
    with.health = true;
    let run = |h: &Household| {
        let plan = h.plan_input(today()).unwrap();
        fin_plan::projection::project_constant(&plan, 0.04).final_balance()
    };
    assert!(run(&with) < run(&base));
}

#[test]
fn old_saved_households_still_load() {
    let json = serde_json::to_value(Household::sample()).unwrap();
    let mut obj = json.as_object().unwrap().clone();
    obj.remove("health");
    let h: Household = serde_json::from_value(obj.into()).unwrap();
    assert!(!h.health);
}

#[test]
fn giving_is_carried_into_the_plan_and_defaults_to_nothing() {
    let mut h = Household::sample();
    assert_eq!(h.plan_input(today()).unwrap().charity.cents(), 0);
    h.giving_cents = 1_200_000;
    assert_eq!(h.plan_input(today()).unwrap().charity.cents(), 1_200_000);
    h.giving_cents = -5;
    assert_eq!(h.plan_input(today()).unwrap().charity.cents(), 0);
}

#[test]
fn your_own_healthcare_figures_replace_the_defaults() {
    use fin_app::household::HealthRow;
    let today = fin_core::NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();
    let mut h = Household::sample();
    h.health = true;
    let total = |h: &Household| -> i64 {
        h.plan_input(today)
            .unwrap()
            .one_time_costs
            .iter()
            .map(|c| c.amount.cents())
            .sum()
    };
    let default_total = total(&h);
    h.health_detail = Some(HealthRow {
        pre_65_cents: 0,
        post_65_cents: 0,
        excess_inflation_bp: 0,
        ltc_start_age: 0,
        ltc_years: 0,
        ltc_annual_cents: 0,
        ltc_chance_bp: 0,
    });
    assert_eq!(total(&h), 0);
    assert!(default_total > 0);
    // Ignored while the switch is off.
    h.health = false;
    assert_eq!(total(&h), 0);
    // Bad figures are readable errors.
    h.health = true;
    h.health_detail.as_mut().unwrap().ltc_chance_bp = 20_000;
    assert!(h.plan_input(today).unwrap_err().contains("chance"));
}
