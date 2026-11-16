//! Paying off debts: how long it takes and what the interest costs.
//!
//! Each month interest is added, every debt gets its minimum, and everything left over (the
//! extra payment plus the minimums of debts already paid off) goes to one target debt chosen by
//! the strategy: the highest rate first ("avalanche", least interest) or the smallest balance
//! first ("snowball", quickest wins). All money is integer cents.
use serde::{Deserialize, Serialize};

pub const MAX_MONTHS: u32 = 600;
/// $10 billion: past this a debt is treated as unpayable.
const RUNAWAY_CENTS: i64 = 1_000_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Debt {
    pub name: String,
    pub balance_cents: i64,
    /// Yearly rate in hundredths of a percent: 1999 is 19.99%.
    pub apr_bp: i64,
    pub min_cents: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Strategy {
    Avalanche,
    Snowball,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaidOff {
    pub name: String,
    /// Months from now until this debt reaches zero.
    pub month: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payoff {
    /// False when some debt never clears within 50 years (payments don't cover interest).
    pub feasible: bool,
    pub months: u32,
    pub total_interest_cents: i64,
    pub total_paid_cents: i64,
    pub order: Vec<PaidOff>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DebtError {
    #[error("{0} has a negative balance, rate or minimum payment.")]
    Negative(String),
    #[error("The extra payment can't be negative.")]
    NegativeExtra,
}

fn monthly_interest(balance: i64, apr_bp: i64) -> i64 {
    // balance * (apr_bp / 10000) / 12, rounded to the nearest cent.
    let n = balance as i128 * apr_bp as i128;
    ((n + 60_000) / 120_000) as i64
}

pub fn payoff(debts: &[Debt], extra_cents: i64, strategy: Strategy) -> Result<Payoff, DebtError> {
    if extra_cents < 0 {
        return Err(DebtError::NegativeExtra);
    }
    if let Some(d) = debts
        .iter()
        .find(|d| d.balance_cents < 0 || d.apr_bp < 0 || d.min_cents < 0)
    {
        return Err(DebtError::Negative(d.name.clone()));
    }
    let mut bal: Vec<i64> = debts.iter().map(|d| d.balance_cents).collect();
    let mut done: Vec<Option<u32>> = bal.iter().map(|b| (*b == 0).then_some(0)).collect();
    let (mut interest_total, mut paid_total) = (0i64, 0i64);
    let mut month = 0u32;
    // The money available each month is fixed: all minimums plus the extra. Paid-off debts'
    // minimums keep flowing to the target, which is what makes the strategies snowball.
    let budget: i64 = debts.iter().map(|d| d.min_cents).sum::<i64>() + extra_cents;

    while bal.iter().any(|b| *b > 0) && month < MAX_MONTHS {
        month += 1;
        for (i, d) in debts.iter().enumerate() {
            if bal[i] > 0 {
                let i_c = monthly_interest(bal[i], d.apr_bp);
                bal[i] = bal[i].saturating_add(i_c);
                interest_total = interest_total.saturating_add(i_c);
            }
        }
        // A balance that has run away (payments far below interest) will never clear; stop
        // before the numbers lose meaning.
        if bal.iter().any(|b| *b > RUNAWAY_CENTS) {
            break;
        }
        let mut left = budget;
        // Minimums first (never more than owed).
        for (i, d) in debts.iter().enumerate() {
            if bal[i] > 0 {
                let p = d.min_cents.min(bal[i]).min(left);
                bal[i] -= p;
                left -= p;
                paid_total += p;
            }
        }
        // Everything else to the target, then the next target, until the money runs out.
        while left > 0 {
            let target = (0..debts.len())
                .filter(|&i| bal[i] > 0)
                .min_by(|&a, &b| match strategy {
                    Strategy::Avalanche => debts[b]
                        .apr_bp
                        .cmp(&debts[a].apr_bp)
                        .then(bal[a].cmp(&bal[b])),
                    Strategy::Snowball => bal[a]
                        .cmp(&bal[b])
                        .then(debts[b].apr_bp.cmp(&debts[a].apr_bp)),
                });
            let Some(t) = target else { break };
            let p = left.min(bal[t]);
            bal[t] -= p;
            left -= p;
            paid_total += p;
        }
        for i in 0..debts.len() {
            if bal[i] == 0 && done[i].is_none() {
                done[i] = Some(month);
            }
        }
    }
    let feasible = bal.iter().all(|b| *b == 0);
    let mut order: Vec<PaidOff> = debts
        .iter()
        .zip(&done)
        .filter_map(|(d, m)| {
            m.map(|month| PaidOff {
                name: d.name.clone(),
                month,
            })
        })
        .collect();
    order.sort_by(|a, b| a.month.cmp(&b.month).then(a.name.cmp(&b.name)));
    Ok(Payoff {
        feasible,
        months: month,
        total_interest_cents: interest_total,
        total_paid_cents: paid_total,
        order,
    })
}
