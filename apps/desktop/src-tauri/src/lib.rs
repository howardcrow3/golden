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

mod bank;
mod deep_link;
mod feedback;

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

/// Import every new statement in a folder (OFX/QFX name their own account; CSV uses `account`).
#[tauri::command]
async fn import_folder(app: tauri::AppHandle, path: String, account: String, kind: String) -> Result<fin_app::folder::FolderReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Mutex<Core>>();
        let mut core = lock(&state)?;
        core.import_folder(std::path::Path::new(path.trim()), &account, &kind)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn watched_folder(state: Shared) -> Result<Option<fin_app::watch::Watch>, String> {
    lock(&state)?.watched_folder()
}

/// Start watching a folder; it is scanned once straight away.
#[tauri::command]
fn watch_folder(state: Shared, path: String, account: String, kind: String) -> Result<fin_app::watch::Watch, String> {
    let mut core = lock(&state)?;
    let w = core.watch_folder(&path, &account, &kind)?;
    core.scan_watched();
    Ok(w)
}

#[tauri::command]
fn stop_watching(state: Shared) -> Result<(), String> {
    lock(&state)?.stop_watching()
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

#[tauri::command]
fn set_note_tags(state: Shared, id: i64, note: String, tags: String) -> Result<(), String> {
    lock(&state)?.set_note_tags(id, &note, &tags)
}

#[tauri::command]
fn set_splits(state: Shared, id: i64, parts: Vec<fin_app::transactions::SplitPart>) -> Result<(), String> {
    lock(&state)?.set_splits(id, parts)
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

/// Save a report, table or data file the screen built into the Exports folder.
#[tauri::command]
fn save_report(app: tauri::AppHandle, stem: String, kind: String, content: String) -> Result<String, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("Exports");
    let path = fin_app::report::save_report(&dir, &stem, &kind, &content)?;
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

/// Wrap the command handler so every command the screens run leaves a line in the log: the
/// command's name only, never its arguments, which can hold financial data.
fn logged<R: tauri::Runtime>(
    inner: impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static,
) -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    move |invoke| {
        let cmd = invoke.message.command();
        if cmd != "client_log" {
            log::info!(target: "cmd", "{cmd}");
        }
        inner(invoke)
    }
}

/// A line from the screens: a failed command, an uncaught error or a crash in the page.
#[tauri::command]
fn client_log(level: String, source: String, message: String) {
    fin_log::from_ui(&level, &source, &message);
}

/// Where the log files are kept, for the About page.
#[tauri::command]
fn log_folder() -> Option<String> {
    fin_log::dir().map(|d| d.display().to_string())
}

/// The newest log lines (already scrubbed of amounts and numbers), for "Copy log".
#[tauri::command]
fn log_tail(lines: usize) -> String {
    fin_log::tail(lines.clamp(1, 2000))
}

/// Write why Golden could not start to `Golden-startup-error.txt` in the home folder.
fn write_startup_note(message: &str) {
    log::error!("{message}");
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    if let Some(home) = home {
        let path = std::path::Path::new(&home).join("Golden-startup-error.txt");
        let _ = std::fs::write(path, format!("{message}\n"));
    }
    eprintln!("{message}");
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

/// The most the saved household can spend a year while the plan works in `target` of the trials.
#[tauri::command]
async fn safe_spend(
    state: Shared<'_>,
    target: f64,
) -> Result<quickplan::SafeSpendResult, String> {
    let (plan, picks) = lock(&state)?.plan(today())?;
    tauri::async_runtime::spawn_blocking(move || quickplan::safe_spend(&plan, &picks, target))
        .await
        .map_err(|e| e.to_string())?
}

/// Compare Roth conversion strategies for the saved household.
#[tauri::command]
async fn roth_explore(
    state: Shared<'_>,
    until_age: i32,
) -> Result<Vec<quickplan::RothRow>, String> {
    let (plan, picks) = lock(&state)?.plan(today())?;
    tauri::async_runtime::spawn_blocking(move || quickplan::roth_explore(&plan, &picks, until_age))
        .await
        .map_err(|e| e.to_string())?
}

/// Compare Social Security claiming ages 62 to 70 for the saved household.
#[tauri::command]
async fn ss_explore(state: Shared<'_>) -> Result<quickplan::SsExplore, String> {
    let (plan, picks) = lock(&state)?.plan(today())?;
    tauri::async_runtime::spawn_blocking(move || quickplan::ss_explore(&plan, &picks))
        .await
        .map_err(|e| e.to_string())?
}

/// Replay the saved household's plan through every start year in US market history.
#[tauri::command]
async fn backtest(state: Shared<'_>, stock_share: f64) -> Result<quickplan::BacktestResult, String> {
    let (plan, picks) = lock(&state)?.plan(today())?;
    tauri::async_runtime::spawn_blocking(move || quickplan::backtest(&plan, &picks, stock_share))
        .await
        .map_err(|e| e.to_string())?
}

/// The same replay for several stock/bond mixes, for a side-by-side table.
#[tauri::command]
async fn backtest_compare(state: Shared<'_>, shares: Vec<f64>) -> Result<Vec<quickplan::MixRow>, String> {
    let (plan, picks) = lock(&state)?.plan(today())?;
    tauri::async_runtime::spawn_blocking(move || quickplan::compare_mixes(&plan, &picks, &shares))
        .await
        .map_err(|e| e.to_string())?
}

/// Monte Carlo that redraws blocks of historical returns, for the saved household.
#[tauri::command]
async fn bootstrap(
    state: Shared<'_>,
    stock_share: f64,
    block_years: usize,
    runs: usize,
) -> Result<quickplan::BootstrapResult, String> {
    let (plan, picks) = lock(&state)?.plan(today())?;
    tauri::async_runtime::spawn_blocking(move || {
        quickplan::bootstrap(&plan, &picks, stock_share, block_years, runs)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The earliest retirement age at which the saved household's plan works in `target` of trials.
#[tauri::command]
async fn retire_when(
    state: Shared<'_>,
    target: f64,
) -> Result<fin_app::retire_when::RetireWhen, String> {
    let household = lock(&state)?.household()?;
    tauri::async_runtime::spawn_blocking(move || {
        fin_app::retire_when::scan(&household, today(), target, 200)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Rent or buy: net worth after some years each way.
#[tauri::command]
fn rent_vs_buy(req: fin_plan::rentbuy::Request) -> Result<fin_plan::rentbuy::Outcome, String> {
    fin_plan::rentbuy::run(&req)
}

/// Room to realize gains at a 0% rate, and the distance to the ACA subsidy cliff.
#[tauri::command]
fn harvest_plan(
    filing: fin_plan::tax::FilingStatus,
    income_cents: i64,
    gains_cents: i64,
    household_size: i64,
) -> Result<fin_plan::harvest::HarvestPlan, String> {
    fin_plan::harvest::plan_cents(filing, income_cents, gains_cents, household_size)
}

/// What an RSU vest is worth and how many shares are sold to cover tax.
#[tauri::command]
fn rsu_vest(
    shares: u64,
    price_cents: i64,
    federal_rate: f64,
    other_rate: f64,
) -> Result<fin_plan::equity::VestResult, String> {
    fin_plan::equity::rsu_vest_cents(shares, price_cents, federal_rate, other_rate)
}

/// Shares bought, and the day-one gain, for one ESPP purchase.
#[tauri::command]
fn espp_purchase(
    contributions_cents: i64,
    offering_cents: i64,
    purchase_cents: i64,
    discount: f64,
    lookback: bool,
) -> Result<fin_plan::equity::EsppResult, String> {
    fin_plan::equity::espp_purchase_cents(contributions_cents, offering_cents, purchase_cents, discount, lookback)
}

#[tauri::command]
fn holdings(state: Shared) -> Result<Vec<fin_plan::investments::Holding>, String> {
    lock(&state)?.holdings()
}

#[tauri::command]
fn parse_holdings(state: Shared, text: String) -> Result<Vec<fin_plan::investments::Holding>, String> {
    lock(&state)?.parse_holdings(&text)
}

#[tauri::command]
fn save_holdings(state: Shared, holdings: Vec<fin_plan::investments::Holding>) -> Result<(), String> {
    lock(&state)?.save_holdings(&holdings)
}

#[tauri::command]
fn investment_checkup(
    state: Shared,
    risk: fin_plan::investments::Risk,
) -> Result<fin_plan::investments::Checkup, String> {
    lock(&state)?.investment_checkup(risk)
}

/// The saved plan as the text of a plan file, for the screen to save or send.
#[tauri::command]
fn export_plan_file(state: Shared) -> Result<String, String> {
    lock(&state)?.export_plan_file()
}

/// Replace the saved plan with the contents of a plan file.
#[tauri::command]
fn import_plan_file(state: Shared, text: String) -> Result<fin_app::planfile::Imported, String> {
    lock(&state)?.import_plan_file(&text)
}

/// Put a file in the encrypted document vault.
#[tauri::command]
fn vault_add(state: Shared, name: String, bytes: Vec<u8>) -> Result<fin_app::vault::DocumentRow, String> {
    lock(&state)?.vault_add(&name, &bytes)
}

#[tauri::command]
fn vault_list(state: Shared, query: String) -> Result<Vec<fin_app::vault::DocumentRow>, String> {
    lock(&state)?.vault_list(&query)
}

#[tauri::command]
fn vault_delete(state: Shared, id: i64) -> Result<(), String> {
    lock(&state)?.vault_delete(id)
}

/// Copy a vault document out to the Exports folder and return where it went.
#[tauri::command]
fn vault_export(app: tauri::AppHandle, state: Shared, id: i64) -> Result<String, String> {
    let (name, bytes) = lock(&state)?.vault_get(id)?;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("Exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Keep only the file name part, so a stored name can't point outside the folder.
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') { c } else { '_' })
        .collect();
    let path = dir.join(format!("vault-{id}-{}", safe.trim_start_matches('.')));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

/// Early IRA withdrawals without the 10% penalty (72(t)), by two methods.
#[tauri::command]
fn sepp(balance_cents: i64, age: i32, rate_bp: i64) -> Result<Vec<fin_plan::sepp::Sepp>, String> {
    fin_plan::sepp::calculate_cents(balance_cents, age, rate_bp)
}

/// Whether bunching several years of charity into one donor-advised fund gift saves tax.
#[tauri::command]
fn daf_bunching(
    filing: fin_plan::tax::FilingStatus,
    income_cents: i64,
    other_itemized_cents: i64,
    giving_cents: i64,
    years: i32,
) -> Result<fin_plan::daf::Bunching, String> {
    fin_plan::daf::bunching_cents(filing, income_cents, other_itemized_cents, giving_cents, years)
}

/// Interpret a what-if sentence and run the plan with and without it. Nothing is saved.
#[tauri::command]
async fn whatif(
    app: tauri::AppHandle,
    text: String,
    runs: usize,
    on: Option<u32>,
) -> Result<WhatIfReply, String> {
    // Reading the question can take seconds when the built-in model answers, so none of this
    // runs on the async runtime's own threads.
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Mutex<Core>>();
        let prep = lock(&state)?.prepare_whatif_on(&text, today(), on)?;
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
fn list_scenarios(state: Shared) -> Result<Vec<fin_app::scenarios::SavedScenario>, String> {
    lock(&state)?.list_scenarios()
}

#[tauri::command]
fn save_scenario(state: Shared, name: String, changes: Vec<Change>) -> Result<fin_app::scenarios::SavedScenario, String> {
    lock(&state)?.save_scenario(&name, &changes, today())
}

#[tauri::command]
fn rename_scenario(state: Shared, id: u32, name: String) -> Result<fin_app::scenarios::SavedScenario, String> {
    lock(&state)?.rename_scenario(id, &name)
}

#[tauri::command]
fn duplicate_scenario(state: Shared, id: u32) -> Result<fin_app::scenarios::SavedScenario, String> {
    lock(&state)?.duplicate_scenario(id)
}

#[tauri::command]
fn move_scenario(state: Shared, id: u32, by: i32) -> Result<Vec<fin_app::scenarios::SavedScenario>, String> {
    lock(&state)?.move_scenario(id, by)
}

#[tauri::command]
fn delete_scenario(state: Shared, id: u32) -> Result<(), String> {
    lock(&state)?.delete_scenario(id)
}

/// Run the current plan and up to three saved scenarios side by side.
#[tauri::command]
async fn compare_scenarios(app: tauri::AppHandle, ids: Vec<u32>, runs: usize) -> Result<fin_app::scenarios::CompareReply, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Mutex<Core>>();
        let prep = lock(&state)?.prepare_compare(&ids, today())?;
        prep.run(runs)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn make_plan(state: Shared, id: u32) -> Result<Household, String> {
    lock(&state)?.make_plan(id)
}

#[tauri::command]
fn undoable_plan(state: Shared) -> Result<Option<String>, String> {
    lock(&state)?.undoable_plan()
}

#[tauri::command]
fn undo_make_plan(state: Shared) -> Result<Household, String> {
    lock(&state)?.undo_make_plan()
}

/// Answer a plain-words question about spending, income, savings, net worth or balances.
#[tauri::command]
fn ask(state: Shared, question: String) -> Result<fin_app::ask::Answer, String> {
    lock(&state)?.ask(&question)
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

/// Read a Social Security Statement's text. Changes nothing.
#[tauri::command]
fn read_ssa_statement(
    state: Shared,
    text: String,
) -> Result<fin_app::ssa::SsaSummary, String> {
    lock(&state)?.read_ssa_statement(&text)
}

/// Set a person's yearly Social Security from a statement; `claim` is "62", "fra" or "70".
#[tauri::command]
fn apply_ssa_statement(
    state: Shared,
    text: String,
    person: usize,
    claim: String,
) -> Result<i64, String> {
    lock(&state)?.apply_ssa_statement(&text, person, &claim)
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

/// Raise an operating-system notification. The text is generic on purpose: banners can show
/// on a locked screen, so no amounts or names go into them.
#[tauri::command]
fn os_notify(app: tauri::AppHandle, count: u32) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    let body = if count == 1 {
        "Your plan has a new heads-up. Open Golden to see it."
    } else {
        "Your plan has new heads-ups. Open Golden to see them."
    };
    app.notification()
        .builder()
        .title("Golden")
        .body(body)
        .show()
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CareCost {
    annual_cents: i64,
    national: bool,
}

/// Typical yearly cost of a kind of long-term care in a state (CareScout survey, 2025).
#[tauri::command]
fn care_cost(state: String, kind: String) -> Result<CareCost, String> {
    let care = fin_plan::carecost::Care::from_key(&kind).ok_or("Unknown kind of care.")?;
    let c = fin_plan::carecost::cost(&state, care);
    Ok(CareCost {
        annual_cents: c.annual.cents(),
        national: c.national_fallback,
    })
}

#[tauri::command]
fn mortgage_offset(
    balance_cents: i64,
    rate_bp: i64,
    years_left: i32,
    offset_cents: i64,
) -> Result<fin_app::offset::OffsetEffect, String> {
    fin_app::offset::mortgage_offset(balance_cents, rate_bp, years_left, offset_cents)
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
fn month_close_check(state: Shared, month: String) -> Result<fin_app::close::CloseReport, String> {
    lock(&state)?.month_close_check(&month)
}

#[tauri::command]
fn close_month(state: Shared, month: String) -> Result<fin_app::close::CloseReport, String> {
    lock(&state)?.close_month(&month)
}

#[tauri::command]
fn reopen_month(state: Shared, month: String) -> Result<fin_app::close::CloseReport, String> {
    lock(&state)?.reopen_month(&month)
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
        self.complete_task("whatif", prompt)
    }

    fn complete_task(&self, task: &str, prompt: &str) -> Result<String, String> {
        let spec = fin_ai::task::by_id(task).ok_or_else(|| format!("unknown model task {task}"))?;
        self.0
            .run_task(spec, &[("input", prompt)])
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

/// Ask the model to suggest categories for lines that have none. Off the async runtime, since a
/// model can take seconds.
#[tauri::command]
async fn suggest_categories(app: tauri::AppHandle) -> Result<Vec<fin_app::ai::CategorySuggestion>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Mutex<Core>>();
        let core = lock(&state)?;
        core.suggest_categories()
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Explain a plan result in plain English, from a few rounded facts only.
#[tauri::command]
async fn explain_result(
    app: tauri::AppHandle,
    facts: fin_app::explain::ExplainFacts,
) -> Result<fin_app::explain::Explanation, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Mutex<Core>>();
        let core = lock(&state)?;
        Ok(core.explain_result(&facts))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Suggest which assumption options suit this household. Nothing is saved.
#[tauri::command]
async fn suggest_assumptions(app: tauri::AppHandle) -> Result<Vec<fin_app::suggest::AssumptionSuggestion>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Mutex<Core>>();
        let core = lock(&state)?;
        core.suggest_assumptions()
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Every model run this session (timings and sizes only, never the text).
#[tauri::command]
fn ai_activity(state: Shared) -> Result<Vec<fin_app::activity::ActivityEntry>, String> {
    Ok(lock(&state)?.ai_activity())
}

#[tauri::command]
fn clear_ai_activity(state: Shared) -> Result<(), String> {
    lock(&state)?.clear_ai_activity();
    Ok(())
}

/// Delete everything in one step. The caller must send the word DELETE.
#[tauri::command]
fn erase_all_data(state: Shared, confirm: String) -> Result<(), String> {
    lock(&state)?.erase_all_data(&confirm)
}

#[tauri::command]
fn connections(state: Shared) -> Result<fin_app::connections::Connections, String> {
    lock(&state)?.connections()
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

/// Start (or restart) tracking real net worth against the plan's projection.
#[tauri::command]
fn start_progress(state: Shared) -> Result<(), String> {
    lock(&state)?.start_progress_tracking()
}

#[tauri::command]
fn stop_progress(state: Shared) -> Result<(), String> {
    lock(&state)?.stop_progress_tracking()
}

#[tauri::command]
fn add_journal_entry(state: Shared, date: String, savings_cents: i64, note: String) -> Result<(), String> {
    lock(&state)?.add_journal_entry(&date, savings_cents, &note)
}

#[tauri::command]
fn remove_journal_entry(state: Shared, date: String) -> Result<(), String> {
    lock(&state)?.remove_journal_entry(&date)
}

#[tauri::command]
fn progress(state: Shared) -> Result<Option<fin_app::progress::ProgressReport>, String> {
    lock(&state)?.progress()
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
    // A crash at start-up (a panic) is written down too.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        write_startup_note(&format!("Golden crashed: {info}"));
        default_hook(info);
    }));
    tauri::Builder::default()
        // Single instance comes first so a second launch (a golden:// link) is handed to the running copy.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            // The local log comes first so everything after it, including a failed start, is written down.
            let verbose = std::env::var("GOLDEN_LOG").map_or(false, |v| v == "debug");
            if let Err(e) = fin_log::init(&dir.join("Logs"), verbose) {
                eprintln!("could not start the log: {e}");
            }
            let db = open_private(&dir.join("golden.db"), &OsKeyStore)?;
            let mut core = Core::new(db)
                .with_public_key(bundled_public_key())
                .with_library(Some(fin_app::library::default_root(&dir)));
            // Use the local language model for the what-if chat when one was set up.
            core.refresh_interpreter();
            // Read scans and photos in the document vault with the operating system's own OCR.
            core.set_ocr(fin_ocr::system_engine());
            app.manage(Mutex::new(core));
            app.manage(std::sync::Arc::new(fin_ai::manager::DownloadState::default()));
            attach_model(app.handle());
            deep_link::setup(app);
            // Look in the watched folder (if any) for new statements every few minutes.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut last_bank_sync = std::time::Instant::now();
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(300));
                    let state = handle.state::<Mutex<Core>>();
                    if let Ok(mut core) = state.lock() {
                        core.scan_watched();
                        // Linked banks refresh about once a day while Golden is open.
                        if last_bank_sync.elapsed() >= std::time::Duration::from_secs(24 * 3600) {
                            last_bank_sync = std::time::Instant::now();
                            bank::sync_quietly(&mut core);
                        }
                    };
                }
            });
            Ok(())
            })();
            // If Golden can't start, leave a note the person can send instead of closing silently.
            if let Err(e) = &result {
                write_startup_note(&format!("Golden could not start: {e}"));
            }
            result
        })
        .invoke_handler(logged(tauri::generate_handler![
            feedback::feedback_available,
            feedback::send_feedback,
            bank::bank_items,
            bank::bank_connect,
            bank::bank_finish_link,
            bank::bank_sync_all,
            bank::bank_set_paused,
            bank::bank_unlink,
            estimate_tax,
            quick_plan,
            app_info,
            setup_progress,
            model_settings,
            model_status,
            suggest_categories,
            explain_result,
            suggest_assumptions,
            ai_activity,
            connections,
            erase_all_data,
            import_folder,
            watched_folder,
            watch_folder,
            stop_watching,
            ask,
            list_scenarios,
            save_scenario,
            delete_scenario,
            rename_scenario,
            duplicate_scenario,
            move_scenario,
            compare_scenarios,
            make_plan,
            undoable_plan,
            undo_make_plan,
            clear_ai_activity,
            start_model_download,
            model_progress,
            cancel_model_download,
            remove_model,
            set_model,
            import_statement,
            import_file,
            export_transactions,
            export_backup,
            save_report,
            restore_backup,
            transactions,
            category_choices,
            budget_status,
            forecast,
            balance_sheet,
            withholding_check,
            net_worth_history,
            start_progress,
            stop_progress,
            add_journal_entry,
            remove_journal_entry,
            progress,
            goals,
            set_goal,
            remove_goal,
            insights,
            month_close_check,
            close_month,
            reopen_month,
            debt_plan,
            mortgage_offset,
            care_cost,
            os_notify,
            set_debt_terms,
            recurring,
            set_opening_balance,
            read_paystub,
            read_ssa_statement,
            apply_ssa_statement,
            suggest_allocation,
            apply_paystub,
            set_budget,
            set_category,
            set_note_tags,
            set_splits,
            library_folder,
            set_library_folder,
            household,
            save_household,
            set_pick,
            assumptions,
            run_plan,
            safe_spend,
            retire_when,
            roth_explore,
            sepp,
            vault_add,
            vault_list,
            vault_delete,
            vault_export,
            export_plan_file,
            import_plan_file,
            holdings,
            save_holdings,
            parse_holdings,
            investment_checkup,
            harvest_plan,
            rsu_vest,
            espp_purchase,
            rent_vs_buy,
            daf_bunching,
            ss_explore,
            backtest,
            backtest_compare,
            bootstrap,
            whatif,
            keep_whatif,
            pricing,
            open_checkout,
            license_status,
            activate_license,
            accounts,
            cash_flow,
            categories,
            client_log,
            log_folder,
            log_tail
        ]))
        .run(tauri::generate_context!())
        .expect("error while running Golden");
}
