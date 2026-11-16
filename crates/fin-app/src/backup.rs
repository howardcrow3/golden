//! A full copy of the person's own data as one JSON file. Works in every licence state.
//! The licence token and the database key are never included.
use crate::{parse_day, App};
use fin_core::Money;
use fin_db::ledger::NewTxn;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const INCLUDED_SETTINGS: [&str; 3] = ["household", "budgets", "user_rules"];

impl App {
    /// Every account, transaction and saved setting, as pretty JSON.
    pub fn backup_json(&self) -> Result<String, String> {
        let names: std::collections::HashMap<i64, String> = self
            .db
            .accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(id, n, _, _)| (id.0, n))
            .collect();
        let accounts: Vec<Value> = self
            .accounts()?
            .into_iter()
            .map(|a| json!({"name": a.name, "kind": a.kind, "balanceCents": a.balance_cents, "openingCents": a.opening_cents}))
            .collect();
        let mut txns = self
            .db
            .txns_between(parse_day("1900-01-01")?, parse_day("2200-01-01")?)
            .map_err(|e| e.to_string())?;
        txns.sort_by(|a, b| a.day.cmp(&b.day).then(a.id.cmp(&b.id)));
        let transactions: Vec<Value> = txns
            .into_iter()
            .map(|t| {
                json!({
                    "day": t.day.format("%Y-%m-%d").to_string(),
                    "account": names.get(&t.account.0).cloned().unwrap_or_default(),
                    "payee": t.payee,
                    "memo": t.memo,
                    "amountCents": t.amount.cents(),
                    "category": t.category,
                })
            })
            .collect();
        let mut settings = serde_json::Map::new();
        for key in INCLUDED_SETTINGS {
            if let Some(raw) = self.db.get_setting(key).map_err(|e| e.to_string())? {
                // Stored as JSON text; embed it as real JSON.
                settings.insert(
                    key.into(),
                    serde_json::from_str(&raw).unwrap_or(Value::String(raw)),
                );
            }
        }
        let doc = json!({
            "app": "Golden",
            "backupVersion": 1,
            "exportedOn": (self.clock)().format("%Y-%m-%d").to_string(),
            "accounts": accounts,
            "transactions": transactions,
            "settings": settings,
        });
        serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())
    }

    /// Write the backup into `dir` as `golden-backup-<date>.json` (numbered if it exists).
    pub fn export_backup(&self, dir: &Path) -> Result<PathBuf, String> {
        let json = self.backup_json()?;
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let base = format!("golden-backup-{}", (self.clock)().format("%Y-%m-%d"));
        let mut path = dir.join(format!("{base}.json"));
        let mut n = 2;
        while path.exists() {
            path = dir.join(format!("{base} ({n}).json"));
            n += 1;
        }
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(path)
    }

    /// Merge a backup made by `backup_json` into this database. Transactions already here are
    /// skipped (so restoring twice changes nothing), and a saved setting is only filled in when
    /// this database doesn't have one yet, so existing work is never overwritten.
    pub fn restore_backup(&mut self, text: &str) -> Result<RestoreReport, String> {
        self.require_edit()?;
        let v: Value = serde_json::from_str(text)
            .map_err(|_| "That file isn't a Golden backup (it isn't valid JSON).".to_string())?;
        if v["app"] != "Golden" {
            return Err("That file isn't a Golden backup.".into());
        }
        match v["backupVersion"].as_i64() {
            Some(1) => {}
            Some(n) => return Err(format!("This backup is version {n}; this copy of Golden reads version 1. Update Golden and try again.")),
            None => return Err("That file isn't a Golden backup (no version).".into()),
        }
        let mut ids = std::collections::HashMap::new();
        for a in v["accounts"].as_array().into_iter().flatten() {
            let name = a["name"]
                .as_str()
                .ok_or("A backed-up account has no name.")?;
            let kind = a["kind"].as_str().unwrap_or("checking");
            let existed = self
                .db
                .accounts()
                .map_err(|e| e.to_string())?
                .iter()
                .any(|(_, n, _, _)| n == name);
            let id = self
                .db
                .find_or_add_account(name, kind)
                .map_err(|e| e.to_string())?;
            // A starting balance comes back only for accounts this copy didn't have: never
            // overwrite what someone has typed here.
            if !existed {
                if let Some(c) = a["openingCents"].as_i64() {
                    self.db.set_opening(id, c).map_err(|e| e.to_string())?;
                }
            }
            ids.insert(name.to_string(), id);
        }
        let mut rows = vec![];
        for t in v["transactions"].as_array().into_iter().flatten() {
            let account = t["account"].as_str().unwrap_or_default();
            let id = match ids.get(account) {
                Some(id) => *id,
                None => {
                    let id = self
                        .db
                        .find_or_add_account(account, "checking")
                        .map_err(|e| e.to_string())?;
                    ids.insert(account.to_string(), id);
                    id
                }
            };
            rows.push(NewTxn {
                account: id,
                day: parse_day(t["day"].as_str().unwrap_or_default())?,
                amount: Money::from_cents(
                    t["amountCents"]
                        .as_i64()
                        .ok_or("A backed-up transaction has no amount.")?,
                ),
                payee: t["payee"].as_str().unwrap_or_default().to_string(),
                memo: t["memo"].as_str().unwrap_or_default().to_string(),
                category: t["category"].as_str().map(String::from),
                stream: None,
            });
        }
        let summary = self.db.import_batch(&rows).map_err(|e| e.to_string())?;
        let mut restored = vec![];
        for key in INCLUDED_SETTINGS {
            let have = self.db.get_setting(key).map_err(|e| e.to_string())?;
            if let (None, Some(saved)) = (have, v["settings"].get(key)) {
                self.db
                    .set_setting(key, &saved.to_string())
                    .map_err(|e| e.to_string())?;
                restored.push(key.to_string());
            }
        }
        self.load_saved_rules();
        Ok(RestoreReport {
            accounts: ids.len(),
            added: summary.added,
            skipped_duplicates: summary.skipped,
            settings_restored: restored,
        })
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    pub accounts: usize,
    pub added: usize,
    pub skipped_duplicates: usize,
    pub settings_restored: Vec<String>,
}
