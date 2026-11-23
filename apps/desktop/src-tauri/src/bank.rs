//! Bank connections (the Plaid add-on): the HTTPS transport to Golden's sync service, the keychain
//! store for access tokens, and the commands the screen calls. The logic is in `fin_app::bank`.
use std::sync::Mutex;

use fin_app::bank::{BankOutcome, BankSyncReport, BankTransport, SecretStore};
use fin_app::App as Core;
use fin_plaid::LinkedItem;
use serde_json::Value;
use tauri::Manager;

/// Sent to the screen when the person comes back from their bank's sign-in page.
pub const LINK_EVENT: &str = "plaid-link";

/// Where the sync service lives, set at build time (see services/plaid-server/README.md).
fn service_base() -> Result<&'static str, String> {
    option_env!("GOLDEN_PLAID_SERVICE_URL")
        .filter(|u| u.starts_with("https://"))
        .ok_or_else(|| "Bank connections aren't open in this build yet.".to_string())
}

struct Https;

impl BankTransport for Https {
    fn post(&self, path: &str, body: &Value) -> Result<(u16, String), String> {
        let url = format!("{}{}", service_base()?.trim_end_matches('/'), path);
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .post(url)
            .json(body)
            .send()
            .map_err(|_| "Couldn't reach the bank sync service. Check your internet connection.".to_string())?;
        let status = resp.status().as_u16();
        let text = resp.text().map_err(|e| e.to_string())?;
        Ok((status, text))
    }
}

/// Access tokens live in the macOS Keychain / Windows Credential Manager, one entry per bank.
struct Keychain;

impl Keychain {
    fn entry(key: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new("app.golden.desktop", key).map_err(|e| e.to_string())
    }
}

impl SecretStore for Keychain {
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        match Self::entry(key)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        Self::entry(key)?.set_password(value).map_err(|e| e.to_string())
    }
    fn delete(&self, key: &str) -> Result<(), String> {
        match Self::entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

fn lock<'a>(s: &'a tauri::State<'_, Mutex<Core>>) -> Result<std::sync::MutexGuard<'a, Core>, String> {
    s.lock().map_err(|_| "the data store is busy; try again".to_string())
}

/// The daily background sync: every active bank, quietly. Errors (no network, licence lapsed,
/// a bank needing sign-in) are left for the Banks page, which shows them when the person syncs.
pub fn sync_quietly(core: &mut Core) {
    if core.bank_items().map(|i| i.iter().any(|b| b.status == "active")).unwrap_or(false) {
        let _ = core.bank_sync_all(&Https, &Keychain);
    }
}

#[tauri::command]
pub fn bank_items(state: tauri::State<'_, Mutex<Core>>) -> Result<Vec<LinkedItem>, String> {
    lock(&state)?.bank_items()
}

/// Ask the sync service for a sign-in token and open the sign-in page in the browser. The page
/// sends the person back to Golden with a `golden://plaid` link when they finish.
#[tauri::command]
pub async fn bank_connect(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let base = service_base()?;
        let state = app.state::<Mutex<Core>>();
        let token = lock(&state)?.bank_link_token(&Https)?;
        crate::open_in_browser(&fin_plaid::link::page_url(base, &token))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn bank_finish_link(
    app: tauri::AppHandle,
    public_token: String,
    institution: String,
) -> Result<BankSyncReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Mutex<Core>>();
        let mut core = lock(&state)?;
        core.bank_finish_link(&Https, &Keychain, &public_token, &institution)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn bank_sync_all(app: tauri::AppHandle) -> Result<Vec<BankOutcome>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Mutex<Core>>();
        let mut core = lock(&state)?;
        core.bank_sync_all(&Https, &Keychain)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn bank_set_paused(state: tauri::State<'_, Mutex<Core>>, item_id: String, paused: bool) -> Result<(), String> {
    lock(&state)?.bank_set_paused(&item_id, paused)
}

#[tauri::command]
pub async fn bank_unlink(app: tauri::AppHandle, item_id: String) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Mutex<Core>>();
        let mut core = lock(&state)?;
        core.bank_unlink(&Https, &Keychain, &item_id)
    })
    .await
    .map_err(|e| e.to_string())?
}
