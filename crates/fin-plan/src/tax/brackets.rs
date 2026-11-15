use fin_core::Money;
use serde::{Deserialize, Serialize};

/// One marginal bracket: income above `floor` (inclusive) is taxed at `rate`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(from = "(i64, f64)", into = "(i64, f64)")]
pub struct Bracket {
    pub floor: Money,
    pub rate: f64,
}

impl From<(i64, f64)> for Bracket {
    fn from((c, r): (i64, f64)) -> Self {
        Bracket {
            floor: Money::from_cents(c),
            rate: r,
        }
    }
}
impl From<Bracket> for (i64, f64) {
    fn from(b: Bracket) -> Self {
        (b.floor.cents(), b.rate)
    }
}

/// An ascending list of marginal brackets starting at zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BracketTable(pub Vec<Bracket>);

impl BracketTable {
    pub fn new(brackets: Vec<Bracket>) -> Self {
        debug_assert!(!brackets.is_empty() && brackets[0].floor == Money::ZERO);
        debug_assert!(brackets.windows(2).all(|w| w[0].floor < w[1].floor));
        BracketTable(brackets)
    }

    /// A single flat rate.
    pub fn flat(rate: f64) -> Self {
        BracketTable(vec![Bracket {
            floor: Money::ZERO,
            rate,
        }])
    }

    /// Tax on `income` (negative or zero income owes nothing).
    pub fn tax(&self, income: Money) -> Money {
        self.tax_stacked(Money::ZERO, income)
    }

    /// Tax on `amount` that sits on top of `base` income already taxed elsewhere
    /// (used for long-term gains stacked above ordinary income).
    pub fn tax_stacked(&self, base: Money, amount: Money) -> Money {
        if amount <= Money::ZERO {
            return Money::ZERO;
        }
        let base = base.max(Money::ZERO);
        let top = base + amount;
        let mut total = 0.0f64;
        for (i, b) in self.0.iter().enumerate() {
            let ceiling = self.0.get(i + 1).map(|n| n.floor);
            let lo = b.floor.max(base);
            let hi = ceiling.map_or(top, |c| c.min(top));
            if hi > lo {
                total += (hi - lo).cents() as f64 * b.rate;
            }
        }
        Money::from_cents(total.round() as i64)
    }

    /// The marginal rate that applies to the next dollar at `income`.
    pub fn marginal_rate(&self, income: Money) -> f64 {
        self.0
            .iter()
            .rev()
            .find(|b| income >= b.floor)
            .map_or(self.0[0].rate, |b| b.rate)
    }

    /// Dollars of room left before the next bracket begins, or None in the top bracket.
    pub fn room_in_bracket(&self, income: Money) -> Option<Money> {
        let income = income.max(Money::ZERO);
        self.0
            .iter()
            .find(|b| b.floor > income)
            .map(|b| b.floor - income)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn t() -> BracketTable {
        BracketTable::new(vec![
            Bracket {
                floor: Money::ZERO,
                rate: 0.10,
            },
            Bracket {
                floor: Money::from_dollars(10_000),
                rate: 0.20,
            },
            Bracket {
                floor: Money::from_dollars(50_000),
                rate: 0.30,
            },
        ])
    }

    #[test]
    fn marginal_tax() {
        assert_eq!(
            t().tax(Money::from_dollars(5_000)),
            Money::from_dollars(500)
        );
        assert_eq!(
            t().tax(Money::from_dollars(10_000)),
            Money::from_dollars(1_000)
        );
        assert_eq!(
            t().tax(Money::from_dollars(30_000)),
            Money::from_dollars(5_000)
        );
        assert_eq!(
            t().tax(Money::from_dollars(60_000)),
            Money::from_dollars(1_000 + 8_000 + 3_000)
        );
    }

    #[test]
    fn zero_and_negative() {
        assert_eq!(t().tax(Money::ZERO), Money::ZERO);
        assert_eq!(t().tax(Money::from_dollars(-10)), Money::ZERO);
    }

    #[test]
    fn stacking_uses_higher_brackets() {
        // 10k of gains stacked on 45k base: 5k at 20%, 5k at 30%.
        let tax = t().tax_stacked(Money::from_dollars(45_000), Money::from_dollars(10_000));
        assert_eq!(tax, Money::from_dollars(1_000 + 1_500));
    }

    #[test]
    fn marginal_rate_and_room() {
        assert_eq!(t().marginal_rate(Money::from_dollars(9_999)), 0.10);
        assert_eq!(t().marginal_rate(Money::from_dollars(10_000)), 0.20);
        assert_eq!(
            t().room_in_bracket(Money::from_dollars(9_000)),
            Some(Money::from_dollars(1_000))
        );
        assert_eq!(t().room_in_bracket(Money::from_dollars(70_000)), None);
    }

    #[test]
    fn flat_table() {
        assert_eq!(
            BracketTable::flat(0.05).tax(Money::from_dollars(1_000)),
            Money::from_dollars(50)
        );
    }

    proptest! {
        #[test]
        fn monotonic_and_bounded(a in 0i64..50_000_000, b in 0i64..50_000_000) {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            let (tl, th) = (t().tax(Money::from_cents(lo)), t().tax(Money::from_cents(hi)));
            prop_assert!(tl <= th);
            // Never more than the top rate, never less than the bottom rate.
            prop_assert!(th.cents() as f64 <= hi as f64 * 0.30 + 1.0);
            prop_assert!(th.cents() as f64 >= hi as f64 * 0.10 - 1.0);
        }

        #[test]
        fn stacking_splits_additively(base in 0i64..20_000_000, x in 0i64..20_000_000) {
            // tax(base + x) == tax(base) + stacked(base, x), within a cent of rounding.
            let whole = t().tax(Money::from_cents(base + x));
            let parts = t().tax(Money::from_cents(base)) + t().tax_stacked(Money::from_cents(base), Money::from_cents(x));
            prop_assert!((whole - parts).abs().cents() <= 1);
        }
    }
}
