//! Replay the plan through every real stretch of US market history since 1928.
//!
//! Returns are Damodaran's annual S&P 500 (with dividends) and 10-year Treasury bond returns,
//! turned into real returns with CPI. A 60/40 mix is the default; the allocation can change.
use crate::params::{Assumptions, Picks};
use crate::projection::{project, PlanInput};
use crate::scenario;

const DATA: &str = include_str!("../data/history_1928_2025.csv");

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HistoryYear {
    pub year: i32,
    pub stocks: f64,
    pub bills: f64,
    pub bonds: f64,
    pub inflation: f64,
}

/// Every year of bundled history, oldest first.
pub fn history() -> Vec<HistoryYear> {
    DATA.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("year"))
        .filter_map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            Some(HistoryYear {
                year: f.first()?.parse().ok()?,
                stocks: f.get(1)?.parse().ok()?,
                bills: f.get(2)?.parse().ok()?,
                bonds: f.get(3)?.parse().ok()?,
                inflation: f.get(4)?.parse().ok()?,
            })
        })
        .collect()
}

/// Real return of a stock and bond mix for one year.
pub fn real_return(y: &HistoryYear, stock_share: f64) -> f64 {
    let w = stock_share.clamp(0.0, 1.0);
    let nominal = w * y.stocks + (1.0 - w) * y.bonds;
    (1.0 + nominal) / (1.0 + y.inflation) - 1.0
}

#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub start_year: i32,
    pub succeeded: bool,
    pub final_balance_cents: i64,
    pub depleted_year: Option<i32>,
    /// Compound real return of the mix over the window.
    pub annualised_return: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Backtest {
    pub stock_share: f64,
    pub years: usize,
    pub windows: Vec<Window>,
    pub success_rate: f64,
    pub first_year: i32,
    pub last_year: i32,
}

/// Run the household plan from each possible starting year in history. The plan's own
/// real-return assumption is replaced by what actually happened.
pub fn run(base: &PlanInput, picks: &Picks, stock_share: f64) -> Result<Backtest, String> {
    let hist = history();
    let (plan, _) =
        scenario::resolve(base, &Assumptions::defaults(), picks).map_err(|e| e.to_string())?;
    // Ages in the projection are the age reached by December 31, so count years the same way.
    let primary_age = plan.people[0].age_on(
        fin_core::NaiveDate::from_ymd_opt(plan.start_year, 12, 31).ok_or("bad start year")?,
    );
    let n = (plan.horizon_age - primary_age).max(0) as usize + 1;
    if n > hist.len() {
        return Err(format!(
            "The plan runs {n} years but history only covers {}.",
            hist.len()
        ));
    }
    let mut windows = Vec::new();
    for start in 0..=(hist.len() - n) {
        let slice = &hist[start..start + n];
        let returns: Vec<f64> = slice.iter().map(|y| real_return(y, stock_share)).collect();
        let p = project(&plan, &|i| returns[i.min(n - 1)]);
        let growth: f64 = returns.iter().map(|r| 1.0 + r).product();
        windows.push(Window {
            start_year: slice[0].year,
            succeeded: p.succeeded(),
            final_balance_cents: p.final_balance().cents(),
            depleted_year: p.depleted_year,
            annualised_return: growth.powf(1.0 / n as f64) - 1.0,
        });
    }
    let ok = windows.iter().filter(|w| w.succeeded).count();
    Ok(Backtest {
        stock_share,
        years: n,
        success_rate: ok as f64 / windows.len() as f64,
        first_year: hist[0].year,
        last_year: hist[hist.len() - 1].year,
        windows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::fixtures::couple;

    #[test]
    fn history_covers_1928_to_2025_with_known_anchor_years() {
        let h = history();
        assert_eq!(h.len(), 98);
        assert_eq!((h[0].year, h[h.len() - 1].year), (1928, 2025));
        let y2008 = h.iter().find(|y| y.year == 2008).unwrap();
        assert!(
            y2008.stocks < -0.35 && y2008.stocks > -0.40,
            "{}",
            y2008.stocks
        );
        let y1931 = h.iter().find(|y| y.year == 1931).unwrap();
        assert!(y1931.stocks < -0.40);
        assert!(h.iter().all(|y| y.inflation > -0.15 && y.inflation < 0.20));
    }

    #[test]
    fn real_return_removes_inflation_and_blends() {
        let y = HistoryYear {
            year: 1,
            stocks: 0.10,
            bills: 0.0,
            bonds: 0.04,
            inflation: 0.02,
        };
        assert!((real_return(&y, 1.0) - (1.10 / 1.02 - 1.0)).abs() < 1e-12);
        assert!((real_return(&y, 0.0) - (1.04 / 1.02 - 1.0)).abs() < 1e-12);
        assert!((real_return(&y, 0.5) - (1.07 / 1.02 - 1.0)).abs() < 1e-12);
        // Out-of-range shares are clamped.
        assert_eq!(real_return(&y, 3.0), real_return(&y, 1.0));
    }

    #[test]
    fn replays_every_start_year_that_fits() {
        let b = run(&couple(), &Picks::new(), 0.6).unwrap();
        // The fixture's projection has a row for each age from 50 to 95 (ages at year end):
        // 46 years, the same span the replay must cover, so 98 - 46 + 1 windows.
        let rows = crate::projection::project_constant(&couple(), 0.04)
            .rows
            .len();
        assert_eq!(rows, 46);
        assert_eq!(b.years, rows);
        assert_eq!(b.windows.len(), 98 - 46 + 1);
        assert_eq!(b.windows[0].start_year, 1928);
        assert_eq!(b.windows.last().unwrap().start_year, 2025 - 45);
        assert!((0.0..=1.0).contains(&b.success_rate));
    }

    #[test]
    fn same_inputs_give_the_same_answer() {
        let a = run(&couple(), &Picks::new(), 0.6).unwrap();
        let b = run(&couple(), &Picks::new(), 0.6).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn more_spending_never_helps() {
        let lean = couple();
        let mut rich = couple();
        rich.annual_spending = fin_core::Money::from_dollars(200_000);
        let a = run(&lean, &Picks::new(), 0.6).unwrap();
        let b = run(&rich, &Picks::new(), 0.6).unwrap();
        assert!(b.success_rate <= a.success_rate);
    }

    #[test]
    fn a_plan_longer_than_history_is_an_error() {
        let mut p = couple();
        p.people[0].birth_date = fin_core::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let mut picks = Picks::new();
        picks.insert("horizon_age".into(), "Age 100".into());
        assert!(run(&p, &picks, 0.6).is_err());
    }
}
