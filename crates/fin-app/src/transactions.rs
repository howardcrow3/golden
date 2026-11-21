//! Browsing transactions and teaching the app categories. Corrections are remembered as rules
//! (saved in the database) and applied to other transactions that still have no category.
use crate::{parse_day, App};
use fin_ledger::rules::Rule;
use serde::Serialize;
use std::collections::HashMap;

const RULES_KEY: &str = "user_rules";
const MAX_ROWS: usize = 500;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TxnRow {
    pub id: i64,
    pub day: String,
    pub payee: String,
    pub memo: String,
    pub amount_cents: i64,
    pub category: Option<String>,
    pub account: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Recategorized {
    pub category: String,
    /// Other transactions that got the category because of the remembered rule.
    pub also_updated: usize,
}

impl App {
    pub(crate) fn load_saved_rules(&mut self) {
        if let Ok(Some(json)) = self.db.get_setting(RULES_KEY) {
            if let Ok(saved) = serde_json::from_str::<Vec<Rule>>(&json) {
                self.rules.load_user_rules(saved);
            }
        }
    }

    /// Newest first, at most 500. `only_uncategorized` shows what still needs a category.
    pub fn transactions(
        &self,
        from: &str,
        to: &str,
        only_uncategorized: bool,
    ) -> Result<Vec<TxnRow>, String> {
        let names: HashMap<i64, String> = self
            .db
            .accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(id, n, _, _)| (id.0, n))
            .collect();
        let mut rows = self
            .db
            .txns_between(parse_day(from)?, parse_day(to)?)
            .map_err(|e| e.to_string())?;
        rows.sort_by(|a, b| b.day.cmp(&a.day).then(b.id.cmp(&a.id)));
        Ok(rows
            .into_iter()
            .filter(|t| !only_uncategorized || t.category.is_none())
            .take(MAX_ROWS)
            .map(|t| TxnRow {
                id: t.id,
                day: t.day.format("%Y-%m-%d").to_string(),
                payee: t.payee,
                memo: t.memo,
                amount_cents: t.amount.cents(),
                category: t.category,
                account: names.get(&t.account.0).cloned().unwrap_or_default(),
            })
            .collect())
    }

    /// Category names to offer: those in the ledger plus everything the rules can produce.
    pub fn category_choices(&self) -> Result<Vec<String>, String> {
        let mut v = self.db.category_names().map_err(|e| e.to_string())?;
        v.extend(self.rules.category_names());
        v.push("Transfer".into());
        v.sort();
        v.dedup();
        Ok(v)
    }

    /// Give one transaction a category. With `remember`, also learn the merchant so future
    /// imports and any other uncategorized lines for it are filed the same way.
    pub fn set_category(
        &mut self,
        id: i64,
        category: &str,
        remember: bool,
    ) -> Result<Recategorized, String> {
        self.require_edit()?;
        let category = category.trim();
        if category.is_empty() {
            return Err("Choose or type a category.".into());
        }
        let txn = self.db.get_txn(id).map_err(|e| e.to_string())?;
        self.ensure_open(txn.day)?;
        self.db
            .set_txn_category(id, Some(category))
            .map_err(|e| e.to_string())?;
        let mut also = 0;
        if remember {
            self.rules.learn(&txn.payee, category);
            let json =
                serde_json::to_string(&self.rules.user_rules()).map_err(|e| e.to_string())?;
            self.db
                .set_setting(RULES_KEY, &json)
                .map_err(|e| e.to_string())?;
            for other in self.db.uncategorized_txns().map_err(|e| e.to_string())? {
                if let Some(c) = self
                    .rules
                    .categorize(&other.payee, other.amount)
                    .map(String::from)
                {
                    self.db
                        .set_txn_category(other.id, Some(&c))
                        .map_err(|e| e.to_string())?;
                    also += 1;
                }
            }
        }
        Ok(Recategorized {
            category: category.to_string(),
            also_updated: also,
        })
    }
}
