use fin_core::{Money, NaiveDate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leg {
    pub id: i64,
    pub account: i64,
    pub day: NaiveDate,
    pub amount: Money,
}

/// Pair up money moved between the household's own accounts: an outflow in one account and an
/// equal inflow in another within `window_days`. Each leg is used at most once. Returns
/// (out_id, in_id) pairs so both sides can be excluded from spending and income.
pub fn match_transfers(legs: &[Leg], window_days: i64) -> Vec<(i64, i64)> {
    let mut used = vec![false; legs.len()];
    let mut pairs = vec![];
    for (i, a) in legs.iter().enumerate() {
        if used[i] || a.amount >= Money::ZERO {
            continue;
        }
        let best = legs
            .iter()
            .enumerate()
            .filter(|(j, b)| {
                !used[*j]
                    && b.account != a.account
                    && b.amount == -a.amount
                    && (b.day - a.day).num_days().abs() <= window_days
            })
            .min_by_key(|(_, b)| (b.day - a.day).num_days().abs());
        if let Some((j, b)) = best {
            used[i] = true;
            used[j] = true;
            pairs.push((a.id, b.id));
        }
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }
    fn leg(id: i64, account: i64, day: u32, dollars: i64) -> Leg {
        Leg {
            id,
            account,
            day: d(day),
            amount: Money::from_dollars(dollars),
        }
    }

    #[test]
    fn pairs_equal_and_opposite_across_accounts() {
        let legs = [leg(1, 1, 5, -500), leg(2, 2, 6, 500), leg(3, 1, 9, -20)];
        assert_eq!(match_transfers(&legs, 3), vec![(1, 2)]);
    }

    #[test]
    fn same_account_is_not_a_transfer() {
        assert!(match_transfers(&[leg(1, 1, 5, -50), leg(2, 1, 5, 50)], 3).is_empty());
    }

    #[test]
    fn outside_window_is_not_a_transfer() {
        assert!(match_transfers(&[leg(1, 1, 1, -50), leg(2, 2, 10, 50)], 3).is_empty());
    }

    #[test]
    fn each_leg_is_used_once_and_nearest_wins() {
        let legs = [leg(1, 1, 5, -100), leg(2, 2, 8, 100), leg(3, 2, 6, 100)];
        assert_eq!(match_transfers(&legs, 5), vec![(1, 3)]);
    }

    #[test]
    fn two_identical_transfers_pair_one_to_one() {
        let legs = [
            leg(1, 1, 5, -100),
            leg(2, 1, 5, -100),
            leg(3, 2, 5, 100),
            leg(4, 2, 5, 100),
        ];
        let p = match_transfers(&legs, 1);
        assert_eq!(p.len(), 2);
        let ids: std::collections::HashSet<i64> = p.iter().flat_map(|(a, b)| [*a, *b]).collect();
        assert_eq!(ids.len(), 4);
    }
}
