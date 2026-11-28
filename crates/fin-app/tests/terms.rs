use fin_app::App;
use fin_core::NaiveDate;

fn app() -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 28).unwrap())
}

#[test]
fn nothing_is_accepted_on_a_new_install() {
    assert_eq!(app().terms_acceptance().unwrap(), None);
}

#[test]
fn remembers_the_version_and_the_day() {
    let a = app();
    let got = a.accept_terms(" 2026-11-28 ").unwrap();
    assert_eq!(got.version, "2026-11-28");
    assert_eq!(got.accepted_on, "2026-11-28");
    assert_eq!(a.terms_acceptance().unwrap(), Some(got));
}

#[test]
fn a_newer_version_replaces_the_older_one() {
    let a = app();
    a.accept_terms("1").unwrap();
    a.accept_terms("2").unwrap();
    assert_eq!(a.terms_acceptance().unwrap().unwrap().version, "2");
}

#[test]
fn refuses_an_empty_or_huge_version() {
    let a = app();
    assert!(a.accept_terms("  ").is_err());
    assert!(a.accept_terms(&"x".repeat(65)).is_err());
    assert_eq!(a.terms_acceptance().unwrap(), None);
}

#[test]
fn can_be_accepted_after_the_trial_ends() {
    let a = app();
    a.license_status(NaiveDate::from_ymd_opt(2026, 11, 28).unwrap())
        .unwrap();
    let late = a.with_clock(|| NaiveDate::from_ymd_opt(2027, 6, 1).unwrap());
    assert!(
        !late
            .license_status(NaiveDate::from_ymd_opt(2027, 6, 1).unwrap())
            .unwrap()
            .can_edit
    );
    assert_eq!(late.accept_terms("1").unwrap().accepted_on, "2027-06-01");
}

#[test]
fn delete_everything_keeps_the_acceptance() {
    let mut a = app();
    a.accept_terms("1").unwrap();
    a.erase_all_data("DELETE").unwrap();
    assert_eq!(a.terms_acceptance().unwrap().unwrap().version, "1");
}
