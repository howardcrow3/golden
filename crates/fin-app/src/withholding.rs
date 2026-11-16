//! Withholding check: is enough tax being taken out of paychecks?
//!
//! Each paystub applied to the plan records what that job withholds per year. This compares the
//! total with an estimate of the year's tax on the household's taxable pay (gross minus
//! pre-tax retirement saving), so a person can fix a W-4 before April instead of after. It
//! checks every ongoing income source; if any lacks a paystub it says which, rather than
//! guessing. It looks at wages only, so other income (interest, gains) is not in the estimate.
use crate::App;
use fin_plan::estimate::{estimate, TaxRequest};
use serde::{Deserialize, Serialize};

const KEY: &str = "withholding";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Withheld {
    label: String,
    person: usize,
    federal_cents: i64,
    state_cents: i64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Gap {
    pub estimated_cents: i64,
    pub withheld_cents: i64,
    /// Withheld minus estimated: positive means a refund, negative means a bill.
    pub difference_cents: i64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WithholdingCheck {
    pub taxable_wages_cents: i64,
    pub federal: Gap,
    pub state: Gap,
    /// Income sources with no paystub on file, so the check can't be trusted yet.
    pub missing: Vec<String>,
    pub state_is_estimate: bool,
}

pub(crate) fn record(
    app: &App,
    label: &str,
    person: usize,
    fed: i64,
    state: i64,
) -> Result<(), String> {
    let mut all = load(app)?;
    let w = Withheld {
        label: label.into(),
        person,
        federal_cents: fed,
        state_cents: state,
    };
    match all
        .iter_mut()
        .find(|x| x.person == person && x.label == label)
    {
        Some(e) => *e = w,
        None => all.push(w),
    }
    let json = serde_json::to_string(&all).map_err(|e| e.to_string())?;
    app.db().set_setting(KEY, &json).map_err(|e| e.to_string())
}

fn load(app: &App) -> Result<Vec<Withheld>, String> {
    match app.db().get_setting(KEY).map_err(|e| e.to_string())? {
        Some(s) => {
            serde_json::from_str(&s).map_err(|e| format!("saved withholding is unreadable: {e}"))
        }
        None => Ok(vec![]),
    }
}

impl App {
    pub fn withholding_check(&self) -> Result<WithholdingCheck, String> {
        let h = self.household()?;
        let recorded = load(self)?;
        // Ongoing pay only: one-time amounts (a bonus) have their own withholding rules.
        let jobs: Vec<_> = h
            .streams
            .iter()
            .filter(|s| s.start_year.is_none() && s.end_year.is_none() && s.annual_cents > 0)
            .collect();
        let wages: i64 = jobs
            .iter()
            .map(|s| (s.annual_cents - s.contribution_cents).max(0))
            .sum();
        let mut missing = vec![];
        let (mut fed, mut state) = (0i64, 0i64);
        for s in &jobs {
            match recorded
                .iter()
                .find(|r| r.person == s.person && r.label == s.label)
            {
                Some(r) => {
                    fed += r.federal_cents;
                    state += r.state_cents;
                }
                None => missing.push(s.label.clone()),
            }
        }
        let est = estimate(&TaxRequest {
            wages_cents: wages,
            filing: serde_json::from_value(serde_json::Value::String(h.filing.clone()))
                .map_err(|_| "Choose a filing status.".to_string())?,
            state: h.state.clone(),
        })?;
        let gap = |estimated: i64, withheld: i64| Gap {
            estimated_cents: estimated,
            withheld_cents: withheld,
            difference_cents: withheld - estimated,
        };
        Ok(WithholdingCheck {
            taxable_wages_cents: wages,
            federal: gap(est.federal_cents, fed),
            state: gap(est.state_cents, state),
            missing,
            state_is_estimate: est.state_is_estimate,
        })
    }
}
