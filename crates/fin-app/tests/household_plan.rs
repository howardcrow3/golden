use fin_app::household::Household;
use fin_app::App;
use fin_core::NaiveDate;
use fin_plan::quickplan;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn app() -> App {
    App::in_memory().unwrap().with_clock(|| d(2026, 11, 16))
}

#[test]
fn first_run_shows_an_editable_sample_and_saves_changes() {
    let app = app();
    assert_eq!(app.household().unwrap(), Household::sample());
    let mut h = app.household().unwrap();
    h.state = "ca".into();
    h.spending_cents = 7_000_000;
    app.save_household(&h).unwrap();
    let got = app.household().unwrap();
    assert_eq!(got.state, "CA");
    assert_eq!(got.spending_cents, 7_000_000);
}

#[test]
fn age_comes_from_the_birth_date_including_before_the_birthday() {
    let h = Household::sample(); // 1981-06-15
    assert_eq!(h.age_on(d(2026, 6, 14)).unwrap(), 44);
    assert_eq!(h.age_on(d(2026, 6, 15)).unwrap(), 45);
    assert!(Household {
        birth_date: "June 1981".into(),
        ..h
    }
    .age_on(d(2026, 1, 1))
    .is_err());
}

#[test]
fn bad_profiles_are_refused_with_a_reason() {
    let app = app();
    let base = Household::sample();
    let cases = [
        Household {
            retire_age: 40,
            ..base.clone()
        },
        Household {
            state: "Texas".into(),
            ..base.clone()
        },
        Household {
            spending_cents: -1,
            ..base.clone()
        },
        Household {
            birth_date: "2020-01-01".into(),
            ..base.clone()
        },
    ];
    for c in cases {
        assert!(app.save_household(&c).is_err(), "{c:?}");
    }
    assert_eq!(app.household().unwrap(), base);
}

#[test]
fn assumptions_list_options_with_probabilities_and_the_active_one() {
    let app = app();
    let rows = app.assumptions().unwrap();
    let ret = rows.iter().find(|r| r.key == "real_return").unwrap();
    assert_eq!(ret.options.len(), 3);
    assert_eq!(ret.active, "Historical");
    assert!(!ret.picked);
    let total: f64 = ret.options.iter().map(|o| o.probability).sum();
    assert!((total - 1.0).abs() < 1e-9);

    app.set_pick("real_return", "Cautious").unwrap();
    let ret = app
        .assumptions()
        .unwrap()
        .into_iter()
        .find(|r| r.key == "real_return")
        .unwrap();
    assert_eq!((ret.active.as_str(), ret.picked), ("Cautious", true));
    app.set_pick("real_return", "").unwrap();
    assert!(
        !app.assumptions()
            .unwrap()
            .iter()
            .find(|r| r.key == "real_return")
            .unwrap()
            .picked
    );
    assert!(app.set_pick("real_return", "Moonshot").is_err());
    assert!(app.set_pick("nonsense", "Flat").is_err());
}

#[test]
fn saved_household_drives_the_plan_and_a_cautious_pick_lowers_success() {
    let app = app();
    let base = quickplan::run(&app.plan_request(d(2026, 11, 16), 300).unwrap()).unwrap();
    assert_eq!(base.years[0].age, 45);
    assert!(base.success_rate > 0.0 && base.success_rate <= 1.0);
    app.set_pick("real_return", "Cautious").unwrap();
    let cautious = quickplan::run(&app.plan_request(d(2026, 11, 16), 300).unwrap()).unwrap();
    assert!(cautious.success_rate <= base.success_rate);
    assert!((cautious.real_return - 0.02).abs() < 1e-9);

    let mut h = app.household().unwrap();
    h.spending_cents *= 3;
    app.save_household(&h).unwrap();
    let broke = quickplan::run(&app.plan_request(d(2026, 11, 16), 300).unwrap()).unwrap();
    assert!(broke.success_rate < cautious.success_rate);
}

#[test]
fn an_expired_trial_cannot_change_the_profile_but_can_still_read_it() {
    let app = app();
    app.license_status(d(2026, 1, 1)).unwrap(); // trial began long ago
    assert!(app.save_household(&Household::sample()).is_err());
    assert!(app.household().is_ok());
    assert!(app.assumptions().is_ok());
}
