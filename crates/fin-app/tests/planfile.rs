use fin_app::household::Household;
use fin_app::App;
use fin_plan::investments::{Class, Holding};

fn holding() -> Holding {
    Holding {
        name: "Total market".into(),
        value_cents: 5_000_000,
        class: Class::Stock,
        expense_bp: 3,
        single_stock: false,
    }
}

#[test]
fn a_plan_travels_from_one_app_to_another() {
    let a = App::in_memory().unwrap();
    let mut h = Household::sample();
    h.spending_cents = 7_654_300;
    a.save_household(&h).unwrap();
    a.save_holdings(&[holding()]).unwrap();
    let file = a.export_plan_file().unwrap();

    let b = App::in_memory().unwrap();
    let r = b.import_plan_file(&file).unwrap();
    assert_eq!(r.holdings, 1);
    assert_eq!(r.people, h.people.len());
    assert_eq!(b.household().unwrap().spending_cents, 7_654_300);
    assert_eq!(b.holdings().unwrap(), vec![holding()]);
}

#[test]
fn files_that_are_not_plans_change_nothing() {
    let b = App::in_memory().unwrap();
    let before = b.household().unwrap();
    for bad in [
        "",
        "not json",
        "{}",
        r#"{"format":"other","version":1,"household":{}}"#,
    ] {
        assert!(
            b.import_plan_file(bad).unwrap_err().contains("plan file"),
            "{bad}"
        );
    }
    assert_eq!(b.household().unwrap(), before);
}

#[test]
fn a_newer_file_is_refused_and_an_invalid_household_does_not_save() {
    let a = App::in_memory().unwrap();
    let file = a.export_plan_file().unwrap();
    let newer = file.replace("\"version\": 1", "\"version\": 99");
    let b = App::in_memory().unwrap();
    assert!(b.import_plan_file(&newer).unwrap_err().contains("newer"));
    // A household that fails validation is refused before anything is saved.
    let mut v: serde_json::Value = serde_json::from_str(&file).unwrap();
    v["household"]["people"] = serde_json::json!([]);
    let c = App::in_memory().unwrap();
    let before = c.household().unwrap();
    assert!(c.import_plan_file(&v.to_string()).is_err());
    assert_eq!(c.household().unwrap(), before);
}

#[test]
fn a_file_with_bad_holdings_leaves_the_household_untouched() {
    let a = App::in_memory().unwrap();
    let mut h = Household::sample();
    h.spending_cents = 1_234_500;
    a.save_household(&h).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&a.export_plan_file().unwrap()).unwrap();
    let mut nameless = holding();
    nameless.name = "  ".into();
    v["holdings"] = serde_json::to_value(vec![nameless]).unwrap();

    let b = App::in_memory().unwrap();
    let before = b.household().unwrap();
    assert!(b.import_plan_file(&v.to_string()).is_err());
    // Everything is checked before anything changes: the household was not replaced.
    assert_eq!(b.household().unwrap(), before);
    assert!(!b.household_is_saved().unwrap());
}
