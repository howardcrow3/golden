//! Tauri shell: a thin command layer over the pure-Rust crates (all logic and tests live there).
use fin_app::{AccountRow, App as Core, CategoryRow, ImportReport, PeriodRow};
use std::sync::Mutex;
use tauri::{Manager, State};

type Shared<'a> = State<'a, Mutex<Core>>;

fn lock<'a>(s: &'a Shared<'_>) -> Result<std::sync::MutexGuard<'a, Core>, String> {
    s.lock().map_err(|_| "the data store is busy; try again".to_string())
}

#[tauri::command]
fn import_statement(state: Shared, bytes: Vec<u8>, account: String, kind: String) -> Result<ImportReport, String> {
    lock(&state)?.import_statement(&bytes, &account, &kind)
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
            let db = fin_db::Db::open(dir.join("golden.db"))?;
            app.manage(Mutex::new(Core::new(db)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            estimate_tax,
            quick_plan,
            app_info,
            import_statement,
            accounts,
            cash_flow,
            categories
        ])
        .run(tauri::generate_context!())
        .expect("error while running Golden");
}
