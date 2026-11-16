use fin_app::App;

const STUB: &str = "Employer: Acme Corp
Pay Period Beginning: 10/01/2026
Pay Period Ending: 10/15/2026
Gross Pay 4,000.00
Federal Income Tax 420.00
State Income Tax 0.00
Social Security 248.00
Medicare 58.00
401(k) 320.00
Net Pay 2,954.00
";

fn single_person_app(salary: i64, state: &str) -> App {
    let app = App::in_memory().unwrap();
    let mut h = app.household().unwrap();
    h.people.truncate(1);
    h.streams.clear();
    h.filing = "single".into();
    h.state = state.into();
    app.save_household(&h).unwrap();
    let _ = salary;
    app
}

#[test]
fn no_income_sources_means_nothing_to_check() {
    let a = single_person_app(0, "TX");
    let c = a.withholding_check().unwrap();
    assert_eq!(c.taxable_wages_cents, 0);
    assert!(c.missing.is_empty());
    assert_eq!(c.federal.estimated_cents, 0);
}

#[test]
fn a_paystub_records_withholding_and_the_check_compares_it() {
    let a = single_person_app(0, "TX");
    a.apply_paystub(STUB, 0).unwrap();
    let c = a.withholding_check().unwrap();
    // 4,000 x 24 = 96,000 gross; 401(k) 320 x 24 = 7,680 -> taxable 88,320.
    assert_eq!(c.taxable_wages_cents, 8_832_000);
    assert_eq!(c.federal.withheld_cents, 42_000 * 24);
    assert!(c.federal.estimated_cents > 0);
    assert_eq!(
        c.federal.difference_cents,
        c.federal.withheld_cents - c.federal.estimated_cents
    );
    assert!(c.missing.is_empty());
}

#[test]
fn too_little_withheld_shows_a_negative_difference_and_too_much_a_positive_one() {
    let a = single_person_app(0, "TX");
    a.apply_paystub(
        &STUB.replace("Federal Income Tax 420.00", "Federal Income Tax 50.00"),
        0,
    )
    .unwrap();
    assert!(a.withholding_check().unwrap().federal.difference_cents < 0);
    a.apply_paystub(
        &STUB
            .replace("Federal Income Tax 420.00", "Federal Income Tax 900.00")
            .replace("Net Pay 2,954.00", "Net Pay 2,474.00"),
        0,
    )
    .unwrap();
    assert!(a.withholding_check().unwrap().federal.difference_cents > 0);
}

#[test]
fn a_job_without_a_paystub_is_named_not_guessed() {
    let a = single_person_app(0, "TX");
    let mut h = a.household().unwrap();
    h.streams.push(fin_app::household::StreamRow {
        label: "Side gig".into(),
        person: 0,
        annual_cents: 2_000_000,
        start_year: None,
        end_year: None,
        contribution_cents: 0,
    });
    a.save_household(&h).unwrap();
    let c = a.withholding_check().unwrap();
    assert_eq!(c.missing, vec!["Side gig".to_string()]);
}

#[test]
fn one_time_amounts_are_left_out() {
    let a = single_person_app(0, "TX");
    let mut h = a.household().unwrap();
    h.streams.push(fin_app::household::StreamRow {
        label: "Bonus".into(),
        person: 0,
        annual_cents: 5_000_000,
        start_year: Some(2026),
        end_year: Some(2026),
        contribution_cents: 0,
    });
    a.save_household(&h).unwrap();
    let c = a.withholding_check().unwrap();
    assert_eq!(c.taxable_wages_cents, 0);
    assert!(c.missing.is_empty());
}

#[test]
fn state_withholding_uses_the_households_state() {
    let a = single_person_app(0, "NC");
    a.apply_paystub(
        &STUB
            .replace("State Income Tax 0.00", "State Income Tax 150.00")
            .replace("Net Pay 2,954.00", "Net Pay 2,804.00"),
        0,
    )
    .unwrap();
    let c = a.withholding_check().unwrap();
    assert_eq!(c.state.withheld_cents, 15_000 * 24);
    assert!(c.state.estimated_cents > 0);
}

#[test]
fn it_updates_when_a_newer_paystub_for_the_same_job_is_applied() {
    let a = single_person_app(0, "TX");
    a.apply_paystub(STUB, 0).unwrap();
    a.apply_paystub(
        &STUB
            .replace("Federal Income Tax 420.00", "Federal Income Tax 500.00")
            .replace("Net Pay 2,954.00", "Net Pay 2,874.00"),
        0,
    )
    .unwrap();
    assert_eq!(
        a.withholding_check().unwrap().federal.withheld_cents,
        50_000 * 24
    );
}
