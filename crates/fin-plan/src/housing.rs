//! A home in the plan: the mortgage paying off, and a sale (with a move to something smaller
//! or to renting). Everything is turned into yearly extra costs (or untaxed inflows when
//! negative) that the projection already understands, in today's dollars.
//!
//! The household's yearly spending is assumed to already include the mortgage payment, so the
//! mortgage shows up as a saving once it is paid off. The home itself is assumed to keep pace
//! with inflation (no real growth), and prices and debts are put into today's dollars with
//! `INFLATION`.
use crate::projection::OneTimeCost;
use crate::tax::FilingStatus;
use fin_core::Money;

/// Yearly inflation used to express a fixed mortgage in today's dollars.
pub const INFLATION: f64 = 0.025;

#[derive(Debug, Clone, PartialEq)]
pub struct Home {
    /// What the home would sell for today.
    pub value: Money,
    /// What was paid for it plus improvements, to work out the gain.
    pub basis: Money,
    pub mortgage_balance: Money,
    /// Yearly interest rate on the mortgage, such as 0.045.
    pub mortgage_rate: f64,
    pub mortgage_years_left: i32,
    /// Sell in this year, if at all.
    pub sell_year: Option<i32>,
    /// Agent fees and closing costs as a share of the price.
    pub selling_cost: f64,
    /// Cost of the home bought instead (a smaller one); zero to rent.
    pub replacement_cost: Money,
    /// Yearly rent after the sale, in today's dollars, when renting.
    pub rent_after: Money,
}

/// Level monthly payment for a loan.
pub fn monthly_payment(balance: Money, rate: f64, years: i32) -> Money {
    let n = (years * 12).max(1) as f64;
    let b = balance.cents().max(0) as f64;
    if rate <= 0.0 {
        return Money::from_cents((b / n).round() as i64);
    }
    let r = rate / 12.0;
    Money::from_cents((b * r / (1.0 - (1.0 + r).powf(-n))).round() as i64)
}

/// What is still owed after `years` of payments, in dollars of that year.
pub fn balance_after(balance: Money, rate: f64, years_left: i32, years: i32) -> Money {
    if years >= years_left {
        return Money::ZERO;
    }
    let pay = monthly_payment(balance, rate, years_left).cents() as f64;
    let r = rate / 12.0;
    let k = (years * 12) as f64;
    let b = balance.cents() as f64;
    let owed = if rate <= 0.0 {
        b - pay * k
    } else {
        b * (1.0 + r).powf(k) - pay * (((1.0 + r).powf(k) - 1.0) / r)
    };
    Money::from_cents(owed.max(0.0).round() as i64)
}

/// What a savings balance held in an offset account does to a mortgage: interest is charged only
/// on what is owed beyond the offset, while the payment stays what it was. Returns the months
/// until the loan is gone and the interest paid over that time. The offset balance is assumed to
/// stay put (it earns nothing, which is the point of an offset).
pub fn offset_payoff(balance: Money, rate: f64, years_left: i32, offset: Money) -> (i32, Money) {
    let pay = monthly_payment(balance, rate, years_left).cents() as f64;
    let r = rate / 12.0;
    let off = offset.cents().max(0) as f64;
    let mut owed = balance.cents().max(0) as f64;
    let mut interest = 0.0_f64;
    let mut months = 0;
    while owed > 0.5 && months < 12 * 100 {
        let i = ((owed - off).max(0.0)) * r;
        if pay <= i && owed > off {
            // The payment no longer covers interest: never pays off.
            return (months, Money::from_cents(interest.round() as i64));
        }
        interest += i;
        owed = owed + i - pay;
        months += 1;
    }
    (months, Money::from_cents(interest.round() as i64))
}

/// The gain on a primary home that is taxed: over $250,000 ($500,000 if married filing
/// jointly). Source: IRC section 121. Taxed at 15% here, as a middle guess.
pub fn taxable_gain_tax(fs: FilingStatus, gain: Money) -> Money {
    let exclusion = if fs == FilingStatus::MarriedJoint {
        500_000
    } else {
        250_000
    };
    (gain - Money::from_dollars(exclusion))
        .max(Money::ZERO)
        .mul_rate(0.15)
}

fn real(m: Money, years: i32) -> Money {
    m.mul_rate(1.0 / (1.0 + INFLATION).powi(years.max(0)))
}

/// Extra yearly costs from `start_year` to `last_year` that the home causes.
pub fn yearly_costs(
    h: &Home,
    fs: FilingStatus,
    start_year: i32,
    last_year: i32,
) -> Vec<OneTimeCost> {
    let mut out = Vec::new();
    let payoff = h.mortgage_years_left.max(0);
    // The payment keeps its dollar amount, so its weight in today's dollars shrinks.
    let payment = Money::from_cents(
        monthly_payment(h.mortgage_balance, h.mortgage_rate, payoff).cents() * 12,
    );
    let owed = h.mortgage_balance > Money::ZERO && payoff > 0;
    let sale_t = h.sell_year.map(|y| y - start_year);
    // Payments end at payoff or at the sale, whichever comes first.
    let stop_t = match sale_t {
        Some(s) if !owed || s < payoff => s.max(0),
        _ => payoff,
    };
    if owed || sale_t.is_some() {
        for t in stop_t..=(last_year - start_year) {
            if t < 0 {
                continue;
            }
            // Spending already pays for the mortgage; once it stops, that part is released.
            let mut amount = if owed {
                -real(payment, stop_t)
            } else {
                Money::ZERO
            };
            if sale_t.is_some_and(|s| t > s) {
                amount += real(h.rent_after, 0);
            } else if sale_t.is_none() && !owed {
                amount = Money::ZERO;
            }
            if amount != Money::ZERO {
                out.push(OneTimeCost {
                    year: start_year + t,
                    amount,
                });
            }
        }
    }
    if let Some(s) = sale_t.filter(|s| *s >= 0 && start_year + *s <= last_year) {
        let sale_year = start_year + s;
        let price = h.value;
        let fees = price.mul_rate(h.selling_cost);
        // The gain is taxed in the dollars of the sale year: the basis and the exclusion are
        // fixed amounts, while the price has kept pace with inflation since today.
        let nominal_price = price.mul_rate((1.0 + INFLATION).powi(s));
        let nominal_gain = nominal_price - nominal_price.mul_rate(h.selling_cost) - h.basis;
        let tax = real(taxable_gain_tax(fs, nominal_gain), s);
        let owed_then = real(
            balance_after(h.mortgage_balance, h.mortgage_rate, payoff, s),
            s,
        );
        let proceeds = price - fees - tax - owed_then - h.replacement_cost;
        merge(&mut out, sale_year, -proceeds);
    }
    out
}

fn merge(out: &mut Vec<OneTimeCost>, year: i32, amount: Money) {
    if let Some(c) = out.iter_mut().find(|c| c.year == year) {
        c.amount += amount;
    } else {
        out.push(OneTimeCost { year, amount });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> Home {
        Home {
            value: Money::from_dollars(600_000),
            basis: Money::from_dollars(300_000),
            mortgage_balance: Money::from_dollars(200_000),
            mortgage_rate: 0.04,
            mortgage_years_left: 15,
            sell_year: None,
            selling_cost: 0.06,
            replacement_cost: Money::ZERO,
            rent_after: Money::ZERO,
        }
    }

    fn at(v: &[OneTimeCost], y: i32) -> Money {
        v.iter()
            .find(|c| c.year == y)
            .map_or(Money::ZERO, |c| c.amount)
    }

    #[test]
    fn an_offset_account_shortens_the_loan_and_saves_interest() {
        let b = Money::from_dollars(200_000);
        let (m0, i0) = offset_payoff(b, 0.04, 15, Money::ZERO);
        assert!((m0 - 180).abs() <= 1, "{m0}");
        let (m1, i1) = offset_payoff(b, 0.04, 15, Money::from_dollars(50_000));
        assert!(m1 < m0 - 12, "{m1} vs {m0}");
        assert!(i1 < i0, "{i1:?} vs {i0:?}");
        // Offsetting the whole loan means no interest at all.
        let (_, i2) = offset_payoff(b, 0.04, 15, b);
        assert_eq!(i2, Money::ZERO);
    }

    #[test]
    fn no_interest_without_a_rate_and_negative_offsets_are_ignored() {
        let b = Money::from_dollars(12_000);
        let (m, i) = offset_payoff(b, 0.0, 1, Money::from_dollars(-5));
        assert_eq!(m, 12);
        assert_eq!(i, Money::ZERO);
    }

    #[test]
    fn payment_and_balance_follow_the_loan_formulas() {
        let p = monthly_payment(Money::from_dollars(200_000), 0.04, 15);
        // Standard: about $1,479 a month.
        assert!((p.cents() - 147_934).abs() < 100, "{p:?}");
        assert_eq!(
            balance_after(Money::from_dollars(200_000), 0.04, 15, 15),
            Money::ZERO
        );
        let half = balance_after(Money::from_dollars(200_000), 0.04, 15, 8);
        assert!(half > Money::from_dollars(80_000) && half < Money::from_dollars(120_000));
        assert_eq!(
            monthly_payment(Money::from_dollars(12_000), 0.0, 1),
            Money::from_dollars(1_000)
        );
    }

    #[test]
    fn paying_off_releases_the_payment_every_year_after() {
        let v = yearly_costs(&home(), FilingStatus::MarriedJoint, 2026, 2060);
        assert_eq!(at(&v, 2040), Money::ZERO);
        assert!(at(&v, 2041) < Money::ZERO);
        assert!(at(&v, 2060) < Money::ZERO);
        // Roughly the real value of $17,750 a year.
        let freed = -at(&v, 2045);
        assert!(
            freed > Money::from_dollars(11_000) && freed < Money::from_dollars(14_000),
            "{freed:?}"
        );
    }

    #[test]
    fn selling_brings_in_proceeds_after_fees_tax_and_the_loan() {
        let mut h = home();
        h.sell_year = Some(2030);
        let v = yearly_costs(&h, FilingStatus::MarriedJoint, 2026, 2060);
        // 600k - 36k fees - ~139k owed (in today's dollars), no tax under the 500k exclusion, plus
        // the year's released mortgage payment.
        let proceeds = -at(&v, 2030);
        assert!(
            proceeds > Money::from_dollars(400_000) && proceeds < Money::from_dollars(460_000),
            "{proceeds:?}"
        );
    }

    #[test]
    fn a_late_sale_taxes_the_nominal_gain_over_the_fixed_exclusion() {
        // No loan, no fees, single. Sold in 20 years the home fetches 600k * 1.025^20 = ~983k
        // in that year's dollars, a ~683k gain over the 300k paid: ~433k is taxed, ~65k of tax
        // then, ~39.7k in today's dollars. The exclusion and the basis are not indexed.
        let mut h = home();
        h.mortgage_balance = Money::ZERO;
        h.selling_cost = 0.0;
        h.sell_year = Some(2046);
        let v = yearly_costs(&h, FilingStatus::Single, 2026, 2060);
        let proceeds = -at(&v, 2046);
        assert!(
            proceeds > Money::from_dollars(558_000) && proceeds < Money::from_dollars(562_000),
            "{proceeds:?}"
        );
    }

    #[test]
    fn a_big_gain_is_taxed_above_the_exclusion() {
        assert_eq!(
            taxable_gain_tax(FilingStatus::MarriedJoint, Money::from_dollars(400_000)),
            Money::ZERO
        );
        assert_eq!(
            taxable_gain_tax(FilingStatus::Single, Money::from_dollars(350_000)),
            Money::from_dollars(15_000)
        );
    }

    #[test]
    fn buying_smaller_or_renting_after_a_sale() {
        let mut h = home();
        h.sell_year = Some(2030);
        h.replacement_cost = Money::from_dollars(250_000);
        let smaller = yearly_costs(&h, FilingStatus::MarriedJoint, 2026, 2060);
        let mut r = home();
        r.sell_year = Some(2030);
        r.rent_after = Money::from_dollars(24_000);
        let rent = yearly_costs(&r, FilingStatus::MarriedJoint, 2026, 2060);
        let plain = yearly_costs(
            &{
                let mut p = home();
                p.sell_year = Some(2030);
                p
            },
            FilingStatus::MarriedJoint,
            2026,
            2060,
        );
        assert!(-at(&smaller, 2030) < -at(&plain, 2030));
        assert!(at(&rent, 2035) > at(&plain, 2035));
    }

    #[test]
    fn no_mortgage_and_no_sale_changes_nothing() {
        let mut h = home();
        h.mortgage_balance = Money::ZERO;
        assert!(yearly_costs(&h, FilingStatus::Single, 2026, 2060).is_empty());
    }

    #[test]
    fn a_sale_before_payoff_stops_the_payments_early() {
        let mut h = home();
        h.sell_year = Some(2030);
        let v = yearly_costs(&h, FilingStatus::MarriedJoint, 2026, 2060);
        assert!(at(&v, 2031) < Money::ZERO);
        assert_eq!(at(&v, 2029), Money::ZERO);
    }
}
