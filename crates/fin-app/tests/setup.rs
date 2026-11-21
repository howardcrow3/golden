use fin_app::goals::Goal;
use fin_app::App;
use fin_core::NaiveDate;

fn app() -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap())
}
fn done(a: &App) -> Vec<&'static str> {
    a.setup_progress()
        .unwrap()
        .steps
        .into_iter()
        .filter(|s| s.done)
        .map(|s| s.key)
        .collect()
}

#[test]
fn a_new_install_has_nothing_done() {
    let s = app().setup_progress().unwrap();
    assert_eq!((s.done, s.complete, s.steps.len()), (0, false, 5));
}

#[test]
fn importing_ticks_import_and_categories_only_once_everything_is_filed() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n11/01/2026,ACME PAYROLL,3000.00\n11/02/2026,MYSTERY VENDOR 9,-20.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    assert_eq!(done(&a), vec!["import"]);
    let id = a.transactions("2026-11-01", "2026-11-30", true).unwrap()[0].id;
    a.set_category(id, "Hobbies", false).unwrap();
    assert_eq!(done(&a), vec!["import", "categories"]);
}

#[test]
fn saving_the_household_a_budget_and_a_goal_tick_their_steps() {
    let a = app();
    let h = a.household().unwrap();
    assert!(!done(&a).contains(&"household"));
    a.save_household(&h).unwrap();
    assert_eq!(done(&a), vec!["household"]);
    a.set_budget("Food", 40_000).unwrap();
    a.set_goal(Goal {
        name: "Trip".into(),
        target_cents: 100_000,
        saved_cents: 0,
        by_date: "2027-06-01".into(),
    })
    .unwrap();
    assert_eq!(done(&a), vec!["household", "budget", "goal"]);
}

#[test]
fn doing_all_five_completes_it() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n11/01/2026,ACME PAYROLL,3000.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    a.save_household(&a.household().unwrap()).unwrap();
    a.set_budget("Food", 40_000).unwrap();
    a.set_goal(Goal {
        name: "Trip".into(),
        target_cents: 100_000,
        saved_cents: 0,
        by_date: "2027-06-01".into(),
    })
    .unwrap();
    let s = a.setup_progress().unwrap();
    assert!(s.complete);
    assert_eq!(s.done, 5);
}

#[test]
fn every_step_names_a_real_sidebar_page() {
    let pages = [
        "Overview",
        "Transactions",
        "Import",
        "Retirement",
        "What if",
        "Tax estimate",
        "Licence",
        "About",
    ];
    for s in app().setup_progress().unwrap().steps {
        assert!(pages.contains(&s.page), "{}", s.page);
    }
}
