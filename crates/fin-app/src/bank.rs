//! Bank connections (the Plaid add-on, opt-in). Golden talks to its own sync service, which holds
//! Plaid's secret; the access token for each linked bank is kept in the OS keychain, never in the
//! database. Synced lines are provisional: importing the month's statement replaces them.
use crate::App;
use fin_db::bank::BankTxn;
use fin_db::ledger::AccountId;
use fin_plaid::proxy::{self, Exchanged, LinkToken};
use fin_plaid::{sync_all, LinkedItem, PlaidApi, PlaidError, SyncPage};
use serde::Serialize;
use serde_json::Value;

/// Sends a request to the sync service and returns the HTTP status and body. The app's version
/// does HTTPS; tests use a canned one.
pub trait BankTransport {
    fn post(&self, path: &str, body: &Value) -> Result<(u16, String), String>;
}

/// Where access tokens live (the OS keychain in the app).
pub trait SecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, String>;
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    fn delete(&self, key: &str) -> Result<(), String>;
}

const ITEMS: &str = "bank.items";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BankSyncReport {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    /// Lines for accounts Golden does not know yet.
    pub skipped: usize,
    /// Bank lines dropped because an imported statement now covers them.
    pub reconciled: usize,
}

/// One bank's result from "sync everything".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BankOutcome {
    pub bank: String,
    pub report: Option<BankSyncReport>,
    pub error: Option<String>,
}

fn secret_key(item_id: &str) -> String {
    format!("plaid:{item_id}")
}

fn say(e: PlaidError) -> String {
    match e {
        PlaidError::Api { code, message } if code == "LICENCE" => message,
        PlaidError::LoginRequired => {
            "Your bank needs you to sign in again. Open Connections and relink it.".into()
        }
        other => other.to_string(),
    }
}

fn call<T: serde::de::DeserializeOwned>(
    t: &dyn BankTransport,
    path: &str,
    body: &Value,
) -> Result<T, PlaidError> {
    let (status, text) = t.post(path, body).map_err(PlaidError::Network)?;
    if status == 200 {
        proxy::parse_ok(&text)
    } else {
        Err(proxy::parse_error(&text))
    }
}

struct Service<'a> {
    transport: &'a dyn BankTransport,
    licence: &'a str,
    access_token: &'a str,
}

impl PlaidApi for Service<'_> {
    fn sync(&self, cursor: Option<&str>) -> Result<SyncPage, PlaidError> {
        call(
            self.transport,
            proxy::PATH_SYNC,
            &proxy::sync_body(self.licence, self.access_token, cursor),
        )
    }
}

impl App {
    pub fn bank_items(&self) -> Result<Vec<LinkedItem>, String> {
        match self.db.get_setting(ITEMS).map_err(|e| e.to_string())? {
            Some(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
            None => Ok(vec![]),
        }
    }

    fn save_bank_items(&self, items: &[LinkedItem]) -> Result<(), String> {
        let s = serde_json::to_string(items).map_err(|e| e.to_string())?;
        self.db.set_setting(ITEMS, &s).map_err(|e| e.to_string())
    }

    /// The licence token to send, after checking the Plaid add-on is active today.
    fn bank_licence(&self) -> Result<String, String> {
        let today = (self.clock)();
        if !self.license_status(today)?.plaid_active {
            return Err("Bank connections need the Plaid add-on. Add it from Licence.".into());
        }
        self.license_token()
            .ok_or_else(|| "Bank connections need the Plaid add-on.".to_string())
    }

    /// A one-time token for Plaid's sign-in window.
    pub fn bank_link_token(&self, t: &dyn BankTransport) -> Result<String, String> {
        self.require_edit()?;
        let lic = self.bank_licence()?;
        let user = self.db.set_setting_if_absent("bank.user", &random_id());
        let user = user.map_err(|e| e.to_string())?;
        let r: LinkToken = call(
            t,
            proxy::PATH_LINK_TOKEN,
            &proxy::link_token_body(&lic, &user),
        )
        .map_err(say)?;
        Ok(r.link_token)
    }

    /// Trade the token Plaid's window returned for a lasting one, keep it in the keychain, and
    /// pull the first batch of transactions.
    pub fn bank_finish_link(
        &mut self,
        t: &dyn BankTransport,
        secrets: &dyn SecretStore,
        public_token: &str,
        institution: &str,
    ) -> Result<BankSyncReport, String> {
        self.require_edit()?;
        let lic = self.bank_licence()?;
        let ex: Exchanged = call(
            t,
            proxy::PATH_EXCHANGE,
            &proxy::exchange_body(&lic, public_token),
        )
        .map_err(say)?;
        secrets.set(&secret_key(&ex.item_id), &ex.access_token)?;
        let mut items = self.bank_items()?;
        items.retain(|i| i.item_id != ex.item_id);
        items.push(LinkedItem {
            item_id: ex.item_id.clone(),
            institution: institution.trim().to_string(),
            cursor: String::new(),
            status: "active".into(),
        });
        self.save_bank_items(&items)?;
        self.bank_sync(t, secrets, &ex.item_id)
    }

    /// Pull what changed at one bank since the last sync.
    pub fn bank_sync(
        &mut self,
        t: &dyn BankTransport,
        secrets: &dyn SecretStore,
        item_id: &str,
    ) -> Result<BankSyncReport, String> {
        self.require_edit()?;
        let lic = self.bank_licence()?;
        let mut items = self.bank_items()?;
        let at = items
            .iter()
            .position(|i| i.item_id == item_id)
            .ok_or("That bank is not linked.")?;
        match items[at].status.as_str() {
            "paused" => return Err("That bank is paused. Resume it to sync.".into()),
            "login_required" => return Err(say(PlaidError::LoginRequired)),
            _ => {}
        }
        let token = secrets
            .get(&secret_key(item_id))?
            .ok_or("The saved sign-in for this bank is missing. Relink it.")?;
        let svc = Service {
            transport: t,
            licence: &lic,
            access_token: &token,
        };
        let cursor = Some(items[at].cursor.as_str()).filter(|c| !c.is_empty());
        let changes = match sync_all(&svc, cursor) {
            Ok(c) => c,
            Err(PlaidError::LoginRequired) => {
                items[at].status = "login_required".into();
                self.save_bank_items(&items)?;
                return Err(say(PlaidError::LoginRequired));
            }
            Err(e) => return Err(say(e)),
        };

        let institution = items[at].institution.clone();
        for a in &changes.accounts {
            let mask = a
                .mask
                .as_deref()
                .map(|m| format!(" ••{m}"))
                .unwrap_or_default();
            let name = format!("{institution} {}{mask}", a.name).trim().to_string();
            let id = self
                .db
                .find_or_add_account(&name, a.golden_kind())
                .map_err(|e| e.to_string())?;
            self.db
                .link_bank_account(&a.account_id, item_id, id)
                .map_err(|e| e.to_string())?;
        }

        let mut report = BankSyncReport::default();
        let mut upserts = vec![];
        let mut touched: Vec<AccountId> = vec![];
        for m in changes.added.iter().chain(changes.modified.iter()) {
            match self
                .db
                .bank_account(&m.account_id)
                .map_err(|e| e.to_string())?
            {
                Some(acct) => {
                    if !touched.contains(&acct) {
                        touched.push(acct);
                    }
                    upserts.push(BankTxn {
                        external_id: m.id.clone(),
                        account: acct,
                        day: m.day,
                        amount: m.amount,
                        payee: m.payee.clone(),
                        memo: m.memo.clone(),
                        pending: m.pending,
                        replaces: m.replaces.clone(),
                        category: None,
                    });
                }
                None => report.skipped += 1,
            }
        }
        let s = self
            .db
            .apply_bank_changes(&upserts, &changes.removed)
            .map_err(|e| e.to_string())?;
        report.added = s.added;
        report.updated = s.updated;
        report.removed = s.removed;
        for acct in touched {
            report.reconciled += self.db.reconcile_bank(acct).map_err(|e| e.to_string())?;
        }
        // Only move the saved position once everything above is stored.
        items[at].cursor = changes.cursor;
        self.save_bank_items(&items)?;
        Ok(report)
    }

    /// Sync every active bank. A bank that fails does not stop the others; its error is returned
    /// beside its name.
    pub fn bank_sync_all(
        &mut self,
        t: &dyn BankTransport,
        secrets: &dyn SecretStore,
    ) -> Result<Vec<BankOutcome>, String> {
        let items = self.bank_items()?;
        let mut out = vec![];
        for i in items.iter().filter(|i| i.status == "active") {
            let r = self.bank_sync(t, secrets, &i.item_id);
            out.push(BankOutcome {
                bank: i.institution.clone(),
                report: r.as_ref().ok().copied(),
                error: r.err(),
            });
        }
        Ok(out)
    }

    pub fn bank_set_paused(&self, item_id: &str, paused: bool) -> Result<(), String> {
        let mut items = self.bank_items()?;
        let it = items
            .iter_mut()
            .find(|i| i.item_id == item_id)
            .ok_or("That bank is not linked.")?;
        it.status = if paused { "paused" } else { "active" }.into();
        self.save_bank_items(&items)
    }

    /// Unlink a bank: tell Plaid to drop access (best effort), delete the token, and remove the
    /// lines it synced. Statements you imported are untouched.
    pub fn bank_unlink(
        &mut self,
        t: &dyn BankTransport,
        secrets: &dyn SecretStore,
        item_id: &str,
    ) -> Result<usize, String> {
        let mut items = self.bank_items()?;
        if !items.iter().any(|i| i.item_id == item_id) {
            return Err("That bank is not linked.".into());
        }
        if let (Some(tok), Ok(lic)) = (secrets.get(&secret_key(item_id))?, self.bank_licence()) {
            // Plaid may already have dropped it; the local cleanup below must happen regardless.
            let _ = t.post(proxy::PATH_REMOVE, &proxy::remove_body(&lic, &tok));
        }
        secrets.delete(&secret_key(item_id))?;
        let removed = self
            .db
            .unlink_bank_item(item_id)
            .map_err(|e| e.to_string())?;
        items.retain(|i| i.item_id != item_id);
        self.save_bank_items(&items)?;
        Ok(removed)
    }
}

fn random_id() -> String {
    let mut b = [0u8; 12];
    getrandom::getrandom(&mut b).expect("system randomness");
    b.iter().map(|x| format!("{x:02x}")).collect()
}
