//! Integration: properties a whole plan keeps in every state, built from the sample household.
use fin_app::household::{Household, StreamRow};
use fin_core::{Money, NaiveDate};
use fin_plan::projection::project;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 28).unwrap()
}

fn with_pensions(mut h: Household) -> Household {
    for (label, kind, cents) in [
        ("Pension", "pension", 2_000_000),
        ("State pension", "public_pension", 1_500_000),
        ("Military retired pay", "military_pension", 1_800_000),
    ] {
        h.streams.push(StreamRow {
            label: label.into(),
            person: 0,
            annual_cents: cents,
            self_employed: false,
            start_year: None,
            end_year: None,
            contribution_cents: 0,
            kind: Some(kind.into()),
            employer_match_cents: 0,
            hsa_cents: 0,
        });
    }
    h
}

fn states() -> Vec<String> {
    fin_plan::tax::StateTaxYear::y2026()
        .codes()
        .map(str::to_string)
        .collect()
}

#[test]
fn every_state_gives_a_sane_plan() {
    for code in states() {
        let mut h = with_pensions(Household::sample());
        h.state = code.clone();
        let input = h.plan_input(today()).unwrap();
        let p = project(&input, &|_| 0.04);
        assert!(!p.rows.is_empty(), "{code}");
        for r in &p.rows {
            assert!(
                r.taxes >= Money::ZERO,
                "{code} {}: taxes {}",
                r.year,
                r.taxes
            );
            assert!(r.income_tax >= Money::ZERO, "{code} {}", r.year);
            assert!(
                r.income_tax <= r.taxes,
                "{code} {}: income tax above all taxes",
                r.year
            );
            assert!(r.end_balance >= Money::ZERO, "{code} {}", r.year);
        }
    }
}

#[test]
fn spending_more_never_leaves_more_at_the_end() {
    for code in ["CA", "TX", "NY", "MO", "ID", "VT"] {
        let mut h = with_pensions(Household::sample());
        h.state = code.into();
        let end = |h: &Household| {
            let p = project(&h.plan_input(today()).unwrap(), &|_| 0.04);
            p.rows.last().unwrap().end_balance
        };
        let base = end(&h);
        h.spending_cents += 1_000_000;
        assert!(end(&h) <= base, "{code}");
    }
}

#[test]
fn a_no_tax_state_never_costs_more_tax_than_any_other() {
    let total = |code: &str| -> i64 {
        let mut h = with_pensions(Household::sample());
        h.state = code.into();
        project(&h.plan_input(today()).unwrap(), &|_| 0.04)
            .rows
            .iter()
            .map(|r| r.income_tax.cents())
            .sum()
    };
    let texas = total("TX");
    for code in states() {
        assert!(total(&code) >= texas, "{code} taxes less than Texas");
    }
}
