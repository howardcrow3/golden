use fin_app::App;
use fin_core::NaiveDate;

const CSV: &str =
    "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n10/06/2026,CAFE,-18.40\n";

fn app() -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap())
}

#[test]
fn needs_the_confirmation_word() {
    let mut a = app();
    a.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    assert!(a.erase_all_data("").is_err());
    assert!(a.erase_all_data("yes").is_err());
    assert_eq!(a.accounts().unwrap().len(), 1, "nothing was erased");
}

#[test]
fn erases_ledger_household_and_settings_but_not_the_trial() {
    let day = NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();
    let mut a = app();
    a.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    a.set_model("http://127.0.0.1:11434/api/generate", "llama3")
        .unwrap();
    let before = a.license_status(day).unwrap();
    a.erase_all_data("DELETE").unwrap();
    assert!(a.accounts().unwrap().is_empty());
    assert!(a
        .transactions("2000-01-01", "2030-01-01", false)
        .unwrap()
        .is_empty());
    assert!(a.model_settings().unwrap().endpoint.is_empty());
    let after = a.license_status(day).unwrap();
    assert_eq!(
        format!("{before:?}"),
        format!("{after:?}"),
        "the trial clock must not restart"
    );
    // The app still works afterwards.
    a.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    assert_eq!(a.accounts().unwrap().len(), 1);
}

#[test]
fn works_even_when_the_trial_has_ended() {
    let mut a = app();
    a.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    let mut late = a.with_clock(|| NaiveDate::from_ymd_opt(2027, 6, 1).unwrap());
    // Locked to read-only, but people can always delete their own data.
    late.erase_all_data("DELETE").unwrap();
}
