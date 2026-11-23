//! Plaid bank sync, minus the network. Golden never holds Plaid's secret: the app talks to a small
//! server (see `services/plaid-server`) that adds the secret and forwards the call. This crate holds
//! everything else: the shapes of the answers, turning Plaid's transactions into Golden's, and the
//! cursor loop that pages through changes. The transport is injected, so it is all testable offline.
use chrono::NaiveDate;
use fin_core::Money;
use serde::{Deserialize, Serialize};

pub mod link;
pub mod proxy;

/// How many times a page loop restarts when Plaid says the data changed under it.
pub const MAX_RESTARTS: usize = 3;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PlaidTxn {
    pub transaction_id: String,
    pub account_id: String,
    pub date: NaiveDate,
    /// Plaid counts money going out as positive.
    pub amount: f64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub merchant_name: Option<String>,
    #[serde(default)]
    pub pending: bool,
    #[serde(default)]
    pub personal_finance_category: Option<PlaidCategory>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PlaidCategory {
    pub primary: String,
    #[serde(default)]
    pub detailed: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RemovedTxn {
    pub transaction_id: String,
}

/// A bank account as Plaid describes it (balances are not requested).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlaidAccount {
    pub account_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub mask: Option<String>,
    /// "depository", "credit", "loan", "investment" or "other".
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub subtype: Option<String>,
}

impl PlaidAccount {
    /// The kind Golden files this account under.
    pub fn golden_kind(&self) -> &'static str {
        match (self.kind.as_str(), self.subtype.as_deref()) {
            ("credit", _) => "credit",
            ("loan", _) => "loan",
            ("investment", Some("401k" | "403b" | "ira" | "roth" | "roth 401k" | "529")) => {
                "retirement"
            }
            ("investment", _) => "investment",
            ("depository", Some("savings" | "cd" | "money market" | "hsa")) => "savings",
            ("depository", _) => "checking",
            _ => "other",
        }
    }
}

/// One page of `/transactions/sync`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SyncPage {
    #[serde(default)]
    pub accounts: Vec<PlaidAccount>,
    #[serde(default)]
    pub added: Vec<PlaidTxn>,
    #[serde(default)]
    pub modified: Vec<PlaidTxn>,
    #[serde(default)]
    pub removed: Vec<RemovedTxn>,
    pub next_cursor: String,
    #[serde(default)]
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlaidError {
    /// The data changed while paging; start the sync again from the first cursor.
    #[error("the bank data changed while syncing")]
    MutationDuringPagination,
    /// The bank wants the person to sign in again.
    #[error("the bank needs you to sign in again")]
    LoginRequired,
    #[error("Plaid said {code}: {message}")]
    Api { code: String, message: String },
    #[error("could not reach the sync service: {0}")]
    Network(String),
    #[error("the sync service sent something unexpected: {0}")]
    Malformed(String),
}

impl PlaidError {
    /// Turn a Plaid error body's `error_code` into our error.
    pub fn from_code(code: &str, message: &str) -> PlaidError {
        match code {
            "TRANSACTIONS_SYNC_MUTATION_DURING_PAGINATION" => PlaidError::MutationDuringPagination,
            "ITEM_LOGIN_REQUIRED" => PlaidError::LoginRequired,
            _ => PlaidError::Api {
                code: code.into(),
                message: message.into(),
            },
        }
    }
}

/// The one call the sync loop needs. The app implements it over HTTPS to the sync service;
/// tests implement it with canned pages.
pub trait PlaidApi {
    fn sync(&self, cursor: Option<&str>) -> Result<SyncPage, PlaidError>;
}

/// A bank line in Golden's terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapped {
    pub id: String,
    pub account_id: String,
    pub day: NaiveDate,
    /// Negative is money out, like everywhere else in Golden.
    pub amount: Money,
    pub payee: String,
    pub memo: String,
    /// Not yet settled; replaced when the statement arrives.
    pub pending: bool,
    /// Plaid's own broad category, kept as a hint for categorizing.
    pub category_hint: Option<String>,
}

/// Dollars as Plaid prints them to whole cents, rounded half away from zero.
pub fn to_cents(amount: f64) -> i64 {
    (amount * 100.0).round() as i64
}

pub fn map_txn(t: &PlaidTxn) -> Mapped {
    let payee = t
        .merchant_name
        .as_deref()
        .filter(|m| !m.trim().is_empty())
        .unwrap_or(&t.name)
        .trim()
        .to_string();
    let memo = if t.name.trim() == payee {
        String::new()
    } else {
        t.name.trim().to_string()
    };
    Mapped {
        id: t.transaction_id.clone(),
        account_id: t.account_id.clone(),
        day: t.date,
        amount: Money::from_cents(-to_cents(t.amount)),
        payee,
        memo,
        pending: t.pending,
        category_hint: t
            .personal_finance_category
            .as_ref()
            .map(|c| c.primary.clone()),
    }
}

/// Everything that changed since a cursor.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Changes {
    pub added: Vec<Mapped>,
    pub modified: Vec<Mapped>,
    pub removed: Vec<String>,
    /// Every account Plaid described along the way, once each.
    pub accounts: Vec<PlaidAccount>,
    /// Save this and pass it next time.
    pub cursor: String,
}

/// Page through every change since `cursor`. If Plaid reports the data moved mid-way, the whole
/// loop starts over from the cursor we were given, up to [`MAX_RESTARTS`] times.
pub fn sync_all(api: &dyn PlaidApi, cursor: Option<&str>) -> Result<Changes, PlaidError> {
    let mut restarts = 0;
    'again: loop {
        let mut out = Changes::default();
        let mut at: Option<String> = cursor.map(str::to_string);
        loop {
            let page = match api.sync(at.as_deref()) {
                Ok(p) => p,
                Err(PlaidError::MutationDuringPagination) if restarts < MAX_RESTARTS => {
                    restarts += 1;
                    continue 'again;
                }
                Err(e) => return Err(e),
            };
            out.added.extend(page.added.iter().map(map_txn));
            out.modified.extend(page.modified.iter().map(map_txn));
            out.removed
                .extend(page.removed.into_iter().map(|r| r.transaction_id));
            for a in page.accounts {
                if !out.accounts.iter().any(|x| x.account_id == a.account_id) {
                    out.accounts.push(a);
                }
            }
            out.cursor = page.next_cursor.clone();
            if !page.has_more {
                return Ok(out);
            }
            at = Some(page.next_cursor);
        }
    }
}

/// A linked bank, as kept on this computer. The access token lives in the OS keychain, never here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedItem {
    pub item_id: String,
    pub institution: String,
    /// Saved sync position; empty before the first sync.
    #[serde(default)]
    pub cursor: String,
    /// "active", "paused" or "login_required".
    pub status: String,
}
