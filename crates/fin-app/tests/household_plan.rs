use fin_app::household::{Household, PersonRow, StreamRow};
use fin_app::App;
use fin_core::NaiveDate;
use fin_plan::quickplan;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn app() -> App {
    App::in_memory().unwrap().with_clock(|| d(2026, 11, 16))
}

const TODAY: fn() -> NaiveDate = || d(2026, 11, 16);

fn run(app: &App) -> quickplan::QuickPlanResult {
    let (plan, picks) = app.plan(TODAY()).unwrap();
    quickplan::run_plan(&plan, &picks, 300).unwrap()
}

fn second_person() -> PersonRow {
    PersonRow {
        name: "Sam".into(),
        birth_date: "1983-02-10".into(),
        retire_age: 60,
        ss_annual_cents: 2_400_000,
    }
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
    let mut bad = h.clone();
    bad.people[0].birth_date = "June 1981".into();
    assert!(bad.age_on(d(2026, 1, 1)).is_err());
}

#[test]
fn bad_profiles_are_refused_with_a_reason() {
    let app = app();
    let base = Household::sample();
    let mut cases = vec![];
    let mut c = base.clone();
    c.people[0].retire_age = 40;
    cases.push(("retirement age", c));
    let mut c = base.clone();
    c.state = "Texas".into();
    cases.push(("two-letter", c));
    let mut c = base.clone();
    c.spending_cents = -1;
    cases.push(("negative", c));
    let mut c = base.clone();
    c.people[0].birth_date = "2020-01-01".into();
    cases.push(("between 18 and 100", c));
    let mut c = base.clone();
    c.streams[0].person = 3;
    cases.push(("which person", c));
    let mut c = base.clone();
    c.streams[0].start_year = Some(2030);
    c.streams[0].end_year = Some(2028);
    cases.push(("before the first year", c));
    let mut c = base.clone();
    c.people.clear();
    cases.push(("between one and four", c));
    for (needle, c) in cases {
        let err = app.save_household(&c).unwrap_err();
        assert!(err.contains(needle), "{needle}: {err}");
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
    let base = run(&app);
    assert_eq!(base.years[0].age, 45);
    assert!(base.success_rate > 0.0 && base.success_rate <= 1.0);
    app.set_pick("real_return", "Cautious").unwrap();
    let cautious = run(&app);
    assert!(cautious.success_rate <= base.success_rate);
    assert!((cautious.real_return - 0.02).abs() < 1e-9);

    let mut h = app.household().unwrap();
    h.spending_cents *= 3;
    app.save_household(&h).unwrap();
    assert!(run(&app).success_rate < cautious.success_rate);
}

#[test]
fn a_second_person_and_their_income_help_the_plan() {
    let app = app();
    let mut h = app.household().unwrap();
    h.spending_cents = 12_000_000;
    app.save_household(&h).unwrap();
    let alone = run(&app);

    h.people.push(second_person());
    h.streams.push(StreamRow {
        self_employed: false,
        label: "Sam salary".into(),
        person: 1,
        annual_cents: 9_000_000,
        start_year: None,
        end_year: None,
        contribution_cents: 1_000_000,
    });
    app.save_household(&h).unwrap();
    let (plan, _) = app.plan(TODAY()).unwrap();
    assert_eq!(plan.people.len(), 2);
    assert_eq!(plan.social_security.len(), 2);
    let sam = plan
        .streams
        .iter()
        .find(|s| s.label == "Sam salary")
        .unwrap();
    // Sam is 43, retires at 60: last working year is 2026 + 17 - 1.
    assert_eq!((sam.start_year, sam.end_year), (2026, 2042));
    assert!(run(&app).success_rate >= alone.success_rate);
}

#[test]
fn lumpy_income_is_a_one_time_stream_in_its_own_year() {
    let app = app();
    let mut h = app.household().unwrap();
    let without = {
        h.spending_cents = 4_000_000;
        h.streams[0].annual_cents = 0;
        app.save_household(&h).unwrap();
        run(&app)
    };
    h.streams.push(StreamRow {
        self_employed: false,
        label: "Stock vest".into(),
        person: 0,
        annual_cents: 40_000_000,
        start_year: Some(2028),
        end_year: Some(2028),
        contribution_cents: 0,
    });
    app.save_household(&h).unwrap();
    let (plan, _) = app.plan(TODAY()).unwrap();
    let vest = plan
        .streams
        .iter()
        .find(|s| s.label == "Stock vest")
        .unwrap();
    assert_eq!((vest.start_year, vest.end_year), (2028, 2028));
    assert!(
        plan.streams.iter().all(|s| s.label != "Salary"),
        "zero-income stream is skipped"
    );
    let with = run(&app);
    assert!(with.p50_cents > without.p50_cents);
    // The vest year shows more earned-then-saved balance than the year before it.
    let y = |r: &quickplan::QuickPlanResult, yr| {
        r.years.iter().find(|p| p.year == yr).unwrap().balance_cents
    };
    assert!(y(&with, 2028) - y(&with, 2027) > y(&without, 2028) - y(&without, 2027));
}

#[test]
fn someone_already_past_retirement_age_has_no_default_salary() {
    let app = app();
    let mut h = app.household().unwrap();
    h.people[0].retire_age = 45; // already there
    app.save_household(&h).unwrap();
    let (plan, _) = app.plan(TODAY()).unwrap();
    assert!(plan.streams.is_empty());
}

#[test]
fn a_profile_saved_by_the_first_version_still_loads() {
    let app = app();
    let legacy = r#"{"birthDate":"1981-06-15","retireAge":62,"filing":"married_joint","state":"TX",
        "salaryCents":18000000,"contributionCents":2350000,"spendingCents":9000000,"cashCents":3000000,
        "taxableCents":15000000,"traditionalCents":60000000,"rothCents":12000000,"ssAnnualCents":3800000,
        "picks":{"real_return":"Cautious"}}"#;
    app.db().set_setting("household", legacy).unwrap();
    let h = app.household().unwrap();
    assert_eq!(h.people.len(), 1);
    assert_eq!(h.people[0].name, "You");
    assert_eq!(h.streams[0].label, "Salary");
    assert_eq!(h.streams[0].annual_cents, 18_000_000);
    assert_eq!(h.picks["real_return"], "Cautious");
    // saving rewrites it in the new shape
    app.save_household(&h).unwrap();
    assert!(app
        .db()
        .get_setting("household")
        .unwrap()
        .unwrap()
        .contains("\"people\""));
}

#[test]
fn an_expired_trial_cannot_change_the_profile_but_can_still_read_it() {
    let app = app();
    app.license_status(d(2026, 1, 1)).unwrap(); // trial began long ago
    assert!(app.save_household(&Household::sample()).is_err());
    assert!(app.household().is_ok());
    assert!(app.assumptions().is_ok());
}
