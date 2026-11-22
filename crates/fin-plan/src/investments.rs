//! A check-up for a set of holdings: how the mix compares with a target for your risk profile,
//! what the fund fees cost over time, and whether too much sits in one holding.
use serde::{Deserialize, Serialize};

/// A single holding is flagged above this share of the portfolio.
pub const CONCENTRATION_LIMIT: f64 = 0.10;
/// An individual company stock is flagged above this lower share.
pub const SINGLE_STOCK_LIMIT: f64 = 0.05;
/// A fee is called high above this yearly share of the holding.
pub const HIGH_FEE: f64 = 0.0075;
/// Return assumed for the long-run fee comparison, before fees.
pub const FEE_RETURN: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Stock,
    Bond,
    Cash,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    Conservative,
    Moderate,
    Aggressive,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Holding {
    pub name: String,
    pub value_cents: i64,
    pub class: Class,
    /// Hundredths of a percent a year: 7 is 0.07%.
    #[serde(default)]
    pub expense_bp: i64,
    /// An individual company, not a fund.
    #[serde(default)]
    pub single_stock: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Slice {
    pub class: Class,
    pub share: f64,
    pub target: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Flag {
    pub name: String,
    pub share: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Checkup {
    pub total_cents: i64,
    pub mix: Vec<Slice>,
    /// Stock share now minus the target; positive means more risk than your profile.
    pub stock_gap: f64,
    pub weighted_fee: f64,
    pub yearly_fee_cents: i64,
    /// What fees take over the next 20 years compared with paying nothing, at a 5% return.
    pub fee_cost_20y_cents: i64,
    pub flags: Vec<Flag>,
}

/// Target mix: stocks by risk profile, less 10 points from 60 on and again from 70, the rest in
/// bonds with a little cash. A rule of thumb, not advice.
pub fn target(risk: Risk, age: i32) -> [(Class, f64); 3] {
    let base: f64 = match risk {
        Risk::Conservative => 0.40,
        Risk::Moderate => 0.60,
        Risk::Aggressive => 0.80,
    };
    let older = (if age >= 60 { 0.10 } else { 0.0 }) + (if age >= 70 { 0.10 } else { 0.0 });
    let stock = (base - older).max(0.10);
    let cash = 0.05;
    [
        (Class::Stock, stock),
        (Class::Bond, 1.0 - stock - cash),
        (Class::Cash, cash),
    ]
}

pub fn checkup(holdings: &[Holding], risk: Risk, age: i32) -> Result<Checkup, String> {
    if holdings.is_empty() {
        return Err("Add at least one holding.".into());
    }
    if holdings
        .iter()
        .any(|h| h.value_cents < 0 || h.expense_bp < 0)
    {
        return Err("Values and fees cannot be negative.".into());
    }
    if holdings.iter().any(|h| h.expense_bp > 500) {
        return Err("A fee above 5% a year looks like a typo.".into());
    }
    let total: i64 = holdings.iter().map(|h| h.value_cents).sum();
    if total <= 0 {
        return Err("The holdings add up to nothing.".into());
    }
    let t = total as f64;
    let share_of = |c: Class| {
        holdings
            .iter()
            .filter(|h| h.class == c)
            .map(|h| h.value_cents)
            .sum::<i64>() as f64
            / t
    };
    let tgt = target(risk, age);
    let mut mix: Vec<Slice> = tgt
        .iter()
        .map(|(c, tg)| Slice {
            class: *c,
            share: share_of(*c),
            target: *tg,
        })
        .collect();
    mix.push(Slice {
        class: Class::Other,
        share: share_of(Class::Other),
        target: 0.0,
    });
    let stock_gap = share_of(Class::Stock) - tgt[0].1;
    let fee: f64 = holdings
        .iter()
        .map(|h| h.value_cents as f64 * h.expense_bp as f64 / 10_000.0)
        .sum();
    let weighted_fee = fee / t;
    let grown = |rate: f64| t * (1.0 + rate).powi(20);
    let fee_cost = grown(FEE_RETURN) - grown(FEE_RETURN - weighted_fee);
    let mut flags = Vec::new();
    for h in holdings {
        let share = h.value_cents as f64 / t;
        let limit = if h.single_stock {
            SINGLE_STOCK_LIMIT
        } else {
            CONCENTRATION_LIMIT
        };
        if share > limit && holdings.len() > 1 && h.class != Class::Cash {
            flags.push(Flag {
                name: h.name.clone(),
                share,
                reason: if h.single_stock {
                    "One company's stock above 5% of the portfolio".into()
                } else {
                    "Above 10% of the portfolio in one holding".into()
                },
            });
        }
        if h.expense_bp as f64 / 10_000.0 > HIGH_FEE {
            flags.push(Flag {
                name: h.name.clone(),
                share,
                reason: format!("Fee of {:.2}% a year is high", h.expense_bp as f64 / 100.0),
            });
        }
    }
    Ok(Checkup {
        total_cents: total,
        mix,
        stock_gap,
        weighted_fee,
        yearly_fee_cents: fee.round() as i64,
        fee_cost_20y_cents: fee_cost.round() as i64,
        flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(name: &str, dollars: i64, class: Class, bp: i64, single: bool) -> Holding {
        Holding {
            name: name.into(),
            value_cents: dollars * 100,
            class,
            expense_bp: bp,
            single_stock: single,
        }
    }

    #[test]
    fn targets_sum_to_one_and_fall_with_age() {
        for r in [Risk::Conservative, Risk::Moderate, Risk::Aggressive] {
            for a in [30, 62, 75] {
                let t = target(r, a);
                assert!((t.iter().map(|x| x.1).sum::<f64>() - 1.0).abs() < 1e-9);
            }
        }
        assert!(target(Risk::Moderate, 75)[0].1 < target(Risk::Moderate, 40)[0].1);
        assert!(target(Risk::Conservative, 90)[0].1 >= 0.10);
    }

    #[test]
    fn mix_gap_and_total() {
        let c = checkup(
            &[
                h("Index", 70_000, Class::Stock, 3, false),
                h("Bonds", 30_000, Class::Bond, 5, false),
            ],
            Risk::Moderate,
            45,
        )
        .unwrap();
        assert_eq!(c.total_cents, 10_000_000);
        assert!((c.stock_gap - 0.10).abs() < 1e-9);
        assert!((c.mix[0].share - 0.7).abs() < 1e-9);
    }

    #[test]
    fn fees_are_weighted_and_cost_more_over_time() {
        let cheap = checkup(
            &[h("A", 100_000, Class::Stock, 5, false)],
            Risk::Moderate,
            40,
        )
        .unwrap();
        let dear = checkup(
            &[h("A", 100_000, Class::Stock, 100, false)],
            Risk::Moderate,
            40,
        )
        .unwrap();
        assert_eq!(dear.yearly_fee_cents, 100_000);
        assert!(dear.fee_cost_20y_cents > cheap.fee_cost_20y_cents * 10);
        assert!(dear.flags.iter().any(|f| f.reason.contains("high")));
        assert!(cheap.flags.is_empty());
    }

    #[test]
    fn concentration_flags_use_a_lower_limit_for_single_stocks() {
        let c = checkup(
            &[
                h("Employer stock", 8_000, Class::Stock, 0, true),
                h("Fund", 92_000, Class::Stock, 3, false),
            ],
            Risk::Aggressive,
            40,
        )
        .unwrap();
        assert!(c.flags.iter().any(|f| f.name == "Employer stock"));
        assert!(c.flags.iter().any(|f| f.name == "Fund"));
        // Cash is never flagged, and a lone holding is not "concentrated".
        let only = checkup(
            &[h("Cash", 5_000, Class::Cash, 0, false)],
            Risk::Moderate,
            40,
        )
        .unwrap();
        assert!(only.flags.is_empty());
    }

    #[test]
    fn bad_input_is_a_readable_error() {
        assert!(checkup(&[], Risk::Moderate, 40).is_err());
        assert!(checkup(&[h("A", 0, Class::Stock, 0, false)], Risk::Moderate, 40).is_err());
        assert!(
            checkup(&[h("A", 10, Class::Stock, 900, false)], Risk::Moderate, 40)
                .unwrap_err()
                .contains("typo")
        );
        let mut neg = h("A", 10, Class::Stock, 0, false);
        neg.value_cents = -1;
        assert!(checkup(&[neg], Risk::Moderate, 40).is_err());
    }
}
