//! A full copy of the person's own data as one JSON file. Works in every licence state.
//! The licence token and the database key are never included.
use crate::{parse_day, App};
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
            .db
            .accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(_, name, kind, bal)| json!({"name": name, "kind": kind, "balanceCents": bal.cents()}))
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
}
