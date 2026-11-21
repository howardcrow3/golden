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
async fn whatif(app: tauri::AppHandle, text: String, runs: usize) -> Result<WhatIfReply, String> {
    // Reading the question can take seconds when the built-in model answers, so none of this
    // runs on the async runtime's own threads.
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Mutex<Core>>();
        let prep = lock(&state)?.prepare_whatif(&text, today())?;
        prep.run(runs)
    })
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
fn set_opening_balance(state: Shared, account_id: i64, cents: i64) -> Result<(), String> {
    lock(&state)?.set_opening_balance(account_id, cents)
}

#[tauri::command]
fn recurring(
    state: Shared,
    from: String,
    to: String,
) -> Result<Vec<fin_app::recurring::Recurring>, String> {
    lock(&state)?.recurring(&from, &to)
}

#[tauri::command]
fn debt_plan(state: Shared, extra_cents: i64) -> Result<fin_app::debts::DebtPlan, String> {
    lock(&state)?.debt_plan(extra_cents)
}

#[tauri::command]
fn set_debt_terms(
    state: Shared,
    name: String,
    apr_bp: i64,
    min_cents: i64,
) -> Result<(), String> {
    lock(&state)?.set_debt_terms(&name, apr_bp, min_cents)
}

#[tauri::command]
fn insights(state: Shared) -> Result<Vec<fin_app::insights::Insight>, String> {
    lock(&state)?.insights()
}

#[tauri::command]
fn model_settings(state: Shared) -> Result<fin_app::localmodel::ModelSettings, String> {
    lock(&state)?.model_settings()
}

#[tauri::command]
fn set_model(state: Shared, endpoint: String, name: String) -> Result<fin_app::localmodel::ModelSettings, String> {
    lock(&state)?.set_model(&endpoint, &name)
}

fn models_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?.join("models"))
}

/// Adapts the built-in model to the what-if chat.
struct EngineCompleter(fin_ai::Engine);

impl fin_app::model::Completer for EngineCompleter {
    fn complete(&self, prompt: &str) -> Result<String, String> {
        self.0
            .run_task(&fin_ai::task::WHATIF, &[("input", prompt)])
            .map(|o| o.raw)
            .map_err(|e| e.to_string())
    }
}

/// Start using the downloaded model for the what-if chat, or stop when it is not on disk.
fn attach_model(app: &tauri::AppHandle) {
    let completer = models_dir(app).ok().and_then(|dir| {
        let catalog = fin_ai::Catalog::builtin();
        let id = fin_ai::manager::status(&catalog, fin_ai::hardware::total_ram_gb(), &dir).ok()?.id;
        let path = fin_ai::manager::installed_path(&catalog, &id, &dir)?;
        let engine = fin_ai::Engine::new(fin_ai::llama::LlamaCppBackend::new(path), fin_ai::engine::IDLE_UNLOAD);
        Some(std::sync::Arc::new(EngineCompleter(engine)) as std::sync::Arc<dyn fin_app::model::Completer + Sync>)
    });
    let state = app.state::<Mutex<Core>>();
    if let Ok(mut core) = state.lock() {
        core.set_embedded_model(completer);
    };
}

#[tauri::command]
fn model_status(app: tauri::AppHandle) -> Result<fin_ai::manager::ModelStatus, String> {
    fin_ai::manager::status(&fin_ai::Catalog::builtin(), fin_ai::hardware::total_ram_gb(), &models_dir(&app)?)
}

#[tauri::command]
fn start_model_download(
    app: tauri::AppHandle,
    state: State<'_, std::sync::Arc<fin_ai::manager::DownloadState>>,
    id: String,
) -> Result<(), String> {
    let entry = fin_ai::Catalog::builtin().get(&id).cloned().ok_or("Unknown model.")?;
    state.begin(&entry)?;
    let dir = models_dir(&app)?;
    let handle = app.clone();
    let st = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        st.run(&entry, &dir).await;
        attach_model(&handle);
    });
    Ok(())
}

#[tauri::command]
fn model_progress(state: State<'_, std::sync::Arc<fin_ai::manager::DownloadState>>) -> fin_ai::manager::DownloadProgress {
    state.progress()
}

#[tauri::command]
fn cancel_model_download(state: State<'_, std::sync::Arc<fin_ai::manager::DownloadState>>) {
    state.cancel();
}

#[tauri::command]
fn remove_model(app: tauri::AppHandle, id: String) -> Result<(), String> {
    fin_ai::manager::remove(&fin_ai::Catalog::builtin(), &id, &models_dir(&app)?)?;
    attach_model(&app);
    Ok(())
}

#[tauri::command]
fn setup_progress(state: Shared) -> Result<fin_app::setup::Setup, String> {
    lock(&state)?.setup_progress()
}

#[tauri::command]
fn goals(state: Shared) -> Result<Vec<fin_app::goals::GoalRow>, String> {
    lock(&state)?.goals()
}

#[tauri::command]
fn set_goal(state: Shared, goal: fin_app::goals::Goal) -> Result<(), String> {
    lock(&state)?.set_goal(goal)
}

#[tauri::command]
fn remove_goal(state: Shared, name: String) -> Result<(), String> {
    lock(&state)?.remove_goal(&name)
}

#[tauri::command]
fn net_worth_history(
    state: Shared,
    months: u32,
) -> Result<Vec<fin_app::history::NetWorthPoint>, String> {
    lock(&state)?.net_worth_history(months)
}

#[tauri::command]
fn withholding_check(
    state: Shared,
) -> Result<fin_app::withholding::WithholdingCheck, String> {
    lock(&state)?.withholding_check()
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
fn app_info(app: tauri::AppHandle) -> serde_json::Value {
    // The version comes from the bundle (CI stamps it 0.1.<build>); the build number and commit
    // are baked in at compile time and read "dev" for a local build.
    serde_json::json!({
        "name": "Golden",
        "version": app.package_info().version.to_string(),
        "build": option_env!("GOLDEN_BUILD").unwrap_or("dev"),
        "commit": option_env!("GOLDEN_COMMIT").unwrap_or("dev"),
        "mode": "private",
    })
}

fn run_mcp() -> Result<(), String> {
    let dir = fin_app::mcp::data_dir(std::env::consts::OS, "com.highturtle.golden", |k| std::env::var(k).ok())
        .ok_or("could not find the Golden data folder")?;
    if !dir.join("golden.db").exists() {
        return Err("Golden has no data yet. Open the app and import a statement first.".into());
    }
    let db = open_private(&dir.join("golden.db"), &OsKeyStore)?;
    let app = Core::new(db).with_public_key(bundled_public_key());
    let stdin = std::io::stdin();
    fin_app::mcp::serve(&app, stdin.lock(), std::io::stdout()).map_err(|e| e.to_string())
}

pub fn run() {
    // `golden --mcp` serves the read-only assistant protocol on stdin/stdout instead of opening
    // a window, using the same encrypted database as the app.
    if std::env::args().any(|a| a == "--mcp") {
        if let Err(e) = run_mcp() {
            eprintln!("golden --mcp: {e}");
            std::process::exit(1);
        }
        return;
    }
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db = open_private(&dir.join("golden.db"), &OsKeyStore)?;
            let mut core = Core::new(db)
                .with_public_key(bundled_public_key())
                .with_library(Some(fin_app::library::default_root(&dir)));
            // Use the local language model for the what-if chat when one was set up.
            core.refresh_interpreter();
            app.manage(Mutex::new(core));
            app.manage(std::sync::Arc::new(fin_ai::manager::DownloadState::default()));
            attach_model(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            estimate_tax,
            quick_plan,
            app_info,
            setup_progress,
            model_settings,
            model_status,
            start_model_download,
            model_progress,
            cancel_model_download,
            remove_model,
            set_model,
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
            withholding_check,
            net_worth_history,
            goals,
            set_goal,
            remove_goal,
            insights,
            debt_plan,
            set_debt_terms,
            recurring,
            set_opening_balance,
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
