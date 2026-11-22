use fin_app::household::Household;
use fin_app::App;
use fin_core::NaiveDate;
use fin_plan::projection::{SpendingRule, WithdrawalOrder};

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}
fn app() -> App {
    App::in_memory().unwrap().with_clock(today)
}

#[test]
fn a_household_saved_before_these_options_still_loads_with_the_defaults() {
    let a = app();
    let mut h = a.household().unwrap();
    assert_eq!(h.spending_rule, "fixed");
    assert_eq!(h.withdrawal_order, "standard");
    // An older saved profile has no such keys at all.
    let mut v = serde_json::to_value(&h).unwrap();
    v.as_object_mut().unwrap().remove("spendingRule");
    v.as_object_mut().unwrap().remove("withdrawalOrder");
    h = Household::from_json(&v.to_string()).unwrap();
    assert_eq!(h.spending_rule, "fixed");
    assert_eq!(h.withdrawal_order, "standard");
    let plan = h.plan_input(today()).unwrap();
    assert_eq!(plan.spending_rule, SpendingRule::Fixed);
    assert_eq!(plan.withdrawal_order, WithdrawalOrder::Standard);
}

#[test]
fn chosen_rules_reach_the_plan_and_survive_a_save() {
    let a = app();
    let mut h = a.household().unwrap();
    h.spending_rule = "guardrails".into();
    h.withdrawal_order = "roth_first".into();
    a.save_household(&h).unwrap();
    let (plan, _) = a.plan(today()).unwrap();
    assert!(matches!(
        plan.spending_rule,
        SpendingRule::Guardrails { .. }
    ));
    assert_eq!(plan.withdrawal_order, WithdrawalOrder::RothFirst);
    for (rule, ok) in [
        ("percent", true),
        ("vpw", true),
        ("fixed", true),
        ("yolo", false),
    ] {
        h.spending_rule = rule.into();
        assert_eq!(h.plan_input(today()).is_ok(), ok, "{rule}");
    }
    h.spending_rule = "fixed".into();
    h.withdrawal_order = "backwards".into();
    assert!(h.plan_input(today()).is_err());
}

#[test]
fn the_plan_result_changes_with_the_rule() {
    use fin_plan::quickplan;
    let a = app();
    let mut h = a.household().unwrap();
    // Spend more than the portfolio comfortably carries.
    h.spending_cents = 22_000_000;
    h.spending_rule = "fixed".into();
    let fixed = {
        let plan = h.plan_input(today()).unwrap();
        quickplan::run_plan(&plan, &h.picks, 100).unwrap()
    };
    h.spending_rule = "guardrails".into();
    let guard = {
        let plan = h.plan_input(today()).unwrap();
        quickplan::run_plan(&plan, &h.picks, 100).unwrap()
    };
    assert!(guard.success_rate >= fixed.success_rate);
}
