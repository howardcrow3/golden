//! Typical yearly cost of long-term care by state: CareScout (Genworth) Cost of Care Survey 2025,
//! "Median Cost Data Tables", provider rates gathered July to November 2025. Figures are annual
//! medians in 2025 dollars: nursing home private and semi-private rooms (365 days), assisted
//! living (private one-bedroom), adult day health care (5 days a week) and a non-medical
//! caregiver (44 hours a week). The survey has no row for DC, and a few cells are blank where
//! CareScout did not publish a median; those fall back to the national median.
use fin_core::Money;

const CSV: &str = include_str!("../data/care_costs_2025.csv");

/// Which kind of care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Care {
    NursingHomePrivate,
    NursingHomeSemiPrivate,
    AssistedLiving,
    AdultDay,
    Caregiver,
}

impl Care {
    pub const ALL: [Care; 5] = [
        Care::NursingHomePrivate,
        Care::NursingHomeSemiPrivate,
        Care::AssistedLiving,
        Care::AdultDay,
        Care::Caregiver,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Care::NursingHomePrivate => "nursing_home_private",
            Care::NursingHomeSemiPrivate => "nursing_home_semi_private",
            Care::AssistedLiving => "assisted_living",
            Care::AdultDay => "adult_day",
            Care::Caregiver => "caregiver",
        }
    }

    pub fn from_key(key: &str) -> Option<Care> {
        Care::ALL.into_iter().find(|c| c.key() == key)
    }
}

/// One cost: the state's median, or the national one when the state has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    pub annual: Money,
    pub national_fallback: bool,
}

fn rows() -> impl Iterator<Item = (&'static str, [Option<i64>; 5])> {
    CSV.lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut p = l.split(',');
            let state = p.next().unwrap_or("");
            let mut v = [None; 5];
            for slot in v.iter_mut() {
                *slot = p.next().and_then(|x| x.trim().parse().ok());
            }
            (state, v)
        })
}

fn index(c: Care) -> usize {
    match c {
        Care::Caregiver => 0,
        Care::NursingHomePrivate => 1,
        Care::NursingHomeSemiPrivate => 2,
        Care::AssistedLiving => 3,
        Care::AdultDay => 4,
    }
}

/// The typical yearly cost for a two-letter state code (any case). An unknown code, including
/// DC, gets the national median.
pub fn cost(state: &str, care: Care) -> Cost {
    let want = state.trim().to_ascii_uppercase();
    let i = index(care);
    let national = rows()
        .find(|(s, _)| *s == "US")
        .and_then(|(_, v)| v[i])
        .unwrap_or(0);
    let local = rows()
        .find(|(s, _)| *s == want && want != "US")
        .and_then(|(_, v)| v[i]);
    match local {
        Some(c) => Cost {
            annual: Money::from_dollars(c),
            national_fallback: false,
        },
        None => Cost {
            annual: Money::from_dollars(national),
            national_fallback: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_gets_its_own_median() {
        let c = cost("ny", Care::NursingHomePrivate);
        assert_eq!(c.annual, Money::from_dollars(200_750));
        assert!(!c.national_fallback);
        assert_eq!(
            cost("TX", Care::NursingHomeSemiPrivate).annual,
            Money::from_dollars(67_525)
        );
    }

    #[test]
    fn missing_cells_and_dc_fall_back_to_the_national_median() {
        let ak = cost("AK", Care::NursingHomePrivate);
        assert!(ak.national_fallback);
        assert_eq!(ak.annual, Money::from_dollars(129_575));
        let dc = cost("DC", Care::AssistedLiving);
        assert!(dc.national_fallback);
        assert_eq!(dc.annual, Money::from_dollars(74_400));
        assert!(cost("DE", Care::AdultDay).national_fallback);
        assert!(cost("US", Care::Caregiver).national_fallback);
    }

    #[test]
    fn every_state_is_present_with_sane_figures() {
        let all: Vec<_> = rows().collect();
        assert_eq!(all.len(), 51);
        for (s, v) in &all {
            for x in v.iter().flatten() {
                assert!((10_000..=400_000).contains(x), "{s}: {x}");
            }
            // A private room costs at least a semi-private one.
            if let (Some(p), Some(sp)) = (v[1], v[2]) {
                assert!(p >= sp, "{s}: private {p} under semi {sp}");
            }
        }
    }

    #[test]
    fn keys_round_trip() {
        for c in Care::ALL {
            assert_eq!(Care::from_key(c.key()), Some(c));
        }
        assert_eq!(Care::from_key("nope"), None);
    }
}
