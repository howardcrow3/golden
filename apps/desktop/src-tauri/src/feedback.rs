//! "Send feedback": signs what the person wrote and posts it to the High Turtle feedback service.
//! The rules and signature live in `fin_license::feedback`; this owns the HTTP call.
use fin_license::feedback::{body, key_from_hex, key_to_hex, new_key, sign, APP_ID, FEEDBACK_URL};

/// This install's signing key: read from the OS keychain, or made and saved on first use. If the
/// keychain refuses, the key lasts only until the app closes, which still works but means the
/// service sees a new install next time.
fn install_key() -> ed25519_dalek::SigningKey {
    if let Ok(entry) = keyring::Entry::new("studio.turtlebox.golden", "feedback-install-key") {
        if let Some(k) = entry.get_password().ok().and_then(|h| key_from_hex(&h)) {
            return k;
        }
        let k = new_key();
        if let Err(e) = entry.set_password(&key_to_hex(&k)) {
            log::warn!("could not save the feedback key to the keychain: {e}");
        }
        return k;
    }
    new_key()
}

#[derive(serde::Deserialize)]
pub struct FeedbackInput {
    rating: Option<u8>,
    #[serde(default)]
    message: String,
    category: Option<String>,
    email: Option<String>,
    /// "Send anonymously": no email is sent, and the service keeps none.
    #[serde(default)]
    anonymous: bool,
}

#[tauri::command]
pub fn feedback_available() -> bool {
    // No build-time secret is needed: each install signs with its own key.
    true
}

#[tauri::command]
pub async fn send_feedback(app: tauri::AppHandle, input: FeedbackInput) -> Result<(), String> {
    let platform = if cfg!(target_os = "windows") { "windows" } else if cfg!(target_os = "macos") { "macos" } else { "linux" };
    let version = app.package_info().version.to_string();
    let payload = body(input.rating, &input.message, input.category.as_deref(), input.email.as_deref(), input.anonymous, &version, platform)?;
    tauri::async_runtime::spawn_blocking(move || {
        let ts = chrono::Utc::now().timestamp();
        let (install, sig) = sign(&install_key(), ts, &payload);
        let res = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| e.to_string())?
            .post(FEEDBACK_URL)
            .header("content-type", "application/json")
            .header("x-ht-app", APP_ID)
            .header("x-ht-install", install)
            .header("x-ht-timestamp", ts.to_string())
            .header("x-ht-signature", sig)
            .body(payload)
            .send()
            .map_err(|_| "Couldn't reach the feedback service. Check your internet connection and try again.".to_string())?;
        match res.status().as_u16() {
            200..=299 => Ok(()),
            429 => Err("You've sent a lot of feedback recently. Please try again later.".into()),
            400 => Err(res
                .json::<serde_json::Value>()
                .ok()
                .and_then(|v| v["error"].as_str().map(str::to_string))
                .unwrap_or_else(|| "The feedback couldn't be sent.".into())),
            code => Err(format!("The feedback service answered {code}. Please try again later.")),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
