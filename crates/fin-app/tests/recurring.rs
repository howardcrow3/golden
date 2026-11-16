use fin_app::App;

fn csv(lines: &[(&str, &str, &str)]) -> String {
    let mut s = String::from("Date,Description,Amount\n");
    for (d, p, a) in lines {
        s += &format!("{d},{p},{a}\n");
    }
    s
}

fn app(lines: &[(&str, &str, &str)]) -> App {
    let mut app = App::in_memory().unwrap();
    app.import_statement(csv(lines).as_bytes(), "Checking", "checking")
        .unwrap();
    app
}

const ALL: (&str, &str) = ("2026-01-01", "2026-12-31");

fn monthly(payee: &str, amount: &str, months: &[u32], day: u32) -> Vec<(String, String, String)> {
    months
        .iter()
        .map(|m| {
            (
                format!("{m:02}/{day:02}/2026"),
                payee.to_string(),
                amount.to_string(),
            )
        })
        .collect()
}

fn build(parts: Vec<Vec<(String, String, String)>>) -> App {
    let flat: Vec<(String, String, String)> = parts.into_iter().flatten().collect();
    let refs: Vec<(&str, &str, &str)> = flat
        .iter()
        .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
        .collect();
    app(&refs)
}

#[test]
fn finds_a_monthly_subscription_and_its_yearly_cost() {
    let a = build(vec![monthly(
        "NETFLIX.COM 8005551212",
        "-15.49",
        &[1, 2, 3, 4, 5, 6],
        14,
    )]);
    let r = a.recurring(ALL.0, ALL.1).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].cadence, "monthly");
    assert_eq!(r[0].typical_cents, -1549);
    assert_eq!(r[0].times_seen, 6);
    assert_eq!(r[0].yearly_cents, -1549 * 12);
    assert_eq!(r[0].last_seen, "2026-06-14");
    assert_eq!(r[0].next_expected, "2026-07-14");
}

#[test]
fn paychecks_are_recurring_income() {
    let a = build(vec![
        monthly("ACME PAYROLL", "5000.00", &[1, 2, 3, 4], 1),
        monthly("ACME PAYROLL", "5000.00", &[1, 2, 3, 4], 15),
    ]);
    // Two paydays a month look like a 15-day rhythm: every two weeks is the nearest.
    let r = a.recurring(ALL.0, ALL.1).unwrap();
    assert_eq!(r.len(), 1);
    assert!(r[0].typical_cents == 500_000 && r[0].yearly_cents > 0);
}

#[test]
fn store_numbers_do_not_split_a_payee() {
    let a = build(vec![
        monthly("SPOTIFY USA 1234", "-10.99", &[1, 2], 5),
        monthly("SPOTIFY USA 9876", "-10.99", &[3, 4], 5),
    ]);
    let r = a.recurring(ALL.0, ALL.1).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].times_seen, 4);
}

#[test]
fn irregular_shopping_is_not_recurring() {
    let a = build(vec![vec![
        ("01/03/2026".into(), "TRADER JOES".into(), "-60.00".into()),
        ("01/05/2026".into(), "TRADER JOES".into(), "-45.00".into()),
        ("01/19/2026".into(), "TRADER JOES".into(), "-80.00".into()),
        ("02/22/2026".into(), "TRADER JOES".into(), "-52.00".into()),
    ]]);
    assert!(a.recurring(ALL.0, ALL.1).unwrap().is_empty());
}

#[test]
fn two_sightings_are_not_enough() {
    let a = build(vec![monthly("GYM", "-30.00", &[1, 2], 3)]);
    assert!(a.recurring(ALL.0, ALL.1).unwrap().is_empty());
}

#[test]
fn a_steady_rhythm_with_wildly_different_amounts_is_not_recurring() {
    let a = build(vec![vec![
        ("01/10/2026".into(), "MYSTERY CO".into(), "-10.00".into()),
        ("02/10/2026".into(), "MYSTERY CO".into(), "-500.00".into()),
        ("03/10/2026".into(), "MYSTERY CO".into(), "-3.00".into()),
        ("04/10/2026".into(), "MYSTERY CO".into(), "-900.00".into()),
    ]]);
    assert!(a.recurring(ALL.0, ALL.1).unwrap().is_empty());
}

#[test]
fn drifting_utility_bills_still_count_and_one_skipped_month_is_forgiven() {
    let a = build(vec![vec![
        ("01/12/2026".into(), "CITY POWER".into(), "-110.00".into()),
        ("02/12/2026".into(), "CITY POWER".into(), "-125.00".into()),
        ("03/12/2026".into(), "CITY POWER".into(), "-118.00".into()),
        ("05/12/2026".into(), "CITY POWER".into(), "-121.00".into()),
        ("06/12/2026".into(), "CITY POWER".into(), "-115.00".into()),
    ]]);
    let r = a.recurring(ALL.0, ALL.1).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].cadence, "monthly");
}

#[test]
fn biggest_yearly_amount_first_and_empty_ledger_is_empty() {
    let a = build(vec![
        monthly("GYM", "-30.00", &[1, 2, 3, 4], 3),
        monthly("RENT PAYMENT LANDLORD", "-1500.00", &[1, 2, 3, 4], 1),
    ]);
    let r = a.recurring(ALL.0, ALL.1).unwrap();
    assert_eq!(r.len(), 2);
    assert!(r[0].payee.contains("RENT"));
    assert!(App::in_memory()
        .unwrap()
        .recurring(ALL.0, ALL.1)
        .unwrap()
        .is_empty());
}

#[test]
fn bad_dates_are_readable_errors() {
    assert!(App::in_memory().unwrap().recurring("nope", ALL.1).is_err());
}
