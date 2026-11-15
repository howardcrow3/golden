//! Tauri shell: a thin command layer over the pure-Rust crates (all logic and tests live there).
use fin_plan::estimate::{estimate, TaxEstimate, TaxRequest};

#[tauri::command]
fn estimate_tax(req: TaxRequest) -> Result<TaxEstimate, String> {
    estimate(&req)
}

#[tauri::command]
fn app_info() -> serde_json::Value {
    serde_json::json!({ "name": "Golden", "version": env!("CARGO_PKG_VERSION"), "mode": "private" })
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![estimate_tax, app_info])
        .run(tauri::generate_context!())
        .expect("error while running Golden");
}
