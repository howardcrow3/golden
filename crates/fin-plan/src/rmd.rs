//! Required minimum distributions from pre-tax retirement accounts.
//!
//! Source: IRS Publication 590-B, Uniform Lifetime Table (effective 2022) and the SECURE 2.0
//! starting ages (73 for people born 1951 to 1959, 75 for 1960 and later). Born before 1951 the
//! start age was 72 or earlier, which is already past for any plan starting now.

use fin_core::Money;

/// First age at which distributions are required, by birth year.
pub fn start_age(birth_year: i32) -> i32 {
    if birth_year >= 1960 {
        75
    } else if birth_year >= 1951 {
        73
    } else {
        72
    }
}

/// Uniform Lifetime Table distribution period for the age reached this year.
pub fn divisor(age: i32) -> Option<f64> {
    const FROM_72: [f64; 49] = [
        27.4, 26.5, 25.5, 24.6, 23.7, 22.9, 22.0, 21.1, 20.2, 19.4, 18.5, 17.7, 16.8, 16.0, 15.2,
        14.4, 13.7, 12.9, 12.2, 11.5, 10.8, 10.1, 9.5, 8.9, 8.4, 7.8, 7.3, 6.8, 6.4, // 72-100
        6.0, 5.6, 5.2, 4.9, 4.6, 4.3, 4.1, 3.9, 3.7, 3.5, 3.4, 3.3, 3.1, 3.0, 2.9, 2.8, 2.7, 2.5,
        2.3, 2.0, // 101-120
    ];
    if age < 72 {
        None
    } else {
        // 120 and over all use 2.0.
        Some(FROM_72[((age - 72) as usize).min(FROM_72.len() - 1)])
    }
}

/// The distribution due for a year: the balance at the end of the year before, divided by the
/// divisor. Zero before the starting age.
pub fn required(birth_year: i32, age: i32, prior_year_end_balance: Money) -> Money {
    if age < start_age(birth_year) {
        return Money::ZERO;
    }
    match divisor(age) {
        Some(d) if d > 0.0 => prior_year_end_balance.max(Money::ZERO).mul_rate(1.0 / d),
        _ => Money::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_ages_follow_birth_year() {
        assert_eq!(start_age(1948), 72);
        assert_eq!(start_age(1955), 73);
        assert_eq!(start_age(1959), 73);
        assert_eq!(start_age(1960), 75);
        assert_eq!(start_age(1985), 75);
    }

    #[test]
    fn the_table_matches_published_values() {
        assert_eq!(divisor(73), Some(26.5));
        assert_eq!(divisor(75), Some(24.6));
        assert_eq!(divisor(80), Some(20.2));
        assert_eq!(divisor(90), Some(12.2));
        assert_eq!(divisor(95), Some(8.9));
        assert_eq!(divisor(100), Some(6.4));
        assert_eq!(divisor(60), None);
        // Past 100 it keeps falling and never gets below 2.0.
        assert!(divisor(105).unwrap() < divisor(100).unwrap());
        assert_eq!(divisor(125), Some(2.0));
    }

    #[test]
    fn the_table_past_100_matches_publication_590b() {
        for (age, d) in [
            (101, 6.0),
            (103, 5.2),
            (104, 4.9),
            (105, 4.6),
            (108, 3.9),
            (110, 3.5),
            (111, 3.4),
            (115, 2.9),
            (118, 2.5),
            (119, 2.3),
            (120, 2.0),
        ] {
            assert_eq!(divisor(age), Some(d), "age {age}");
        }
    }

    #[test]
    fn nothing_is_due_before_the_start_age() {
        assert_eq!(
            required(1960, 74, Money::from_dollars(1_000_000)),
            Money::ZERO
        );
        // $1,000,000 at 75 is $40,650.40 (1,000,000 / 24.6).
        let r = required(1960, 75, Money::from_dollars(1_000_000));
        assert!((r.cents() - 4_065_041).abs() <= 1, "{r:?}");
        assert_eq!(required(1950, 75, Money::ZERO), Money::ZERO);
    }

    #[test]
    fn a_negative_balance_owes_nothing() {
        assert_eq!(required(1950, 80, Money::from_dollars(-5)), Money::ZERO);
    }
}

/// 2026 limit on qualified charitable distributions per person (inflation indexed).
const QCD_LIMIT: i64 = 111_000;

/// The part of this year's giving that can go straight from an IRA: only from age 70 and a half
/// (we use 71 for a whole-year test), never more than the IRA gives up this year (the required
/// distribution), and within the yearly limit. Source: IRC section 408(d)(8).
pub fn qcd(age: i32, giving: Money, rmd: Money) -> Money {
    if age < 71 {
        return Money::ZERO;
    }
    giving
        .max(Money::ZERO)
        .min(rmd.max(Money::ZERO))
        .min(Money::from_dollars(QCD_LIMIT))
}

#[cfg(test)]
mod qcd_tests {
    use super::*;

    #[test]
    fn giving_comes_out_of_the_required_distribution() {
        let g = Money::from_dollars(10_000);
        assert_eq!(qcd(75, g, Money::from_dollars(40_000)), g);
        assert_eq!(
            qcd(75, g, Money::from_dollars(4_000)),
            Money::from_dollars(4_000)
        );
    }

    #[test]
    fn nothing_before_the_age_or_without_a_distribution() {
        let g = Money::from_dollars(10_000);
        assert_eq!(qcd(65, g, Money::from_dollars(40_000)), Money::ZERO);
        assert_eq!(qcd(80, g, Money::ZERO), Money::ZERO);
        assert_eq!(
            qcd(80, Money::from_dollars(-5), Money::from_dollars(9)),
            Money::ZERO
        );
    }

    #[test]
    fn the_yearly_limit_applies() {
        assert_eq!(
            qcd(
                80,
                Money::from_dollars(500_000),
                Money::from_dollars(900_000)
            ),
            Money::from_dollars(111_000)
        );
    }
}
