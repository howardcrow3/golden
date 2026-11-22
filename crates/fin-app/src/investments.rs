//! Saved holdings and the check-up on them.
use crate::App;
use fin_plan::investments::{checkup, Checkup, Holding, Risk};

const KEY: &str = "investments.holdings";

impl App {
    pub fn holdings(&self) -> Result<Vec<Holding>, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            Some(raw) if !raw.is_empty() => serde_json::from_str(&raw).map_err(|e| e.to_string()),
            _ => Ok(vec![]),
        }
    }

    pub fn save_holdings(&self, holdings: &[Holding]) -> Result<(), String> {
        self.require_edit()?;
        if holdings.len() > 200 {
            return Err("Keep it to 200 holdings or fewer.".into());
        }
        if holdings.iter().any(|h| h.name.trim().is_empty()) {
            return Err("Give every holding a name.".into());
        }
        self.db
            .set_setting(
                KEY,
                &serde_json::to_string(holdings).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())
    }

    /// Check the saved holdings against a target for `risk`, using the first person's age.
    pub fn investment_checkup(&self, risk: Risk) -> Result<Checkup, String> {
        let today = (self.clock)();
        let h = self.household()?;
        let age = h
            .people
            .first()
            .and_then(|p| fin_core::NaiveDate::parse_from_str(&p.birth_date, "%Y-%m-%d").ok())
            .map_or(45, |b| fin_core::age_on(b, today));
        checkup(&self.holdings()?, risk, age)
    }
}
