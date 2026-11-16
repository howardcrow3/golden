//! Tauri shell: a thin command layer over the pure-Rust crates (all logic and tests live there).
use fin_app::budget::BudgetRow;
use fin_app::chat::{Change, WhatIfReply};
use fin_app::household::{AssumptionRow, Household};
use fin_app::licensing::LicenseStatus;
use fin_app::transactions::{Recategorized, TxnRow};
use fin_app::{AccountRow, App as Core, CategoryRow, ImportReport, PeriodRow};
use fin_app::keystore::{open_private, KeyStore};
use std::sync::Mutex;
use tauri::{Manager, State};

/// The database key lives in the macOS Keychain / Windows Credential Manager.
struct OsKeyStore;

impl OsKeyStore {
    fn entry() -> Result<keyring::Entry, String> {
        keyring::Entry::new("app.golden.desktop", "database-key").map_err(|e| e.to_string())
    }
}

impl KeyStore for OsKeyStore {
    fn get(&self) -> Result<Option<String>, String> {
        match Self::entry()?.get_password() {
            Ok(k) => Ok(Some(k)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn set(&self, key: &str) -> Result<(), String> {
        Self::entry()?.set_password(key).map_err(|e| e.to_string())
    }
}

type Shared<'a> = State<'a, Mutex<Core>>;

fn lock<'a>(s: &'a Shared<'_>) -> Result<std::sync::MutexGuard<'a, Core>, String> {
    s.lock().map_err(|_| "the data store is busy; try again".to_string())
}

#[tauri::command]
fn import_statement(state: Shared, bytes: Vec<u8>, account: String, kind: String) -> Result<ImportReport, String> {
    lock(&state)?.import_statement(&bytes, &account, &kind)
}

#[tauri::command]
fn import_file(state: Shared, name: String, bytes: Vec<u8>, account: String, kind: String) -> Result<ImportReport, String> {
    lock(&state)?.import_file(&name, &bytes, &account, &kind)
}

/// Choose the statement library folder ("" goes back to the default); returns the folder now in use.
#[tauri::command]
fn set_library_folder(state: Shared, path: String) -> Result<String, String> {
    lock(&state)?.set_library_folder(&path)
}

#[tauri::command]
fn library_folder(state: Shared) -> Result<Option<String>, String> {
    Ok(lock(&state)?.library_root().map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
fn transactions(state: Shared, from: String, to: String, only_uncategorized: bool) -> Result<Vec<TxnRow>, String> {
    lock(&state)?.transactions(&from, &to, only_uncategorized)
}

#[tauri::command]
fn category_choices(state: Shared) -> Result<Vec<String>, String> {
    lock(&state)?.category_choices()
}

#[tauri::command]
fn set_category(state: Shared, id: i64, category: String, remember: bool) -> Result<Recategorized, String> {
    lock(&state)?.set_category(id, &category, remember)
}

/// Write a CSV of the transactions between two dates into the Exports folder and return its path.
#[tauri::command]
fn export_transactions(app: tauri::AppHandle, state: Shared, from: String, to: String) -> Result<String, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("Exports");
    let path = lock(&state)?.export_transactions(&dir, &from, &to)?;
    Ok(path.to_string_lossy().to_string())
}

/// Write a full JSON copy of everything the person has entered into the Exports folder.
#[tauri::command]
fn export_backup(app: tauri::AppHandle, state: Shared) -> Result<String, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("Exports");
    let path = lock(&state)?.export_backup(&dir)?;
    Ok(path.to_string_lossy().to_string())
}

/// Merge a Golden backup file (as bytes) into this database.
#[tauri::command]
fn restore_backup(state: Shared, bytes: Vec<u8>) -> Result<fin_app::backup::RestoreReport, String> {
    let text = String::from_utf8(bytes).map_err(|_| "That file isn't a Golden backup.".to_string())?;
    lock(&state)?.restore_backup(&text)
}

/// What Golden costs, plus the Paddle price IDs this build was given (see the signing guide).
#[tauri::command]
fn pricing() -> fin_license::pricing::PricingInfo {
    fin_license::pricing::PricingInfo::from_ids(
        option_env!("GOLDEN_PRICE_ID"),
        option_env!("GOLDEN_PLAID_PRICE_ID"),
        option_env!("GOLDEN_CHECKOUT_URL"),
    )
}

/// Open the hosted checkout page for "core" or "plaid" in the person's browser. The link is
/// rebuilt here from the build's configuration and validated (https only), never taken from the UI.
#[tauri::command]
fn open_checkout(plan: String) -> Result<(), String> {
    let info = pricing();
    let url = match plan.as_str() {
        "core" => info.core_checkout_url,
        "plaid" => info.plaid_checkout_url,
        _ => return Err("Unknown purchase option.".into()),
    }
    .ok_or("Purchasing isn't open yet.")?;
    open_in_browser(&url)
}

fn open_in_browser(url: &str) -> Result<(), String> {
    use std::process::Command;
    #[cfg(target_os = "macos")]
    let mut cmd = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut cmd = Command::new("explorer");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = Command::new("xdg-open");
    cmd.arg(url).spawn().map(|_| ()).map_err(|e| format!("Couldn't open your browser: {e}"))
}

#[tauri::command]
fn license_status(state: Shared) -> Result<LicenseStatus, String> {
    lock(&state)?.license_status(today())
}

#[tauri::command]
fn activate_license(state: Shared, token: String) -> Result<LicenseStatus, String> {
    lock(&state)?.activate_license(&token, today())
}

fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}

/// The seller's public key, bundled at build time. Empty until the first key is generated.
fn bundled_public_key() -> Option<fin_license::VerifyingKey> {
    fin_license::parse_public_key(include_str!("../license_public_key.txt")).ok()
}

#[tauri::command]
fn household(state: Shared) -> Result<Household, String> {
    lock(&state)?.household()
}

#[tauri::command]
fn save_household(state: Shared, household: Household) -> Result<(), String> {
    lock(&state)?.save_household(&household)
}

#[tauri::command]
fn set_pick(state: Shared, key: String, label: String) -> Result<Household, String> {
    lock(&state)?.set_pick(&key, &label)
}

#[tauri::command]
fn assumptions(state: Shared) -> Result<Vec<AssumptionRow>, String> {
    lock(&state)?.assumptions()
}

/// Run the saved household through the plan. The lock is released before the CPU-heavy part.
#[tauri::command]
async fn run_plan(state: Shared<'_>, runs: usize) -> Result<QuickPlanResult, String> {
    let (plan, picks) = lock(&state)?.plan(today())?;
    tauri::async_runtime::spawn_blocking(move || quickplan::run_plan(&plan, &picks, runs))
        .await
        .map_err(|e| e.to_string())?
}

/// Interpret a what-if sentence and run the plan with and without it. Nothing is saved.
#[tauri::command]
async fn whatif(state: Shared<'_>, text: String, runs: usize) -> Result<WhatIfReply, String> {
    let prep = lock(&state)?.prepare_whatif(&text, today())?;
    tauri::async_runtime::spawn_blocking(move || prep.run(runs))
        .await
        .map_err(|e| e.to_string())?
}

/// Keep a what-if: save its changes into the household.
#[tauri::command]
fn keep_whatif(state: Shared, changes: Vec<Change>) -> Result<Household, String> {
    lock(&state)?.apply_changes(&changes)
}

#[tauri::command]
fn accounts(state: Shared) -> Result<Vec<AccountRow>, String> {
    lock(&state)?.accounts()
}

#[tauri::command]
fn cash_flow(state: Shared, from: String, to: String, grain: String) -> Result<Vec<PeriodRow>, String> {
    lock(&state)?.cash_flow_str(&from, &to, &grain)
}

#[tauri::command]
fn budget_status(state: Shared, day: String, grain: String) -> Result<Vec<BudgetRow>, String> {
    lock(&state)?.budget_status(&day, &grain)
}

#[tauri::command]
fn set_budget(state: Shared, category: String, monthly_cents: i64) -> Result<(), String> {
    lock(&state)?.set_budget(&category, monthly_cents).map(|_| ())
}

#[tauri::command]
fn balance_sheet(state: Shared) -> Result<fin_app::balance::BalanceSheet, String> {
    lock(&state)?.balance_sheet()
}

#[tauri::command]
fn read_paystub(state: Shared, text: String) -> Result<fin_app::paystub::PaystubSummary, String> {
    lock(&state)?.read_paystub(&text)
}

#[tauri::command]
fn apply_paystub(
    state: Shared,
    text: String,
    person: usize,
) -> Result<fin_app::household::StreamRow, String> {
    lock(&state)?.apply_paystub(&text, person)
}

#[tauri::command]
fn suggest_allocation(
    state: Shared,
    yearly_savings_cents: i64,
) -> Result<fin_app::allocate::Allocation, String> {
    lock(&state)?.suggest_allocation(yearly_savings_cents)
}

#[tauri::command]
fn forecast(state: Shared, months: u32) -> Result<fin_app::forecast::Forecast, String> {
    lock(&state)?.forecast(months)
}

#[tauri::command]
fn categories(state: Shared, day: String, grain: String, income: bool) -> Result<Vec<CategoryRow>, String> {
    lock(&state)?.categories_str(&day, &grain, income)
}

use fin_plan::estimate::{estimate, TaxEstimate, TaxRequest};
use fin_plan::quickplan::{self, QuickPlanRequest, QuickPlanResult};

#[tauri::command]
fn estimate_tax(req: TaxRequest) -> Result<TaxEstimate, String> {
    estimate(&req)
}

#[tauri::command]
async fn quick_plan(req: QuickPlanRequest) -> Result<QuickPlanResult, String> {
    // Monte Carlo is CPU work: keep it off the UI thread.
    tauri::async_runtime::spawn_blocking(move || quickplan::run(&req)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn app_info() -> serde_json::Value {
    serde_json::json!({ "name": "Golden", "version": env!("CARGO_PKG_VERSION"), "mode": "private" })
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db = open_private(&dir.join("golden.db"), &OsKeyStore)?;
            app.manage(Mutex::new(Core::new(db)
                .with_public_key(bundled_public_key())
                .with_library(Some(fin_app::library::default_root(&dir)))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            estimate_tax,
            quick_plan,
            app_info,
            import_statement,
            import_file,
            export_transactions,
            export_backup,
            restore_backup,
            transactions,
            category_choices,
            budget_status,
            forecast,
            balance_sheet,
            read_paystub,
            suggest_allocation,
            apply_paystub,
            set_budget,
            set_category,
            library_folder,
            set_library_folder,
            household,
            save_household,
            set_pick,
            assumptions,
            run_plan,
            whatif,
            keep_whatif,
            pricing,
            open_checkout,
            license_status,
            activate_license,
            accounts,
            cash_flow,
            categories
        ])
        .run(tauri::generate_context!())
        .expect("error while running Golden");
}
