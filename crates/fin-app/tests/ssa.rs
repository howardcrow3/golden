use fin_app::App;

const STATEMENT: &str = "Your Social Security Statement\n\
    Age 62: $1,742 a month\n\
    Full retirement age (67 years): $2,490 a month\n\
    Age 70: $3,086 a month\n";

#[test]
fn reading_shows_the_amounts_and_changes_nothing() {
    let app = App::in_memory().unwrap();
    let before = app.household().unwrap();
    let s = app.read_ssa_statement(STATEMENT).unwrap();
    assert_eq!(s.monthly_at_62_cents, Some(174_200));
    assert_eq!(s.monthly_at_full_retirement_cents, Some(249_000));
    assert_eq!(s.full_retirement_age, Some(67));
    assert_eq!(s.monthly_at_70_cents, Some(308_600));
    assert_eq!(app.household().unwrap(), before);
}

#[test]
fn applying_sets_the_yearly_amount_for_the_chosen_claim_age() {
    let app = App::in_memory().unwrap();
    assert_eq!(
        app.apply_ssa_statement(STATEMENT, 0, "fra").unwrap(),
        249_000 * 12
    );
    assert_eq!(
        app.household().unwrap().people[0].ss_annual_cents,
        249_000 * 12
    );
    assert_eq!(
        app.apply_ssa_statement(STATEMENT, 0, "70").unwrap(),
        308_600 * 12
    );
    assert_eq!(
        app.household().unwrap().people[0].ss_annual_cents,
        308_600 * 12
    );
}

#[test]
fn bad_requests_are_readable_errors_and_change_nothing() {
    let app = App::in_memory().unwrap();
    let before = app.household().unwrap();
    assert!(app
        .apply_ssa_statement(STATEMENT, 9, "fra")
        .unwrap_err()
        .contains("household"));
    assert!(app
        .apply_ssa_statement(STATEMENT, 0, "65")
        .unwrap_err()
        .contains("Choose age 62"));
    assert!(app
        .apply_ssa_statement("Pay stub", 0, "fra")
        .unwrap_err()
        .contains("Social Security Statement"));
    let only_62 = "Social Security Statement\nAge 62: $1,500 a month\n";
    assert!(app
        .apply_ssa_statement(only_62, 0, "70")
        .unwrap_err()
        .contains("no amount for age 70"));
    assert_eq!(app.household().unwrap(), before);
}
