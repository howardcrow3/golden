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
            core: None,
        },
    )
}

fn keyed(sk: &SigningKey, plaid_until: Option<NaiveDate>, core: Option<bool>) -> String {
    issue(
        sk,
        &LicensePayload {
            product: "golden".into(),
            major: APP_MAJOR,
            issued: d(2026, 11, 1),
            plaid_until,
            buyer: "b1".into(),
            core,
        },
    )
}

fn licensed_app(sk: &SigningKey) -> App {
    let app = App::in_memory()
        .unwrap()
        .with_public_key(Some(sk.verifying_key()))
        .with_clock(|| d(2026, 12, 20));
    app.license_status(d(2026, 11, 1)).unwrap(); // trial started 2026-11-01, over by 12-20
    app
}

#[test]
fn an_addon_only_key_turns_on_bank_sync_but_does_not_unlock_the_app() {
    let sk = SigningKey::generate(&mut OsRng);
    let app = licensed_app(&sk);
    let s = app
        .activate_license(
            &keyed(&sk, Some(d(2027, 1, 20)), Some(false)),
            d(2026, 12, 20),
        )
        .unwrap();
    assert_eq!(
        (s.state.as_str(), s.can_edit, s.plaid_active),
        ("expired", false, true)
    );
    // Buying the app afterwards unlocks it and keeps the add-on's date.
    let s = app
        .activate_license(&keyed(&sk, None, Some(true)), d(2026, 12, 20))
        .unwrap();
    assert_eq!((s.state.as_str(), s.plaid_active), ("licensed", true));
    // The sync service is sent the key that carries the add-on.
    assert!(app.license_status(d(2027, 1, 20)).unwrap().plaid_active);
}

#[test]
fn an_addon_only_key_never_takes_away_the_app_someone_bought() {
    let sk = SigningKey::generate(&mut OsRng);
    for app_key in [
        keyed(&sk, None, None),
        keyed(&sk, None, Some(true)),
        keyed(&sk, Some(d(2026, 12, 25)), None),
    ] {
        let app = licensed_app(&sk);
        app.activate_license(&app_key, d(2026, 12, 20)).unwrap();
        // A renewal key that only covers the add-on, with a later date.
        let s = app
            .activate_license(
                &keyed(&sk, Some(d(2027, 2, 20)), Some(false)),
                d(2026, 12, 20),
            )
            .unwrap();
        assert_eq!((s.state.as_str(), s.plaid_active), ("licensed", true));
        let later = app.license_status(d(2027, 2, 1)).unwrap();
        assert_eq!(
            (later.state.as_str(), later.plaid_active),
            ("licensed", true)
        );
        // An older add-on key changes nothing.
        app.activate_license(
            &keyed(&sk, Some(d(2027, 1, 1)), Some(false)),
            d(2026, 12, 20),
        )
        .unwrap();
        assert!(app.license_status(d(2027, 2, 1)).unwrap().plaid_active);
    }
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

#[test]
fn an_older_email_never_downgrades_the_plaid_date() {
    let sk = SigningKey::generate(&mut OsRng);
    let app = App::in_memory()
        .unwrap()
        .with_public_key(Some(sk.verifying_key()))
        .with_clock(|| d(2026, 12, 20));
    let core_only = token(&sk, APP_MAJOR, None);
    let monthly = token(&sk, APP_MAJOR, Some(d(2026, 12, 25)));
    let annual = token(&sk, APP_MAJOR, Some(d(2027, 12, 20)));

    app.activate_license(&core_only, d(2026, 12, 20)).unwrap();
    assert!(!app.license_status(d(2026, 12, 21)).unwrap().plaid_active);
    app.activate_license(&monthly, d(2026, 12, 20)).unwrap();
    assert!(app.license_status(d(2026, 12, 21)).unwrap().plaid_active);
    // Upgrade to the annual plan, then click the old links again.
    app.activate_license(&annual, d(2026, 12, 20)).unwrap();
    app.activate_license(&monthly, d(2026, 12, 20)).unwrap();
    app.activate_license(&core_only, d(2026, 12, 20)).unwrap();
    let s = app.license_status(d(2027, 6, 1)).unwrap();
    assert_eq!((s.state.as_str(), s.plaid_active), ("licensed", true));
    // A later renewal still replaces it.
    let later = token(&sk, APP_MAJOR, Some(d(2028, 12, 20)));
    app.activate_license(&later, d(2026, 12, 20)).unwrap();
    assert!(app.license_status(d(2028, 6, 1)).unwrap().plaid_active);
}

#[test]
fn the_trial_ends_exactly_after_fourteen_days() {
    let app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 1));
    app.license_status(d(2026, 11, 1)).unwrap();
    let day = |n| {
        app.license_status(d(2026, 11, 1) + chrono::Duration::days(n))
            .unwrap()
    };
    assert_eq!(
        (day(13).state.as_str(), day(13).days_left),
        ("trial", Some(1))
    );
    assert_eq!(day(14).state, "expired");
    // Setting the clock back, even before the first run, does not restart the trial.
    assert_eq!(day(-30).state, "expired");
}

#[test]
fn setting_the_clock_back_does_not_give_the_trial_back() {
    let app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 1));
    let at = |y, m, dd| app.license_status(d(y, m, dd)).unwrap();
    assert_eq!(at(2026, 11, 1).days_left, Some(14));
    assert_eq!(at(2026, 11, 11).days_left, Some(4)); // day 10
                                                     // Back to the first day, and to before the first run.
    assert_eq!(at(2026, 11, 1).days_left, Some(4));
    assert_eq!(at(2026, 10, 1).days_left, Some(4));
    // Time moving on normally still counts down.
    assert_eq!(at(2026, 11, 13).days_left, Some(2));
    assert_eq!(at(2026, 11, 15).state, "expired");
    let back = at(2026, 11, 2);
    assert_eq!((back.state.as_str(), back.can_edit), ("expired", false));
}

#[test]
fn a_clock_set_back_before_the_first_run_on_day_four_keeps_ten_days() {
    let app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 1));
    app.license_status(d(2026, 11, 1)).unwrap();
    assert_eq!(
        app.license_status(d(2026, 11, 5)).unwrap().days_left,
        Some(10)
    );
    assert_eq!(
        app.license_status(d(2025, 1, 1)).unwrap().days_left,
        Some(10)
    );
}

#[test]
fn a_clock_that_was_wrongly_ahead_never_locks_a_licensed_app() {
    let sk = SigningKey::generate(&mut OsRng);
    let mut app = App::in_memory()
        .unwrap()
        .with_public_key(Some(sk.verifying_key()))
        .with_clock(|| d(2026, 11, 2));
    app.license_status(d(2026, 11, 1)).unwrap();
    // The clock was years ahead once: the trial is used up (that is the most it costs) ...
    let s = app.license_status(d(2031, 1, 1)).unwrap();
    assert_eq!(s.state, "expired");
    let s = app.license_status(d(2026, 11, 2)).unwrap();
    assert_eq!((s.state.as_str(), s.can_edit), ("expired", false));
    // ... reads still work, and a licence unlocks everything whatever the clock says.
    assert!(app.accounts().is_ok());
    let s = app
        .activate_license(&token(&sk, APP_MAJOR, Some(d(2027, 11, 1))), d(2026, 11, 2))
        .unwrap();
    assert_eq!((s.state.as_str(), s.can_edit), ("licensed", true));
    // The add-on's date follows the real clock, not the latest date seen.
    assert!(app.license_status(d(2026, 11, 2)).unwrap().plaid_active);
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
}

#[test]
fn erasing_or_restoring_a_backup_does_not_reset_the_latest_date_seen() {
    let mut app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 11));
    app.license_status(d(2026, 11, 1)).unwrap();
    assert_eq!(
        app.license_status(d(2026, 11, 11)).unwrap().days_left,
        Some(4)
    );
    app.erase_all_data("DELETE").unwrap();
    assert_eq!(
        app.license_status(d(2026, 11, 1)).unwrap().days_left,
        Some(4)
    );
    let fresh = App::in_memory().unwrap().with_clock(|| d(2026, 11, 1));
    fresh.license_status(d(2026, 11, 1)).unwrap();
    let backup = fresh.backup_json().unwrap();
    app.restore_backup(&backup).unwrap();
    assert_eq!(
        app.license_status(d(2026, 11, 1)).unwrap().days_left,
        Some(4)
    );
}

#[test]
fn an_addon_key_saved_by_an_older_version_survives_buying_the_app() {
    // Older versions kept one key; an add-on-only key may sit in the app's place.
    let sk = SigningKey::generate(&mut OsRng);
    let app = licensed_app(&sk);
    app.db()
        .set_setting(
            "license_token",
            &keyed(&sk, Some(d(2027, 3, 1)), Some(false)),
        )
        .unwrap();
    let s = app.license_status(d(2026, 12, 20)).unwrap();
    assert_eq!((s.state.as_str(), s.plaid_active), ("expired", true));
    let s = app
        .activate_license(&keyed(&sk, None, Some(true)), d(2026, 12, 20))
        .unwrap();
    assert_eq!((s.state.as_str(), s.plaid_active), ("licensed", true));
    assert!(app.license_status(d(2027, 2, 1)).unwrap().plaid_active);
}
