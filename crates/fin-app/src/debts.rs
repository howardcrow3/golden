//! Debt payoff: the debts on the balance sheet, their rates and minimum payments, and how fast
//! they clear under each strategy. Rates and minimums are typed in once per debt (statements
//! rarely carry them in a form we can read), and the plan only uses debts that have them.
use crate::App;
use fin_plan::debt::{payoff, Debt, Payoff, Strategy};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const KEY: &str = "debt_terms";
const MAX_APR_BP: i64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Terms {
    pub apr_bp: i64,
    pub min_cents: i64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DebtRow {
    pub name: String,
    pub balance_cents: i64,
    pub apr_bp: i64,
    pub min_cents: i64,
    /// False until a rate and minimum have been entered.
    pub has_terms: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DebtPlan {
    pub debts: Vec<DebtRow>,
    pub extra_cents: i64,
    /// None when no debt has terms yet.
    pub avalanche: Option<Payoff>,
    pub snowball: Option<Payoff>,
    /// The same debts paid with minimums only, to show what the extra money saves.
    pub minimums_only: Option<Payoff>,
}

impl App {
    fn saved_terms(&self) -> Result<BTreeMap<String, Terms>, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            Some(s) => serde_json::from_str(&s)
                .map_err(|e| format!("saved debt terms are unreadable: {e}")),
            None => Ok(BTreeMap::new()),
        }
    }

    /// Record the yearly rate (hundredths of a percent) and monthly minimum for one debt.
    pub fn set_debt_terms(&self, name: &str, apr_bp: i64, min_cents: i64) -> Result<(), String> {
        self.require_edit()?;
        if !(0..=MAX_APR_BP).contains(&apr_bp) {
            return Err("The interest rate must be between 0% and 100%.".into());
        }
        if !(0..=1_000_000_000).contains(&min_cents) {
            return Err("The minimum payment must be zero or more.".into());
        }
        if !self.balance_sheet()?.debts.iter().any(|d| d.name == name) {
            return Err(format!("{name} isn't one of the debts Golden knows about."));
        }
        let mut t = self.saved_terms()?;
        t.insert(name.to_string(), Terms { apr_bp, min_cents });
        let json = serde_json::to_string(&t).map_err(|e| e.to_string())?;
        self.db.set_setting(KEY, &json).map_err(|e| e.to_string())
    }

    /// How the current debts clear with `extra_cents` a month on top of the minimums.
    pub fn debt_plan(&self, extra_cents: i64) -> Result<DebtPlan, String> {
        if extra_cents < 0 {
            return Err("The extra payment can't be negative.".into());
        }
        let terms = self.saved_terms()?;
        let rows: Vec<DebtRow> = self
            .balance_sheet()?
            .debts
            .into_iter()
            .map(|d| {
                let t = terms.get(&d.name);
                DebtRow {
                    has_terms: t.is_some(),
                    apr_bp: t.map_or(0, |t| t.apr_bp),
                    min_cents: t.map_or(0, |t| t.min_cents),
                    name: d.name,
                    balance_cents: d.cents,
                }
            })
            .collect();
        let usable: Vec<Debt> = rows
            .iter()
            .filter(|r| r.has_terms)
            .map(|r| Debt {
                name: r.name.clone(),
                balance_cents: r.balance_cents,
                apr_bp: r.apr_bp,
                min_cents: r.min_cents,
            })
            .collect();
        let run = |extra, s| -> Result<Option<Payoff>, String> {
            if usable.is_empty() {
                return Ok(None);
            }
            payoff(&usable, extra, s)
                .map(Some)
                .map_err(|e| e.to_string())
        };
        Ok(DebtPlan {
            avalanche: run(extra_cents, Strategy::Avalanche)?,
            snowball: run(extra_cents, Strategy::Snowball)?,
            minimums_only: run(0, Strategy::Avalanche)?,
            debts: rows,
            extra_cents,
        })
    }
}
