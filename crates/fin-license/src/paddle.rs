//! The seller side of a Paddle purchase: check that a webhook really came from Paddle, then
//! turn a completed transaction into a signed licence. Pure functions with no network, so a
//! small hosted function (see docs/signing-and-licensing-setup.md) can call them and be sure
//! of what it does.
use crate::{issue, LicensePayload, APP_MAJOR};
use ed25519_dalek::SigningKey;
use fin_core::NaiveDate;
use hmac::{Hmac, Mac};
use serde::Serialize;
use serde_json::Value;
use sha2::Sha256;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WebhookError {
    #[error("the Paddle-Signature header is missing or malformed")]
    BadHeader,
    #[error("the webhook signature does not match")]
    BadSignature,
    #[error("the webhook is too old or from the future (replay protection)")]
    Stale,
    #[error("the webhook body is not the JSON Paddle sends: {0}")]
    BadBody(String),
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// The signature Paddle would send for `body` at `ts` (used by tests and for local replays).
pub fn sign(secret: &str, ts: i64, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("any key length");
    mac.update(format!("{ts}:").as_bytes());
    mac.update(body);
    format!("ts={ts};h1={}", hex(&mac.finalize().into_bytes()))
}

/// Check a `Paddle-Signature` header (`ts=...;h1=...`) against the raw request body.
/// `tolerance_secs` bounds how far the timestamp may be from `now_unix`.
pub fn verify_signature(
    header: &str,
    body: &[u8],
    secret: &str,
    now_unix: i64,
    tolerance_secs: i64,
) -> Result<(), WebhookError> {
    let mut ts = None;
    let mut sigs = vec![];
    for part in header.split(';') {
        match part.trim().split_once('=') {
            Some(("ts", v)) => ts = v.parse::<i64>().ok(),
            Some(("h1", v)) => sigs.push(v.to_string()),
            _ => {}
        }
    }
    let ts = ts.ok_or(WebhookError::BadHeader)?;
    if sigs.is_empty() {
        return Err(WebhookError::BadHeader);
    }
    // abs_diff cannot overflow, so an attacker's ts=-9223372036854775808 is just stale.
    if now_unix.abs_diff(ts) > tolerance_secs.max(0).unsigned_abs() {
        return Err(WebhookError::Stale);
    }
    // Paddle may send more than one h1 while a secret is being rotated; any match is enough.
    for h1 in sigs {
        let Some(want) = unhex(&h1) else { continue };
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("any key length");
        mac.update(format!("{ts}:").as_bytes());
        mac.update(body);
        if mac.verify_slice(&want).is_ok() {
            return Ok(());
        }
    }
    Err(WebhookError::BadSignature)
}

/// Which Paddle prices mean what.
#[derive(Debug, Clone)]
pub struct PriceIds {
    pub core: String,
    /// The monthly Plaid add-on.
    pub plaid: String,
    /// The yearly Plaid add-on, when one is sold. Treated the same as `plaid`.
    pub plaid_annual: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issued {
    /// The licence token to give the buyer.
    pub token: String,
    /// Paddle customer ID (ctm_...), also stored in the licence as the opaque buyer reference.
    pub customer_id: String,
    /// The transaction (txn_...) this licence is for, so a retried webhook is recognised.
    pub transaction_id: String,
    pub plaid_until: Option<NaiveDate>,
    pub bought_core: bool,
}

fn s<'a>(v: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut cur = v;
    for p in path {
        cur = cur.get(*p)?;
    }
    cur.as_str()
}

fn day(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.get(..10)?, "%Y-%m-%d").ok()
}

/// Turn a verified webhook body into a licence. Returns `Ok(None)` for events that are not a
/// completed purchase of one of our prices (Paddle sends many event types; ignoring the rest
/// is correct, so the endpoint can answer 200 and Paddle won't retry).
///
/// A Plaid item sets `plaid_until` to the end of the billing period (or one month after the
/// purchase when Paddle gives none), so each monthly renewal yields a fresh token with a later date.
pub fn licence_from_event(
    body: &str,
    prices: &PriceIds,
    key: &SigningKey,
) -> Result<Option<Issued>, WebhookError> {
    let v: Value = serde_json::from_str(body).map_err(|e| WebhookError::BadBody(e.to_string()))?;
    if s(&v, &["event_type"]) != Some("transaction.completed") {
        return Ok(None);
    }
    let data = v
        .get("data")
        .ok_or_else(|| WebhookError::BadBody("no data".into()))?;
    let mut core = false;
    let mut plaid = false;
    for item in data["items"].as_array().into_iter().flatten() {
        let id = s(item, &["price", "id"]).or_else(|| s(item, &["price_id"]));
        match id {
            Some(id) if id == prices.core => core = true,
            Some(id) if id == prices.plaid => plaid = true,
            Some(id) if prices.plaid_annual.as_deref() == Some(id) => plaid = true,
            _ => {}
        }
    }
    if !core && !plaid {
        return Ok(None);
    }
    let transaction_id = s(data, &["id"])
        .ok_or_else(|| WebhookError::BadBody("no transaction id".into()))?
        .to_string();
    let customer_id = s(data, &["customer_id"])
        .ok_or_else(|| WebhookError::BadBody("no customer id".into()))?
        .to_string();
    let when = s(&v, &["occurred_at"])
        .or_else(|| s(data, &["billed_at"]))
        .and_then(day)
        .ok_or_else(|| WebhookError::BadBody("no date".into()))?;
    let plaid_until = plaid.then(|| {
        s(data, &["billing_period", "ends_at"])
            .and_then(day)
            .unwrap_or_else(|| when.checked_add_months(chrono_months(1)).unwrap_or(when))
    });
    let token = issue(
        key,
        &LicensePayload {
            product: "golden".into(),
            major: APP_MAJOR,
            issued: when,
            plaid_until,
            buyer: customer_id.clone(),
        },
    );
    Ok(Some(Issued {
        token,
        customer_id,
        transaction_id,
        plaid_until,
        bought_core: core,
    }))
}

fn chrono_months(n: u32) -> fin_core::Months {
    fin_core::Months::new(n)
}
