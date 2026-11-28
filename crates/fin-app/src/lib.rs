//! Application service: the one place the UI talks to. It wires file import, categorization
//! rules and the ledger together and returns plain serializable shapes. The Tauri shell only
//! forwards calls here, so everything is testable without a window.
pub mod activity;
pub mod ai;
pub mod allocate;
pub mod ask;
pub mod backup;
pub mod balance;
pub mod bank;
pub mod budget;
mod card_signs;
pub mod chat;
pub mod close;
pub mod connections;
pub mod debts;
pub mod explain;
pub mod export;
pub mod folder;
pub mod forecast;
pub mod goals;
pub mod history;
pub mod household;
pub mod insights;
pub mod investments;
pub mod keystore;
pub mod library;
pub mod licensing;
pub mod localmodel;
pub mod mcp;
pub mod model;
pub mod offset;
pub mod paystub;
pub mod planfile;
pub mod progress;
pub mod receipts;
pub mod recurring;
pub mod refunds;
pub mod report;
pub mod retire_when;
pub mod scenarios;
pub mod setup;
pub mod ssa;
pub mod subscriptions;
pub mod suggest;
pub mod terms;
pub mod transactions;
pub mod transfers;
pub mod vault;
pub mod watch;
pub mod watchlists;
pub mod withholding;
pub mod xlsx;

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
    /// Last transaction date in the file (YYYY-MM-DD), if any.
    pub period_end: Option<String>,
    /// Where the original file was filed in the statement library, relative to its folder.
    pub filed_as: Option<String>,
    /// The closing balance the statement itself prints (OFX/QFX only).
    pub statement_balance_cents: Option<i64>,
    /// The account's balance in Golden after this import.
    pub balance_after_cents: Option<i64>,
    /// `Some(true)` when Golden's balance equals the statement's; `Some(false)` when it does not
    /// (statement balance minus Golden's is in `difference_cents`); `None` when the statement
    /// prints no balance.
    pub reconciled: Option<bool>,
    pub difference_cents: Option<i64>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountRow {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub balance_cents: i64,
    /// What the account held before its first imported statement (included in the balance).
    pub opening_cents: i64,
    /// Whether this account's CSV files print charges as positive amounts: None (detect from
    /// each file), true (always turn them round) or false (never).
    pub amounts_reversed: Option<bool>,
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
    /// A model running inside the app; it wins over a saved model address.
    /// Whichever model is in use (built-in, or a saved local address) for the AI helpers.
    pub(crate) ai: Option<std::sync::Arc<dyn model::Completer + Sync>>,
    pub(crate) embedded: Option<std::sync::Arc<dyn model::Completer + Sync>>,
    pub(crate) activity: activity::ActivityLog,
    /// Reads text out of pictures (scans, photos) on this computer, when the platform has one.
    pub(crate) ocr: Option<std::sync::Arc<dyn vault::Ocr>>,
    library_root: Option<std::path::PathBuf>,
    pub(crate) library_default: Option<std::path::PathBuf>,
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

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "checking" => "Checking",
        "savings" => "Savings",
        "credit" => "Credit card",
        "investment" => "Brokerage",
        "retirement" => "Retirement",
        "loan" => "Loan",
        _ => "Statement",
    }
}

/// Parse a `YYYY-MM-DD` string from the UI.
pub fn parse_day(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").map_err(|_| format!("not a date: {s}"))
}

impl App {
    pub fn new(db: Db) -> App {
        let mut app = App {
            db,
            rules: RuleSet::starter(),
            public_key: None,
            clock: today_local,
            interpreter: Box::new(chat::RuleInterpreter),
            ai: None,
            embedded: None,
            ocr: None,
            activity: activity::ActivityLog::default(),
            library_root: None,
            library_default: None,
        };
        app.load_saved_rules();
        app
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
        let mut parsed = fin_ingest::read_statement(bytes).map_err(|e| e.to_string())?;
        for t in &parsed {
            self.ensure_open(t.day)?;
        }
        let acct: AccountId = self
            .db
            .find_or_add_account(account, kind)
            .map_err(|e| e.to_string())?;
        if self.card_csv_reversed(bytes, acct, &parsed)? {
            for t in &mut parsed {
                t.amount = -t.amount;
            }
        }
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
        self.mark_transfers()?;
        self.mark_refunds()?;
        let printed = fin_ingest::statement_balance(bytes);
        let balance_after = self
            .accounts()?
            .into_iter()
            .find(|a| a.id == acct.0)
            .map(|a| a.balance_cents);
        let (reconciled, difference_cents) = match (printed, balance_after) {
            (Some((_, p)), Some(b)) => (Some(p.cents() == b), Some(p.cents() - b)),
            _ => (None, None),
        };
        Ok(ImportReport {
            statement_balance_cents: printed.map(|(_, m)| m.cents()),
            balance_after_cents: balance_after,
            reconciled,
            difference_cents,
            account: account.to_string(),
            parsed: parsed.len(),
            added: summary.added,
            skipped_duplicates: summary.skipped,
            uncategorized,
            total_cents: fin_ingest::total(&parsed).cents(),
            period_end: parsed
                .iter()
                .map(|t| t.day)
                .max()
                .map(|d| d.format("%Y-%m-%d").to_string()),
            filed_as: None,
        })
    }

    /// Where original statements are filed (None disables filing).
    /// `root` is the default; a folder the person chose earlier (saved in settings) wins.
    pub fn with_library(mut self, root: Option<std::path::PathBuf>) -> App {
        self.library_default = root.clone();
        self.library_root = root;
        if let Ok(Some(saved)) = self.db.get_setting(library::FOLDER_KEY) {
            if !saved.trim().is_empty() {
                self.library_root = Some(std::path::PathBuf::from(saved));
            }
        }
        self
    }

    pub fn library_root(&self) -> Option<&std::path::Path> {
        self.library_root.as_deref()
    }

    /// Import a file the person picked: read it into the ledger, then file the original in the
    /// statement library named by account, month and type. Nothing is filed when it adds nothing
    /// new or when the import fails.
    pub fn import_file(
        &mut self,
        original_name: &str,
        bytes: &[u8],
        account: &str,
        kind: &str,
    ) -> Result<ImportReport, String> {
        let mut report = self.import_statement(bytes, account, kind)?;
        if let (Some(root), Some(end), true) =
            (&self.library_root, &report.period_end, report.added > 0)
        {
            let meta = fin_ingest::library::StatementMeta {
                institution: account.trim().to_string(),
                kind: kind_label(kind).to_string(),
                last4: None,
                period_end: parse_day(end)?,
                extension: std::path::Path::new(original_name)
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_else(|| "csv".into()),
            };
            let meta = self.refine_statement_meta(bytes, meta);
            report.filed_as = Some(library::file_statement(root, bytes, &meta).map_err(|e| {
                format!("The transactions were imported, but filing the original failed: {e}")
            })?);
        }
        self.remember_file(bytes)?;
        Ok(report)
    }

    pub fn accounts(&self) -> Result<Vec<AccountRow>, String> {
        let openings: std::collections::HashMap<i64, i64> = self
            .db
            .openings()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(id, c)| (id.0, c))
            .collect();
        let reversed: std::collections::HashMap<i64, Option<bool>> = self
            .db
            .amounts_reversed()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(id, r)| (id.0, r))
            .collect();
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
                opening_cents: openings.get(&id.0).copied().unwrap_or(0),
                amounts_reversed: reversed.get(&id.0).copied().flatten(),
            })
            .collect())
    }

    /// Set what an account held before its first imported statement. The balance, net worth
    /// and projection all include it; cash flow and budgets do not (it isn't income).
    pub fn set_opening_balance(&self, account_id: i64, cents: i64) -> Result<(), String> {
        self.require_edit()?;
        if cents.abs() > 100_000_000_000 {
            return Err("That amount is too large.".into());
        }
        self.db
            .set_opening(AccountId(account_id), cents)
            .map_err(|_| "No such account.".to_string())
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
