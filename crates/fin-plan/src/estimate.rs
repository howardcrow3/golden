//! One-call household tax estimate used by the app's quick calculators.
use crate::tax::{FederalInput, FederalTaxYear, FilingStatus, StateTaxYear};
use fin_core::Money;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxRequest {
    pub wages_cents: i64,
    pub filing: FilingStatus,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxEstimate {
    pub federal_cents: i64,
    pub state_cents: i64,
    pub fica_cents: i64,
    pub total_cents: i64,
    pub effective_rate: f64,
    pub marginal_rate: f64,
    pub state_is_estimate: bool,
}

pub fn estimate(req: &TaxRequest) -> Result<TaxEstimate, String> {
    let wages = Money::from_cents(req.wages_cents);
    let fed = FederalTaxYear::y2026();
    let states = StateTaxYear::y2026();
    let r = fed.income_tax(
        req.filing,
        FederalInput {
            ordinary_income: wages,
            ..Default::default()
        },
    );
    let state = states
        .income_tax(&req.state, req.filing, r.agi)
        .ok_or_else(|| format!("unknown state {}", req.state))?;
    let fica = fed.fica_for_wages(req.filing, wages);
    let total = r.income_tax + state + fica;
    Ok(TaxEstimate {
        federal_cents: r.income_tax.cents(),
        state_cents: state.cents(),
        fica_cents: fica.cents(),
        total_cents: total.cents(),
        effective_rate: if wages.is_zero() {
            0.0
        } else {
            total.cents() as f64 / wages.cents() as f64
        },
        marginal_rate: r.marginal_rate,
        state_is_estimate: states
            .rules(&req.state)
            .map(|s| s.approximate)
            .unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(c: i64, fs: FilingStatus, s: &str) -> TaxRequest {
        TaxRequest {
            wages_cents: c,
            filing: fs,
            state: s.into(),
        }
    }

    #[test]
    fn texas_single_100k() {
        let e = estimate(&req(10_000_000, FilingStatus::Single, "TX")).unwrap();
        assert_eq!(e.state_cents, 0);
        assert_eq!(e.federal_cents, 124_000 + 456_000 + 737_000);
        assert_eq!(e.fica_cents, 765_000);
        assert_eq!(e.total_cents, e.federal_cents + e.fica_cents);
        assert!(!e.state_is_estimate);
    }

    #[test]
    fn unknown_state_is_an_error() {
        assert!(estimate(&req(1, FilingStatus::Single, "ZZ")).is_err());
    }

    #[test]
    fn approximate_state_flagged() {
        assert!(
            estimate(&req(5_000_000, FilingStatus::Single, "KS"))
                .unwrap()
                .state_is_estimate
        );
    }

    #[test]
    fn zero_wages_zero_rate() {
        let e = estimate(&req(0, FilingStatus::Single, "CA")).unwrap();
        assert_eq!((e.total_cents, e.effective_rate), (0, 0.0));
    }

    #[test]
    fn request_deserializes_from_ui_json() {
        let r: TaxRequest = serde_json::from_str(
            r#"{"wagesCents": 9000000, "filing": "married_joint", "state": "NY"}"#,
        )
        .unwrap();
        assert_eq!(r.filing, FilingStatus::MarriedJoint);
        let e = estimate(&r).unwrap();
        let json = serde_json::to_value(&e).unwrap();
        assert!(json["effectiveRate"].as_f64().unwrap() > 0.1);
    }

    #[test]
    fn california_costs_more_than_texas() {
        let ca = estimate(&req(15_000_000, FilingStatus::Single, "CA")).unwrap();
        let tx = estimate(&req(15_000_000, FilingStatus::Single, "TX")).unwrap();
        assert!(ca.total_cents > tx.total_cents);
        assert_eq!(ca.federal_cents, tx.federal_cents);
    }
}
