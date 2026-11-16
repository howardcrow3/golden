//! Paystubs: read one, show what it says, and (when asked) turn it into an income source.
use crate::household::StreamRow;
use crate::App;
use chrono::Datelike;
use fin_ingest::paystub::{self, Paystub};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaystubSummary {
    pub employer: String,
    pub pay_date: Option<String>,
    /// weekly, biweekly, semimonthly, monthly, or "" when the pay period isn't on the stub.
    pub frequency: String,
    pub periods_per_year: i64,
    pub gross_cents: i64,
    pub net_cents: i64,
    pub federal_tax_cents: i64,
    pub state_tax_cents: i64,
    pub social_security_cents: i64,
    pub medicare_cents: i64,
    pub retirement_cents: i64,
    pub benefits_cents: i64,
    pub annual_gross_cents: i64,
    pub annual_retirement_cents: i64,
    /// Share of gross that goes to income tax, in hundredths of a percent (1250 = 12.50%).
    pub income_tax_rate_bp: i64,
}

fn frequency(s: &Paystub) -> (&'static str, i64) {
    let (Some(a), Some(b)) = (s.period_start, s.period_end) else {
        return ("", 0);
    };
    match (b - a).num_days() + 1 {
        5..=8 => ("weekly", 52),
        12..=16 => {
            // 15 days ending on the 15th or the last day of the month is a semimonthly stub.
            let end_of_month = (b + chrono::Duration::days(1)).day() == 1;
            if (a.day() == 1 && b.day() == 15) || (a.day() == 16 && end_of_month) {
                ("semimonthly", 24)
            } else {
                ("biweekly", 26)
            }
        }
        27..=31 => ("monthly", 12),
        _ => ("", 0),
    }
}

impl App {
    /// Read paystub text and explain it. Changes nothing.
    pub fn read_paystub(&self, text: &str) -> Result<PaystubSummary, String> {
        let s = paystub::parse(text).map_err(|e| e.to_string())?;
        let (freq, n) = frequency(&s);
        let gross = s.gross.cents();
        let tax = s.federal_tax.cents() + s.state_tax.cents();
        Ok(PaystubSummary {
            employer: s.employer.clone(),
            pay_date: s.pay_date.map(|d| d.to_string()),
            frequency: freq.into(),
            periods_per_year: n,
            gross_cents: gross,
            net_cents: s.net.cents(),
            federal_tax_cents: s.federal_tax.cents(),
            state_tax_cents: s.state_tax.cents(),
            social_security_cents: s.social_security.cents(),
            medicare_cents: s.medicare.cents(),
            retirement_cents: s.retirement.cents(),
            benefits_cents: s.benefits.cents(),
            annual_gross_cents: gross * n,
            annual_retirement_cents: s.retirement.cents() * n,
            income_tax_rate_bp: if gross > 0 { tax * 10_000 / gross } else { 0 },
        })
    }

    /// Add (or update) an income source for `person` from a paystub. Needs a known pay period.
    pub fn apply_paystub(&self, text: &str, person: usize) -> Result<StreamRow, String> {
        self.require_edit()?;
        let p = self.read_paystub(text)?;
        if p.periods_per_year == 0 {
            return Err(
                "The pay period isn't on this stub, so I can't tell how often you're paid.".into(),
            );
        }
        let mut h = self.household()?;
        if person >= h.people.len() {
            return Err("Choose one of the people in your household.".into());
        }
        let label = if p.employer.is_empty() {
            "Salary".to_string()
        } else {
            format!("{} salary", p.employer)
        };
        let row = StreamRow {
            label: label.clone(),
            person,
            annual_cents: p.annual_gross_cents,
            start_year: None,
            end_year: None,
            contribution_cents: p.annual_retirement_cents,
        };
        match h
            .streams
            .iter_mut()
            .find(|s| s.person == person && s.label == label)
        {
            Some(existing) => *existing = row.clone(),
            None => h.streams.push(row.clone()),
        }
        self.save_household(&h)?;
        Ok(row)
    }
}
