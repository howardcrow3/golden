//! The requests Golden sends to the sync service, and the answers it expects. The service adds
//! Plaid's client id and secret, so none of that is ever in the app. Every request carries the
//! person's licence token so the service can check the Plaid add-on is paid.
use serde::Deserialize;
use serde_json::{json, Value};

use crate::PlaidError;

/// The paths the service answers, and the only ones the app will ever call.
pub const PATH_LINK_TOKEN: &str = "/link/token";
pub const PATH_EXCHANGE: &str = "/link/exchange";
pub const PATH_SYNC: &str = "/transactions/sync";
pub const PATH_REMOVE: &str = "/item/remove";

pub fn link_token_body(licence: &str, client_user_id: &str) -> Value {
    json!({ "licence": licence, "client_user_id": client_user_id })
}

pub fn exchange_body(licence: &str, public_token: &str) -> Value {
    json!({ "licence": licence, "public_token": public_token })
}

pub fn sync_body(licence: &str, access_token: &str, cursor: Option<&str>) -> Value {
    let mut v = json!({ "licence": licence, "access_token": access_token });
    if let Some(c) = cursor.filter(|c| !c.is_empty()) {
        v["cursor"] = json!(c);
    }
    v
}

pub fn remove_body(licence: &str, access_token: &str) -> Value {
    json!({ "licence": licence, "access_token": access_token })
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LinkToken {
    pub link_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Exchanged {
    pub access_token: String,
    pub item_id: String,
}

/// The service passes Plaid's error body through with a non-200 status. Read it.
pub fn parse_error(body: &str) -> PlaidError {
    match serde_json::from_str::<Value>(body) {
        Ok(v) => {
            let code = v["error_code"].as_str().unwrap_or("UNKNOWN");
            let msg = v["error_message"]
                .as_str()
                .or_else(|| v["error"].as_str())
                .unwrap_or("no details");
            PlaidError::from_code(code, msg)
        }
        Err(_) => PlaidError::Malformed(body.chars().take(120).collect()),
    }
}

/// Read a good answer into `T`.
pub fn parse_ok<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, PlaidError> {
    serde_json::from_str(body).map_err(|e| PlaidError::Malformed(e.to_string()))
}
