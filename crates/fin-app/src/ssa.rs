//! Social Security Statement: read the benefit estimates and set a person's Social Security.
use crate::App;
use fin_ingest::ssa;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SsaSummary {
    pub monthly_at_62_cents: Option<i64>,
    pub monthly_at_full_retirement_cents: Option<i64>,
    pub full_retirement_age: Option<u32>,
    pub monthly_at_70_cents: Option<i64>,
}

fn summary(s: &ssa::SsaStatement) -> SsaSummary {
    SsaSummary {
        monthly_at_62_cents: s.at_62.map(|m| m.cents()),
        monthly_at_full_retirement_cents: s.at_full_retirement.map(|m| m.cents()),
        full_retirement_age: s.full_retirement_age,
        monthly_at_70_cents: s.at_70.map(|m| m.cents()),
    }
}

impl App {
    /// Read statement text and show what it holds. Changes nothing.
    pub fn read_ssa_statement(&self, text: &str) -> Result<SsaSummary, String> {
        ssa::parse(text)
            .map(|s| summary(&s))
            .map_err(|e| e.to_string())
    }

    /// Set `person`'s yearly Social Security from the statement. `claim` is `"62"`, `"fra"` or `"70"`.
    /// Returns the yearly amount in cents that was saved.
    pub fn apply_ssa_statement(
        &self,
        text: &str,
        person: usize,
        claim: &str,
    ) -> Result<i64, String> {
        self.require_edit()?;
        let s = ssa::parse(text).map_err(|e| e.to_string())?;
        let (pick, name) = match claim {
            "62" => (s.at_62, "age 62"),
            "fra" => (s.at_full_retirement, "full retirement age"),
            "70" => (s.at_70, "age 70"),
            _ => return Err("Choose age 62, full retirement age or age 70.".into()),
        };
        let monthly = pick.ok_or_else(|| format!("This statement has no amount for {name}."))?;
        let mut h = self.household()?;
        let p = h
            .people
            .get_mut(person)
            .ok_or("Choose one of the people in your household.")?;
        p.ss_annual_cents = monthly.cents() * 12;
        let yearly = p.ss_annual_cents;
        self.save_household(&h)?;
        Ok(yearly)
    }
}
