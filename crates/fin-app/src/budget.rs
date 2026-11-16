//! Spending budgets: a monthly amount per category (and an optional overall one),
//! shown against what was actually spent in any week, month, quarter or year.
use crate::{grain_of, App};
use fin_db::ledger::Metric;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const KEY: &str = "budgets";
/// Key of the overall budget.
pub const TOTAL: &str = "*";

/// Monthly budget in cents by category; `TOTAL` is the overall one.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Budgets {
    pub monthly: BTreeMap<String, i64>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BudgetRow {
    /// Category name, or "Total".
    pub category: String,
    pub budget_cents: i64,
    pub spent_cents: i64,
    /// Budget minus spent; negative means over budget.
    pub remaining_cents: i64,
}

/// A monthly amount expressed per `grain` by average length in days; mirrors the UI's
/// `convertBudget` (units are 1/4800ths of a day).
pub fn convert_monthly(cents: i64, grain: &str) -> Result<i64, String> {
    let unit: i128 = match grain {
        "week" => 33_600,
        "month" => 146_097,
        "quarter" => 438_291,
        "year" => 1_753_164,
        other => return Err(format!("unknown grain {other:?}")),
    };
    let n = cents as i128 * unit;
    let d: i128 = 146_097;
    Ok(((n * 2 + d) / (2 * d)) as i64)
}

impl App {
    pub fn budgets(&self) -> Result<Budgets, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            Some(s) => {
                serde_json::from_str(&s).map_err(|e| format!("saved budgets are unreadable: {e}"))
            }
            None => Ok(Budgets::default()),
        }
    }

    /// Set a category's monthly budget; zero removes it. Use `TOTAL` for the overall budget.
    pub fn set_budget(&self, category: &str, monthly_cents: i64) -> Result<Budgets, String> {
        self.require_edit()?;
        let category = category.trim();
        if category.is_empty() {
            return Err("choose a category for the budget".into());
        }
        if monthly_cents < 0 {
            return Err("a budget can't be negative".into());
        }
        let mut b = self.budgets()?;
        if monthly_cents == 0 {
            b.monthly.remove(category);
        } else {
            b.monthly.insert(category.to_string(), monthly_cents);
        }
        let json = serde_json::to_string(&b).map_err(|e| e.to_string())?;
        self.db.set_setting(KEY, &json).map_err(|e| e.to_string())?;
        Ok(b)
    }

    /// Budget against spending for the period containing `day`. The overall row comes first
    /// when set; categories follow, most overspent first.
    pub fn budget_status(&self, day: &str, grain: &str) -> Result<Vec<BudgetRow>, String> {
        let g = grain_of(grain)?;
        let p = fin_core::PeriodKey::containing(g, crate::parse_day(day)?);
        let spent: BTreeMap<String, i64> = self
            .db
            .category_totals(p, Metric::Spending)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(c, m)| (c, m.cents()))
            .collect();
        let b = self.budgets()?;
        let mut rows = vec![];
        for (cat, monthly) in &b.monthly {
            if cat == TOTAL {
                continue;
            }
            let budget = convert_monthly(*monthly, grain)?;
            let s = spent.get(cat).copied().unwrap_or(0);
            rows.push(BudgetRow {
                category: cat.clone(),
                budget_cents: budget,
                spent_cents: s,
                remaining_cents: budget - s,
            });
        }
        rows.sort_by_key(|r| (r.remaining_cents, r.category.clone()));
        if let Some(t) = b.monthly.get(TOTAL) {
            let budget = convert_monthly(*t, grain)?;
            let s: i64 = spent.values().sum();
            rows.insert(
                0,
                BudgetRow {
                    category: "Total".into(),
                    budget_cents: budget,
                    spent_cents: s,
                    remaining_cents: budget - s,
                },
            );
        }
        Ok(rows)
    }
}
