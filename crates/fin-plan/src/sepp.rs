//! Substantially equal periodic payments (IRC 72(t)): a way to take money from an IRA before
//! age 59 and a half without the 10% early-withdrawal penalty. Payments must continue for five
//! years or until 59 and a half, whichever is longer; changing them sooner brings back the
//! penalty on everything taken so far, with interest.
use fin_core::Money;
use serde::Serialize;

/// The interest rate ceiling the IRS allows is the greater of 5% and 120% of the federal
/// mid-term rate. 5% is used here; a higher published rate can be entered instead.
pub const DEFAULT_RATE: f64 = 0.05;

/// Single Life Expectancy table (Treasury Regulation 1.401(a)(9)-9, effective 2022), ages 40-70.
const SINGLE_LIFE: [f64; 31] = [
    45.7, 44.8, 43.8, 42.9, 41.9, 41.0, 40.0, 39.0, 38.1, 37.1, 36.2, 35.3, 34.3, 33.4, 32.5, 31.6,
    30.6, 29.8, 28.9, 28.0, 27.1, 26.2, 25.4, 24.5, 23.7, 22.9, 22.0, 21.2, 20.4, 19.6, 18.8,
];

pub fn life_expectancy(age: i32) -> Option<f64> {
    if (40..=70).contains(&age) {
        Some(SINGLE_LIFE[(age - 40) as usize])
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// Balance divided by life expectancy, recalculated every year: smallest, and it flexes.
    RequiredMinimum,
    /// A level payment that would pay the balance off over life expectancy at the chosen rate.
    FixedAmortization,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sepp {
    pub method: Method,
    pub yearly_cents: i64,
    /// Whether the payment is the same every year.
    pub level: bool,
    /// First age at which the payments may stop: the later of 59 and a half and five years on.
    pub earliest_stop_age: i32,
}

/// Both methods for an IRA of `balance` at `age`. Not available under 40 or over 70 (outside
/// the table here) or when the rate is not positive.
pub fn calculate(balance: Money, age: i32, rate: f64) -> Result<Vec<Sepp>, String> {
    let n = life_expectancy(age).ok_or("This calculator covers ages 40 to 70.")?;
    if balance <= Money::ZERO {
        return Err("Enter the IRA balance you would draw on.".into());
    }
    if !(rate > 0.0 && rate <= 0.2) {
        return Err("The interest rate must be between 0% and 20%.".into());
    }
    let b = balance.cents() as f64;
    let amort = b * rate / (1.0 - (1.0 + rate).powf(-n));
    let stop = (age + 5).max(60);
    Ok(vec![
        Sepp {
            method: Method::RequiredMinimum,
            yearly_cents: (b / n).round() as i64,
            level: false,
            earliest_stop_age: stop,
        },
        Sepp {
            method: Method::FixedAmortization,
            yearly_cents: amort.round() as i64,
            level: true,
            earliest_stop_age: stop,
        },
    ])
}

/// The same in cents and basis points, for the app's commands.
pub fn calculate_cents(balance_cents: i64, age: i32, rate_bp: i64) -> Result<Vec<Sepp>, String> {
    calculate(
        Money::from_cents(balance_cents),
        age,
        rate_bp as f64 / 10_000.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_required_minimum_method_divides_by_life_expectancy() {
        let r = calculate(Money::from_dollars(500_000), 50, 0.05).unwrap();
        assert_eq!(r[0].method, Method::RequiredMinimum);
        // 500,000 / 36.2
        assert_eq!(r[0].yearly_cents, (50_000_000.0f64 / 36.2).round() as i64);
        assert!(!r[0].level);
    }

    #[test]
    fn amortization_pays_more_and_matches_the_loan_formula() {
        let r = calculate(Money::from_dollars(500_000), 50, 0.05).unwrap();
        let rmd = r[0].yearly_cents;
        let am = r[1].yearly_cents;
        assert!(am > rmd);
        // Paying `am` for 36.2 years at 5% exactly retires the balance.
        let mut bal = 50_000_000.0f64;
        for _ in 0..36 {
            bal = bal * 1.05 - am as f64;
        }
        let tail = bal * 1.05f64.powf(0.2) - am as f64 * 0.2;
        assert!(tail.abs() < 50_000_000.0 * 0.01, "tail {tail}");
        assert!(r[1].level);
    }

    #[test]
    fn a_higher_rate_allows_a_bigger_payment() {
        let lo = calculate(Money::from_dollars(400_000), 55, 0.03).unwrap()[1].yearly_cents;
        let hi = calculate(Money::from_dollars(400_000), 55, 0.05).unwrap()[1].yearly_cents;
        assert!(hi > lo);
    }

    #[test]
    fn payments_run_to_59_and_a_half_or_five_years() {
        assert_eq!(
            calculate(Money::from_dollars(1), 45, 0.05).unwrap()[0].earliest_stop_age,
            60
        );
        assert_eq!(
            calculate(Money::from_dollars(1), 58, 0.05).unwrap()[0].earliest_stop_age,
            63
        );
    }

    #[test]
    fn out_of_range_inputs_are_readable_errors() {
        assert!(calculate(Money::from_dollars(1000), 39, 0.05).is_err());
        assert!(calculate(Money::from_dollars(1000), 71, 0.05).is_err());
        assert!(calculate(Money::ZERO, 50, 0.05).is_err());
        assert!(calculate(Money::from_dollars(1000), 50, 0.0).is_err());
        assert!(calculate(Money::from_dollars(1000), 50, 0.5).is_err());
    }

    #[test]
    fn cents_wrapper_matches() {
        assert_eq!(
            calculate_cents(50_000_000, 50, 500).unwrap(),
            calculate(Money::from_dollars(500_000), 50, 0.05).unwrap()
        );
    }

    #[test]
    fn table_ends_match_the_published_table() {
        assert_eq!(life_expectancy(40), Some(45.7));
        assert_eq!(life_expectancy(59), Some(28.0));
        assert_eq!(life_expectancy(70), Some(18.8));
    }
}
