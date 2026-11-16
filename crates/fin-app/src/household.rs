//! The household profile the plan runs on, plus the assumption options a person can pick from.
//! Stored as JSON in the settings table.
use crate::App;
use chrono::{Datelike, NaiveDate};
use fin_plan::params::Assumptions;
use fin_plan::quickplan::QuickPlanRequest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const KEY: &str = "household";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Household {
    pub birth_date: String,
    pub retire_age: i32,
    pub filing: String,
    pub state: String,
    pub salary_cents: i64,
    pub contribution_cents: i64,
    pub spending_cents: i64,
    pub cash_cents: i64,
    pub taxable_cents: i64,
    pub traditional_cents: i64,
    pub roth_cents: i64,
    pub ss_annual_cents: i64,
    /// Chosen option label per assumption key; missing means "the likeliest".
    #[serde(default)]
    pub picks: BTreeMap<String, String>,
}

impl Household {
    /// A placeholder household so a first-time user sees a working plan to edit.
    pub fn sample() -> Household {
        Household {
            birth_date: "1981-06-15".into(),
            retire_age: 62,
            filing: "married_joint".into(),
            state: "TX".into(),
            salary_cents: 18_000_000,
            contribution_cents: 2_350_000,
            spending_cents: 9_000_000,
            cash_cents: 3_000_000,
            taxable_cents: 15_000_000,
            traditional_cents: 60_000_000,
            roth_cents: 12_000_000,
            ss_annual_cents: 3_800_000,
            picks: BTreeMap::new(),
        }
    }

    pub fn age_on(&self, today: NaiveDate) -> Result<i32, String> {
        let b = NaiveDate::parse_from_str(&self.birth_date, "%Y-%m-%d")
            .map_err(|_| "Enter your birth date as YYYY-MM-DD.".to_string())?;
        let mut age = today.year() - b.year();
        if (today.month(), today.day()) < (b.month(), b.day()) {
            age -= 1;
        }
        Ok(age)
    }

    pub fn validate(&self, today: NaiveDate) -> Result<(), String> {
        let age = self.age_on(today)?;
        if !(18..=100).contains(&age) {
            return Err("Birth date should make you between 18 and 100.".into());
        }
        if self.retire_age < age {
            return Err("Retirement age must be at or after your current age.".into());
        }
        if self.state.len() != 2 || !self.state.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err("State should be a two-letter code like TX.".into());
        }
        if ![
            self.salary_cents,
            self.contribution_cents,
            self.spending_cents,
            self.cash_cents,
            self.taxable_cents,
            self.traditional_cents,
            self.roth_cents,
            self.ss_annual_cents,
        ]
        .iter()
        .all(|c| *c >= 0)
        {
            return Err("Amounts cannot be negative.".into());
        }
        let defaults = Assumptions::defaults();
        for (k, label) in &self.picks {
            let p = defaults.get(k).map_err(|e| e.to_string())?;
            if !p.options.iter().any(|o| &o.label == label) {
                return Err(format!("'{k}' has no option '{label}'."));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OptionRow {
    pub label: String,
    pub value: f64,
    pub likelihood: f64,
    pub probability: f64,
    pub source: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssumptionRow {
    pub key: String,
    pub options: Vec<OptionRow>,
    /// The label in use: the person's pick, else the likeliest option.
    pub active: String,
    pub picked: bool,
}

impl App {
    pub fn household(&self) -> Result<Household, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            Some(json) => serde_json::from_str(&json).map_err(|e| e.to_string()),
            None => Ok(Household::sample()),
        }
    }

    pub fn save_household(&self, h: &Household) -> Result<(), String> {
        self.require_edit()?;
        let mut h = h.clone();
        h.state = h.state.to_ascii_uppercase();
        h.validate((self.clock)())?;
        let json = serde_json::to_string(&h).map_err(|e| e.to_string())?;
        self.db.set_setting(KEY, &json).map_err(|e| e.to_string())
    }

    /// Choose an option for one assumption ("" goes back to the likeliest).
    pub fn set_pick(&self, key: &str, label: &str) -> Result<Household, String> {
        let mut h = self.household()?;
        if label.is_empty() {
            h.picks.remove(key);
        } else {
            h.picks.insert(key.to_string(), label.to_string());
        }
        self.save_household(&h)?;
        self.household()
    }

    pub fn assumptions(&self) -> Result<Vec<AssumptionRow>, String> {
        let h = self.household()?;
        let defaults = Assumptions::defaults();
        let mut rows = vec![];
        for key in defaults.keys() {
            let p = defaults.get(key).map_err(|e| e.to_string())?;
            let probs = p.probabilities();
            let picked = h.picks.get(key);
            let active = picked
                .cloned()
                .unwrap_or_else(|| p.options[p.likeliest()].label.clone());
            rows.push(AssumptionRow {
                key: key.to_string(),
                options: p
                    .options
                    .iter()
                    .zip(probs)
                    .map(|(o, pr)| OptionRow {
                        label: o.label.clone(),
                        value: o.value,
                        likelihood: o.likelihood,
                        probability: pr,
                        source: o.source.clone(),
                    })
                    .collect(),
                active,
                picked: picked.is_some(),
            });
        }
        Ok(rows)
    }

    /// The saved household as an engine request, with age worked out from the birth date.
    pub fn plan_request(&self, today: NaiveDate, runs: usize) -> Result<QuickPlanRequest, String> {
        let h = self.household()?;
        h.validate(today)?;
        let mut v = serde_json::to_value(&h).map_err(|e| e.to_string())?;
        let o = v.as_object_mut().ok_or("household is not an object")?;
        o.remove("birthDate");
        o.insert("currentYear".into(), today.year().into());
        o.insert("age".into(), h.age_on(today)?.into());
        o.insert("runs".into(), runs.into());
        serde_json::from_value(v).map_err(|e| e.to_string())
    }
}
