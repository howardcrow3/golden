//! Where the next savings dollars should go, in a fixed and explained order.
//!
//! 1. Emergency cash: six months of spending.
//! 2. Workplace retirement plan, up to the yearly limit per person (money already contributed
//!    through income sources counts against the limit).
//! 3. IRA, up to the yearly limit per person.
//! 4. Everything else to a taxable account.
//!
//! This is a rules-based starting point, not advice: it does not know about employer matches,
//! HSA eligibility, income limits on Roth IRAs, or catch-up contributions.
use crate::App;
use serde::Serialize;

/// 2026 limits, in cents.
pub const WORKPLACE_LIMIT_CENTS: i64 = 2_450_000;
pub const IRA_LIMIT_CENTS: i64 = 750_000;
pub const EMERGENCY_MONTHS: i64 = 6;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AllocationStep {
    pub name: String,
    pub cents: i64,
    pub why: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Allocation {
    pub yearly_savings_cents: i64,
    pub steps: Vec<AllocationStep>,
}

impl App {
    /// Suggest how to split `yearly_savings_cents` of new saving.
    pub fn suggest_allocation(&self, yearly_savings_cents: i64) -> Result<Allocation, String> {
        if yearly_savings_cents < 0 {
            return Err("Savings can't be negative.".into());
        }
        let h = self.household()?;
        let people = h.people.len().max(1) as i64;
        let mut left = yearly_savings_cents;
        let mut steps = vec![];
        let mut take = |name: &str, want: i64, why: String| {
            let c = want.min(left).max(0);
            if c > 0 {
                left -= c;
                steps.push(AllocationStep {
                    name: name.into(),
                    cents: c,
                    why,
                });
            }
        };

        let target = h.spending_cents / 12 * EMERGENCY_MONTHS;
        let gap = (target - h.cash_cents).max(0);
        take(
            "Emergency cash",
            gap,
            format!("Cash covers {EMERGENCY_MONTHS} months of spending first, so a bad month never forces a sale."),
        );

        let contributed: i64 = h.streams.iter().map(|s| s.contribution_cents).sum();
        let room = (WORKPLACE_LIMIT_CENTS * people - contributed).max(0);
        take(
            "Workplace retirement plan",
            room,
            "Tax-advantaged room, and any employer match is only earned by contributing here."
                .into(),
        );
        take(
            "IRA",
            IRA_LIMIT_CENTS * people,
            "More tax-advantaged room once the workplace plan is full.".into(),
        );
        take(
            "Taxable account",
            i64::MAX,
            "Whatever is left, with no limits and full access.".into(),
        );
        Ok(Allocation {
            yearly_savings_cents,
            steps,
        })
    }
}
