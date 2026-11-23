//! `golden://activate?key=...` links from the purchase email. The operating system starts Golden
//! (or hands the link to the running copy) and the licence is activated with no copy and paste.
//! The link parsing lives in `fin_license::link` (tested there); this only wires it to the app.
use std::sync::Mutex;

use fin_app::App as Core;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

/// Sent to the UI after a link was handled, so it can refresh the trial banner and say what happened.
pub const EVENT: &str = "license-link";

#[derive(Debug, Clone, Serialize)]
pub struct LinkResult {
    pub ok: bool,
    pub message: String,
}

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn activate(app: &AppHandle, token: &str) -> LinkResult {
    let Some(state) = app.try_state::<Mutex<Core>>() else {
        return LinkResult {
            ok: false,
            message: "Golden is still starting. Open the link again in a moment.".into(),
        };
    };
    let result = match state.lock() {
        Ok(core) => core.activate_license(token, crate::today()),
        Err(_) => Err("the data store is busy; try again".to_string()),
    };
    match result {
        Ok(_) => LinkResult {
            ok: true,
            message: "Your licence is active. Thank you!".into(),
        },
        Err(message) => LinkResult { ok: false, message },
    }
}

fn dispatch(app: &AppHandle, urls: Vec<String>) {
    show_main_window(app);
    // A link that is not an activation link only brings the window forward.
    if let Some(token) = fin_license::link::token_from_links(&urls) {
        let result = activate(app, &token);
        let _ = app.emit(EVENT, result);
    }
}

pub fn setup(app: &tauri::App) {
    let handle = app.handle().clone();
    app.deep_link().on_open_url({
        let handle = handle.clone();
        move |event| dispatch(&handle, event.urls().iter().map(|u| u.to_string()).collect())
    });
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        dispatch(&handle, urls.iter().map(|u| u.to_string()).collect());
    }
    // A development run has no installer to register the scheme.
    #[cfg(windows)]
    if cfg!(debug_assertions) {
        let _ = app.deep_link().register_all();
    }
}
