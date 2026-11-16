//! Application service: the one place the UI talks to. It wires file import, categorization
//! rules and the ledger together and returns plain serializable shapes. The Tauri shell only
//! forwards calls here, so everything is testable without a window.
pub mod chat;
pub mod household;
pub mod keystore;
pub mod licensing;

use chrono::NaiveDate;
use fin_core::{Grain, PeriodKey};
use fin_db::ledger::{AccountId, Metric, NewTxn};
use fin_db::Db;
use fin_ledger::rules::RuleSet;
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub account: String,
    pub parsed: usize,
    pub added: usize,
    pub skipped_duplicates: usize,
    pub uncategorized: usize,
    pub total_cents: i64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountRow {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub balance_cents: i64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PeriodRow {
    pub period: String,
    pub income_cents: i64,
    pub spending_cents: i64,
    pub net_cents: i64,
    pub count: i64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CategoryRow {
    pub category: String,
    pub cents: i64,
}

pub struct App {
    db: Db,
    rules: RuleSet,
    pub(crate) public_key: Option<ed25519_dalek::VerifyingKey>,
    pub(crate) clock: fn() -> NaiveDate,
    pub(crate) interpreter: Box<dyn chat::Interpreter>,
}

fn today_local() -> NaiveDate {
    chrono::Local::now().date_naive()
}

fn grain_of(s: &str) -> Result<Grain, String> {
    match s {
        "week" => Ok(Grain::Week),
        "month" => Ok(Grain::Month),
        "quarter" => Ok(Grain::Quarter),
        "year" => Ok(Grain::Year),
        other => Err(format!("unknown grain: {other}")),
    }
}

/// Parse a `YYYY-MM-DD` string from the UI.
pub fn parse_day(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").map_err(|_| format!("not a date: {s}"))
}

impl App {
    pub fn new(db: Db) -> App {
        App {
            db,
            rules: RuleSet::starter(),
            public_key: None,
            clock: today_local,
            interpreter: Box::new(chat::RuleInterpreter),
        }
    }

    /// The app's licence-verification key (bundled at build time).
    pub fn with_public_key(mut self, key: Option<ed25519_dalek::VerifyingKey>) -> App {
        self.public_key = key;
        self
    }

    /// Replace the date source (tests).
    pub fn with_clock(mut self, clock: fn() -> NaiveDate) -> App {
        self.clock = clock;
        self
    }

    pub fn in_memory() -> Result<App, String> {
        Ok(App::new(Db::open_in_memory().map_err(|e| e.to_string())?))
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    /// Parse a statement file, categorize it, and store it in `account` (created if new).
    /// Importing the same file twice adds nothing.
    pub fn import_statement(
        &mut self,
        bytes: &[u8],
        account: &str,
        kind: &str,
    ) -> Result<ImportReport, String> {
        self.require_edit()?;
        let account = account.trim();
        if account.is_empty() {
            return Err("Give the account a name.".into());
        }
        let parsed = fin_ingest::read_statement(bytes).map_err(|e| e.to_string())?;
        let acct: AccountId = self
            .db
            .find_or_add_account(account, kind)
            .map_err(|e| e.to_string())?;
        let mut uncategorized = 0;
        let rows: Vec<NewTxn> = parsed
            .iter()
            .map(|t| {
                let category = self.rules.categorize(&t.payee, t.amount).map(String::from);
                if category.is_none() {
                    uncategorized += 1;
                }
                NewTxn {
                    account: acct,
                    day: t.day,
                    amount: t.amount,
                    payee: t.payee.clone(),
                    memo: t.memo.clone(),
                    category,
                    stream: None,
                }
            })
            .collect();
        let summary = self.db.import_batch(&rows).map_err(|e| e.to_string())?;
        Ok(ImportReport {
            account: account.to_string(),
            parsed: parsed.len(),
            added: summary.added,
            skipped_duplicates: summary.skipped,
            uncategorized,
            total_cents: fin_ingest::total(&parsed).cents(),
        })
    }

    pub fn accounts(&self) -> Result<Vec<AccountRow>, String> {
        Ok(self
            .db
            .accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(id, name, kind, bal)| AccountRow {
                id: id.0,
                name,
                kind,
                balance_cents: bal.cents(),
            })
            .collect())
    }

    /// Income / spending / net per period; `grain` is week, month, quarter or year.
    pub fn cash_flow(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        grain: &str,
    ) -> Result<Vec<PeriodRow>, String> {
        let g = grain_of(grain)?;
        Ok(self
            .db
            .totals(from, to, g)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|p| PeriodRow {
                period: p.period,
                income_cents: p.income.cents(),
                spending_cents: p.spending.cents(),
                net_cents: p.net.cents(),
                count: p.count,
            })
            .collect())
    }

    /// Spending (or income) by category for the period containing `day`.
    pub fn categories(
        &self,
        day: NaiveDate,
        grain: &str,
        income: bool,
    ) -> Result<Vec<CategoryRow>, String> {
        let p = PeriodKey::containing(grain_of(grain)?, day);
        let metric = if income {
            Metric::Income
        } else {
            Metric::Spending
        };
        Ok(self
            .db
            .category_totals(p, metric)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(category, m)| CategoryRow {
                category,
                cents: m.cents(),
            })
            .collect())
    }

    /// String-date variants for the Tauri layer.
    pub fn cash_flow_str(
        &self,
        from: &str,
        to: &str,
        grain: &str,
    ) -> Result<Vec<PeriodRow>, String> {
        self.cash_flow(parse_day(from)?, parse_day(to)?, grain)
    }

    pub fn categories_str(
        &self,
        day: &str,
        grain: &str,
        income: bool,
    ) -> Result<Vec<CategoryRow>, String> {
        self.categories(parse_day(day)?, grain, income)
    }

    /// Teach the app a merchant's category for future imports.
    pub fn learn_category(&mut self, payee: &str, category: &str) {
        self.rules.learn(payee, category);
    }
}
