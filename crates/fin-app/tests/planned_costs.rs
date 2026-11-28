use fin_app::chat::Change;
use fin_app::App;
use fin_core::NaiveDate;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}
fn app() -> App {
    App::in_memory().unwrap().with_clock(today)
}
fn changes(a: &App, q: &str) -> Vec<Change> {
    a.prepare_whatif(q, today())
        .unwrap()
        .run(20)
        .unwrap()
        .changes
}
fn costs(c: &[Change]) -> Vec<(i32, i64)> {
    c.iter()
        .filter_map(|c| match c {
            Change::PlannedCost { year, cents, .. } => Some((*year, *cents)),
            _ => None,
        })
        .collect()
}

#[test]
fn one_off_costs_are_read_from_plain_words() {
    let a = app();
    assert_eq!(
        costs(&changes(&a, "what if we buy a house for $400k in 2030")),
        vec![(2030, 40_000_000)]
    );
    assert_eq!(
        costs(&changes(&a, "a one-time expense of $20k in 2028")),
        vec![(2028, 2_000_000)]
    );
    // "spend" with one year and no "a year" is a one-off, not a new spending level.
    let c = changes(&a, "spend $20k in 2028");
    assert_eq!(costs(&c), vec![(2028, 2_000_000)]);
    assert!(!c.iter().any(|c| matches!(c, Change::Spending { .. })));
    // Ongoing spending still works.
    let c = changes(&a, "spend $80k a year");
    assert!(c
        .iter()
        .any(|c| matches!(c, Change::Spending { cents: 8_000_000 })));
    assert!(costs(&c).is_empty());
}

#[test]
fn a_span_of_years_repeats_the_cost() {
    let a = app();
    let c = costs(&changes(&a, "college costs $30k a year from 2036 to 2038"));
    assert_eq!(
        c,
        vec![(2036, 3_000_000), (2037, 3_000_000), (2038, 3_000_000)]
    );
    // Without "a year" the amount is a total spread over the span.
    let c = costs(&changes(&a, "college costs $90k from 2036 to 2038"));
    assert_eq!(
        c,
        vec![(2036, 3_000_000), (2037, 3_000_000), (2038, 3_000_000)]
    );
    let c = costs(&changes(
        &a,
        "tuition of $20k a year for 4 years starting 2030",
    ));
    assert_eq!(c.len(), 4);
    assert_eq!(c[0].0, 2030);
}

#[test]
fn inheritances_and_home_sales_are_money_coming_in() {
    let a = app();
    assert_eq!(
        costs(&changes(&a, "an inheritance of $200k in 2032")),
        vec![(2032, -20_000_000)]
    );
    assert_eq!(
        costs(&changes(&a, "sell the house in 2035 for $500k")),
        vec![(2035, -50_000_000)]
    );
    // A bonus is still income.
    let c = changes(&a, "a bonus of $100k in 2028");
    assert!(c.iter().any(|c| matches!(c, Change::OneTime { .. })));
    assert!(costs(&c).is_empty());
}

#[test]
fn a_big_cost_hurts_the_plan_and_an_inflow_helps_it() {
    let a = app();
    let rate = |q: &str| {
        a.prepare_whatif(q, today())
            .unwrap()
            .run(300)
            .unwrap()
            .after
            .unwrap()
            .success_rate
    };
    let base = a
        .prepare_whatif("buy a house for $1m in 2030", today())
        .unwrap()
        .run(300)
        .unwrap()
        .before
        .unwrap()
        .success_rate;
    assert!(rate("buy a house for $1m in 2030") <= base);
    assert!(rate("an inheritance of $1m in 2030") >= base);
}

#[test]
fn keeping_it_saves_the_event_in_the_household() {
    let a = app();
    let c = changes(&a, "buy a house for $400k in 2030");
    let h = a.apply_changes(&c).unwrap();
    assert_eq!(h.events.len(), 1);
    assert_eq!(h.events[0].year, 2030);
    // And a household saved before events existed still loads.
    let j = serde_json::to_string(&h).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&j).unwrap();
    v.as_object_mut().unwrap().remove("events");
    let old: fin_app::household::Household = serde_json::from_value(v).unwrap();
    assert!(old.events.is_empty());
}

#[test]
fn a_raise_scales_regular_income_but_not_one_time_amounts() {
    let a = app();
    let c = changes(&a, "what if I get a 10% raise");
    assert!(c.iter().any(|c| matches!(
        c,
        Change::Raise {
            person: 0,
            bp: 1000
        }
    )));
    let c = changes(&a, "a raise of 3.5 percent");
    assert!(c.iter().any(|c| matches!(c, Change::Raise { bp: 350, .. })));
    // Words without a number are not guessed at.
    assert!(changes(&a, "what if I get a raise").is_empty());

    let mut h = a.household().unwrap();
    let before = h.streams.clone();
    h.streams.push(fin_app::household::StreamRow {
        self_employed: false,
        kind: None,
        employer_match_cents: 0,
        hsa_cents: 0,
        label: "Bonus".into(),
        person: 0,
        annual_cents: 1_000_000,
        start_year: Some(2028),
        end_year: Some(2028),
        contribution_cents: 0,
    });
    Change::Raise {
        person: 0,
        bp: 1000,
    }
    .apply(&mut h)
    .unwrap();
    for (i, s) in before.iter().enumerate().filter(|(_, s)| s.person == 0) {
        assert_eq!(h.streams[i].annual_cents, s.annual_cents * 11 / 10);
    }
    assert_eq!(h.streams.last().unwrap().annual_cents, 1_000_000);
    assert!(Change::Raise { person: 9, bp: 100 }.apply(&mut h).is_err());
}

#[test]
fn a_raise_helps_the_plan() {
    let a = app();
    let r = a
        .prepare_whatif("a 10% raise", today())
        .unwrap()
        .run(300)
        .unwrap();
    assert!(r.after.unwrap().success_rate >= r.before.unwrap().success_rate);
    assert!(
        r.descriptions[0].contains("10% raise"),
        "{:?}",
        r.descriptions
    );
}

#[test]
fn a_dashed_span_of_years_is_read_like_from_to() {
    let a = app();
    assert_eq!(
        costs(&changes(&a, "college costs $30k a year 2036-2038")),
        vec![(2036, 3_000_000), (2037, 3_000_000), (2038, 3_000_000)]
    );
}
