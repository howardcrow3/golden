//! Rent or buy: after `years`, which leaves you with more? Buying puts the down payment into a
//! home and pays a mortgage, property tax and upkeep; renting invests the down payment and
//! any monthly difference. Everything is in nominal dollars with the growth rates you give, and
//! tax effects (mortgage interest deduction, capital gains exclusion) are left out.
use crate::housing::{balance_after, monthly_payment};
use fin_core::Money;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct Inputs {
    pub price: Money,
    pub down_payment: Money,
    pub mortgage_rate: f64,
    pub mortgage_years: i32,
    /// Property tax and insurance as a share of the price each year.
    pub carrying_rate: f64,
    /// Maintenance as a share of the price each year.
    pub upkeep_rate: f64,
    pub appreciation: f64,
    pub selling_cost: f64,
    /// Monthly rent today for a similar place.
    pub monthly_rent: Money,
    pub rent_growth: f64,
    /// What invested money earns a year.
    pub invest_return: f64,
    pub years: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub years: i32,
    /// Home value less the mortgage still owed and selling costs.
    pub buy_net_worth_cents: i64,
    /// The invested down payment plus invested monthly savings.
    pub rent_net_worth_cents: i64,
    /// Buying minus renting; positive favours buying.
    pub buy_advantage_cents: i64,
    pub monthly_payment_cents: i64,
    /// The first year in which buying comes out ahead, if it does within the horizon.
    pub break_even_year: Option<i32>,
}

fn worth(i: &Inputs, years: i32) -> (f64, f64) {
    let price = i.price.cents() as f64;
    let down = i.down_payment.cents() as f64;
    let loan = Money::from_cents((price - down).max(0.0).round() as i64);
    let pay = monthly_payment(loan, i.mortgage_rate, i.mortgage_years).cents() as f64;
    let r_m = (1.0 + i.invest_return).powf(1.0 / 12.0) - 1.0;
    let mut renter = down;
    let mut buyer_savings = 0.0;
    for m in 0..(years * 12) {
        let year = (m / 12) as f64;
        let rent = i.monthly_rent.cents() as f64 * (1.0 + i.rent_growth).powf(year);
        let owning = if m / 12 < i.mortgage_years { pay } else { 0.0 }
            + price * (1.0 + i.appreciation).powf(year) * (i.carrying_rate + i.upkeep_rate) / 12.0;
        // Whoever spends less puts the difference into investments.
        let diff = owning - rent;
        renter = renter * (1.0 + r_m) + diff.max(0.0);
        buyer_savings = buyer_savings * (1.0 + r_m) + (-diff).max(0.0);
    }
    let value = price * (1.0 + i.appreciation).powf(years as f64);
    let owed = balance_after(loan, i.mortgage_rate, i.mortgage_years, years).cents() as f64;
    let buy = value * (1.0 - i.selling_cost) - owed + buyer_savings;
    (buy, renter)
}

pub fn compare(i: &Inputs) -> Result<Outcome, String> {
    if i.price <= Money::ZERO || i.monthly_rent <= Money::ZERO {
        return Err("Enter a price and a rent above zero.".into());
    }
    if i.down_payment < Money::ZERO || i.down_payment > i.price {
        return Err("The down payment must be between zero and the price.".into());
    }
    if !(1..=50).contains(&i.years) || !(1..=40).contains(&i.mortgage_years) {
        return Err("Use 1 to 50 years to compare and a 1 to 40 year mortgage.".into());
    }
    let (buy, rent) = worth(i, i.years);
    let loan = Money::from_cents((i.price.cents() - i.down_payment.cents()).max(0));
    let break_even = (1..=i.years).find(|&y| {
        let (b, r) = worth(i, y);
        b > r
    });
    Ok(Outcome {
        years: i.years,
        buy_net_worth_cents: buy.round() as i64,
        rent_net_worth_cents: rent.round() as i64,
        buy_advantage_cents: (buy - rent).round() as i64,
        monthly_payment_cents: monthly_payment(loan, i.mortgage_rate, i.mortgage_years).cents(),
        break_even_year: break_even,
    })
}

/// The same inputs as the screen sends them: cents and basis points (hundredths of a percent).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub price_cents: i64,
    pub down_payment_cents: i64,
    pub mortgage_rate_bp: i64,
    pub mortgage_years: i32,
    pub carrying_bp: i64,
    pub upkeep_bp: i64,
    pub appreciation_bp: i64,
    pub selling_cost_bp: i64,
    pub monthly_rent_cents: i64,
    pub rent_growth_bp: i64,
    pub invest_return_bp: i64,
    pub years: i32,
}

pub fn run(r: &Request) -> Result<Outcome, String> {
    let bp = |v: i64| v as f64 / 10_000.0;
    for v in [
        r.mortgage_rate_bp,
        r.carrying_bp,
        r.upkeep_bp,
        r.appreciation_bp,
        r.selling_cost_bp,
        r.rent_growth_bp,
        r.invest_return_bp,
    ] {
        if !(-1_000..=3_000).contains(&v) {
            return Err("Rates should be between -10% and 30%.".into());
        }
    }
    compare(&Inputs {
        price: Money::from_cents(r.price_cents),
        down_payment: Money::from_cents(r.down_payment_cents),
        mortgage_rate: bp(r.mortgage_rate_bp),
        mortgage_years: r.mortgage_years,
        carrying_rate: bp(r.carrying_bp),
        upkeep_rate: bp(r.upkeep_bp),
        appreciation: bp(r.appreciation_bp),
        selling_cost: bp(r.selling_cost_bp),
        monthly_rent: Money::from_cents(r.monthly_rent_cents),
        rent_growth: bp(r.rent_growth_bp),
        invest_return: bp(r.invest_return_bp),
        years: r.years,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Inputs {
        Inputs {
            price: Money::from_dollars(500_000),
            down_payment: Money::from_dollars(100_000),
            mortgage_rate: 0.065,
            mortgage_years: 30,
            carrying_rate: 0.015,
            upkeep_rate: 0.01,
            appreciation: 0.035,
            selling_cost: 0.06,
            monthly_rent: Money::from_dollars(2_500),
            rent_growth: 0.03,
            invest_return: 0.06,
            years: 20,
        }
    }

    #[test]
    fn low_rent_favours_renting_and_high_rent_favours_buying() {
        let mut cheap = base();
        cheap.monthly_rent = Money::from_dollars(1_500);
        assert!(compare(&cheap).unwrap().buy_advantage_cents < 0);
        let mut dear = base();
        dear.monthly_rent = Money::from_dollars(5_000);
        let d = compare(&dear).unwrap();
        assert!(d.buy_advantage_cents > 0);
        assert!(d.break_even_year.is_some());
    }

    #[test]
    fn a_stronger_housing_market_helps_the_buyer() {
        let slow = compare(&base()).unwrap().buy_advantage_cents;
        let mut hot = base();
        hot.appreciation = 0.06;
        assert!(compare(&hot).unwrap().buy_advantage_cents > slow);
    }

    #[test]
    fn buying_with_no_growth_and_selling_costs_loses_in_a_short_horizon() {
        let mut i = base();
        i.appreciation = 0.0;
        i.years = 2;
        let o = compare(&i).unwrap();
        assert!(o.buy_advantage_cents < 0);
        assert_eq!(o.break_even_year, None);
    }

    #[test]
    fn the_request_form_matches_the_typed_inputs() {
        let r = Request {
            price_cents: 50_000_000,
            down_payment_cents: 10_000_000,
            mortgage_rate_bp: 650,
            mortgage_years: 30,
            carrying_bp: 150,
            upkeep_bp: 100,
            appreciation_bp: 350,
            selling_cost_bp: 600,
            monthly_rent_cents: 250_000,
            rent_growth_bp: 300,
            invest_return_bp: 600,
            years: 20,
        };
        assert_eq!(run(&r).unwrap(), compare(&base()).unwrap());
        let mut bad = r.clone();
        bad.invest_return_bp = 9_000;
        assert!(run(&bad).unwrap_err().contains("Rates"));
    }

    #[test]
    fn payment_matches_the_loan_formula_and_bad_input_is_rejected() {
        let o = compare(&base()).unwrap();
        // $400,000 at 6.5% for 30 years is about $2,528 a month.
        assert!(
            (o.monthly_payment_cents - 252_800).abs() < 300,
            "{}",
            o.monthly_payment_cents
        );
        let mut bad = base();
        bad.down_payment = Money::from_dollars(600_000);
        assert!(compare(&bad).is_err());
        bad = base();
        bad.years = 0;
        assert!(compare(&bad).is_err());
        bad = base();
        bad.monthly_rent = Money::ZERO;
        assert!(compare(&bad).is_err());
    }
}
