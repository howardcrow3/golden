//! "Getting started": the handful of things that make Golden useful, and which are done.
//!
//! Shown on the Overview until everything is ticked. Each step is judged from what is actually
//! stored, so it ticks itself when the person does the thing anywhere in the app.
use crate::App;
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetupStep {
    pub key: &'static str,
    pub title: &'static str,
    pub done: bool,
    /// Which screen does it, matching the sidebar name.
    pub page: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Setup {
    pub steps: Vec<SetupStep>,
    pub done: usize,
    pub complete: bool,
}

impl App {
    pub fn setup_progress(&self) -> Result<Setup, String> {
        let has_accounts = !self.accounts()?.is_empty();
        let categorized = has_accounts
            && self
                .db
                .uncategorized_txns()
                .map_err(|e| e.to_string())?
                .is_empty();
        let steps = vec![
            SetupStep {
                key: "import",
                title: "Import a bank or card statement",
                done: has_accounts,
                page: "Import",
            },
            SetupStep {
                key: "categories",
                title: "Give every transaction a category",
                done: categorized,
                page: "Transactions",
            },
            SetupStep {
                key: "household",
                title: "Tell Golden about your household for the retirement plan",
                done: self.household_is_saved()?,
                page: "Retirement",
            },
            SetupStep {
                key: "budget",
                title: "Set a monthly budget for a category",
                done: !self.budgets()?.monthly.is_empty(),
                page: "Overview",
            },
            SetupStep {
                key: "goal",
                title: "Add a savings goal",
                done: !self.goals()?.is_empty(),
                page: "Overview",
            },
        ];
        let done = steps.iter().filter(|s| s.done).count();
        Ok(Setup {
            complete: done == steps.len(),
            done,
            steps,
        })
    }
}
