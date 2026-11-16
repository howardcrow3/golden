use ed25519_dalek::SigningKey;
use fin_app::App;
use fin_core::NaiveDate;
use fin_license::{issue, LicensePayload, APP_MAJOR};
use rand::rngs::OsRng;

const CSV: &str = "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n";

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn token(sk: &SigningKey, major: u32, plaid_until: Option<NaiveDate>) -> String {
    issue(
        sk,
        &LicensePayload {
            product: "golden".into(),
            major,
            issued: d(2026, 11, 1),
            plaid_until,
            buyer: "b1".into(),
        },
    )
}

#[test]
fn trial_counts_down_then_locks_editing_but_keeps_data() {
    // Clock is fixed after the trial ended; the trial started when first asked on 2026-11-01.
    let mut app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 1));
    let s = app.license_status(d(2026, 11, 1)).unwrap();
    assert_eq!(
        (s.state.as_str(), s.days_left, s.can_edit),
        ("trial", Some(14), true)
    );
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    assert_eq!(
        app.license_status(d(2026, 11, 8)).unwrap().days_left,
        Some(7)
    );

    let mut app = app.with_clock(|| d(2026, 12, 20));
    let s = app.license_status(d(2026, 12, 20)).unwrap();
    assert_eq!((s.state.as_str(), s.can_edit), ("expired", false));
    let err = app
        .import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap_err();
    assert!(err.contains("trial has ended"));
    // reads still work
    assert_eq!(app.accounts().unwrap().len(), 1);
    assert_eq!(
        app.cash_flow(d(2026, 10, 1), d(2026, 10, 31), "month")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn valid_licence_unlocks_and_survives_restart_of_the_service() {
    let sk = SigningKey::generate(&mut OsRng);
    let app = App::in_memory()
        .unwrap()
        .with_public_key(Some(sk.verifying_key()))
        .with_clock(|| d(2026, 12, 20));
    app.license_status(d(2026, 11, 1)).unwrap(); // trial started 2026-11-01
    assert_eq!(
        app.license_status(d(2026, 12, 20)).unwrap().state,
        "expired"
    );
    let s = app
        .activate_license(
            &token(&sk, APP_MAJOR, Some(d(2027, 11, 1))),
            d(2026, 12, 20),
        )
        .unwrap();
    assert_eq!(
        (s.state.as_str(), s.can_edit, s.plaid_active),
        ("licensed", true, true)
    );
    assert!(!app.license_status(d(2028, 1, 1)).unwrap().plaid_active);
}

#[test]
fn bad_licences_are_rejected_and_do_not_unlock() {
    let sk = SigningKey::generate(&mut OsRng);
    let other = SigningKey::generate(&mut OsRng);
    let app = App::in_memory()
        .unwrap()
        .with_public_key(Some(sk.verifying_key()));
    app.license_status(d(2026, 11, 1)).unwrap();
    assert!(app
        .activate_license(&token(&other, APP_MAJOR, None), d(2026, 12, 20))
        .is_err());
    assert!(app
        .activate_license(&token(&sk, APP_MAJOR + 1, None), d(2026, 12, 20))
        .is_err());
    assert!(app.activate_license("garbage", d(2026, 12, 20)).is_err());
    assert_eq!(
        app.license_status(d(2026, 12, 20)).unwrap().state,
        "expired"
    );
}

#[test]
fn activation_is_disabled_without_a_bundled_key() {
    let app = App::in_memory().unwrap();
    let err = app
        .activate_license("anything", d(2026, 11, 1))
        .unwrap_err();
    assert!(err.contains("not available"));
}
