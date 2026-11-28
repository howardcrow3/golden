//! Saved holdings and the check-up on them.
use crate::App;
use fin_ingest::holdings::{self, AssetKind};
use fin_plan::investments::{checkup, Checkup, Class, Holding, Risk};

const KEY: &str = "investments.holdings";

/// The checks `save_holdings` makes, so a caller can run them before changing anything else.
pub(crate) fn check_holdings(holdings: &[Holding]) -> Result<(), String> {
    if holdings.len() > 200 {
        return Err("Keep it to 200 holdings or fewer.".into());
    }
    if holdings.iter().any(|h| h.name.trim().is_empty()) {
        return Err("Give every holding a name.".into());
    }
    Ok(())
}

impl App {
    pub fn holdings(&self) -> Result<Vec<Holding>, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            Some(raw) if !raw.is_empty() => serde_json::from_str(&raw).map_err(|e| e.to_string()),
            _ => Ok(vec![]),
        }
    }

    pub fn save_holdings(&self, holdings: &[Holding]) -> Result<(), String> {
        self.require_edit()?;
        check_holdings(holdings)?;
        self.db
            .set_setting(
                KEY,
                &serde_json::to_string(holdings).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())
    }

    /// Read a broker's positions CSV into holdings for the person to look over. Nothing is saved
    /// here; the screen adds them to the list and saves with the rest.
    pub fn parse_holdings(&self, text: &str) -> Result<Vec<Holding>, String> {
        let parsed = holdings::parse(text).map_err(|e| e.to_string())?;
        if parsed.is_empty() {
            return Err("No holdings with a value were found in that file.".into());
        }
        Ok(parsed
            .into_iter()
            .map(|p| Holding {
                name: p.name,
                value_cents: p.value.cents(),
                class: match p.kind {
                    AssetKind::Stock => Class::Stock,
                    AssetKind::Bond => Class::Bond,
                    AssetKind::Cash => Class::Cash,
                    AssetKind::Other => Class::Other,
                },
                expense_bp: p.expense_bp,
                single_stock: p.single_stock,
            })
            .collect())
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
