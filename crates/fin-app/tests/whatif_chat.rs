use fin_app::chat::{parse_amount, Change, Interpretation, Interpreter, RuleInterpreter};
use fin_app::household::Household;
use fin_app::App;
use fin_core::NaiveDate;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}
const TODAY: fn() -> NaiveDate = || d(2026, 11, 16);
fn app() -> App {
    App::in_memory().unwrap().with_clock(TODAY)
}
fn interpret(text: &str) -> Vec<Change> {
    RuleInterpreter
        .interpret(text, &Household::sample(), TODAY())
        .changes
}

#[test]
fn amounts_in_the_ways_people_type_them() {
    assert_eq!(parse_amount("80k"), Some(8_000_000));
    assert_eq!(parse_amount("$80,000"), Some(8_000_000));
    assert_eq!(parse_amount("1.2m"), Some(120_000_000));
    assert_eq!(parse_amount("80000"), Some(8_000_000));
    assert_eq!(parse_amount("$1,234.50"), Some(123_450));
    assert_eq!(parse_amount("lots"), None);
    assert_eq!(parse_amount("-5"), None);
}

#[test]
fn retirement_age_phrasings() {
    for t in [
        "What if I retire at 65?",
        "retiring at age 65",
        "what if we retire at 65 instead",
    ] {
        assert_eq!(
            interpret(t),
            vec![Change::RetireAt { person: 0, age: 65 }],
            "{t}"
        );
    }
    assert!(
        interpret("retire at 12").is_empty(),
        "nonsense ages are ignored"
    );
}

#[test]
fn spending_phrasings_including_monthly() {
    assert_eq!(
        interpret("spend $80k a year"),
        vec![Change::Spending { cents: 8_000_000 }]
    );
    assert_eq!(
        interpret("What if spending is 7,000 a month"),
        vec![Change::Spending { cents: 8_400_000 }]
    );
}

#[test]
fn market_and_horizon_what_ifs_become_picks() {
    let pick = |k: &str, l: &str| Change::SetPick {
        key: k.into(),
        label: l.into(),
    };
    assert_eq!(
        interpret("what if the market crashes"),
        vec![pick("real_return", "Cautious")]
    );
    assert_eq!(
        interpret("assume optimistic returns"),
        vec![pick("real_return", "Optimistic")]
    );
    assert_eq!(
        interpret("what if we live to 100"),
        vec![pick("horizon_age", "Age 100")]
    );
    assert_eq!(
        interpret("claim social security at 70"),
        vec![pick("ss_claim_age", "Age 70")]
    );
    assert_eq!(
        interpret("spending declines as we age"),
        vec![pick("spending_change", "Declines")]
    );
    assert_eq!(
        interpret("more risk, all stocks"),
        vec![pick("return_volatility", "Aggressive")]
    );
}

#[test]
fn moving_by_name_or_code() {
    assert_eq!(
        interpret("what if we move to California"),
        vec![Change::MoveTo { state: "CA".into() }]
    );
    assert_eq!(
        interpret("moving to ny"),
        vec![Change::MoveTo { state: "NY".into() }]
    );
    assert!(interpret("move to Narnia").is_empty());
}

#[test]
fn one_time_income_needs_an_amount_and_a_future_year() {
    assert_eq!(
        interpret("a bonus of $100k in 2028"),
        vec![Change::OneTime {
            label: "Bonus".into(),
            person: 0,
            cents: 10_000_000,
            year: 2028
        }]
    );
    assert!(interpret("a bonus of $100k").is_empty());
    assert!(interpret("a bonus of $100k in 1999").is_empty());
}

#[test]
fn several_changes_in_one_sentence_and_named_people() {
    let mut h = Household::sample();
    h.people.push(fin_app::household::PersonRow {
        name: "Sam".into(),
        birth_date: "1983-02-10".into(),
        retire_age: 60,
        ss_annual_cents: 0,
    });
    let got = RuleInterpreter
        .interpret(
            "what if Sam retires at 55 and the market crashes",
            &h,
            TODAY(),
        )
        .changes;
    assert_eq!(got[0], Change::RetireAt { person: 1, age: 55 });
    assert_eq!(got.len(), 2);
}

#[test]
fn unknown_text_gets_help_and_no_plan_runs() {
    let app = app();
    let r = app
        .prepare_whatif("tell me a joke", TODAY())
        .unwrap()
        .run(50)
        .unwrap();
    assert!(!r.understood);
    assert!(r.message.contains("Try things like"));
    assert!(r.before.is_none() && r.after.is_none());
}

#[test]
fn a_what_if_compares_before_and_after_and_saves_nothing() {
    let app = app();
    let before_saved = app.household().unwrap();
    let r = app
        .prepare_whatif("what if the market crashes and I spend $200k", TODAY())
        .unwrap()
        .run(300)
        .unwrap();
    assert!(r.understood);
    assert_eq!(r.descriptions.len(), 2);
    let (b, a) = (r.before.unwrap(), r.after.unwrap());
    assert!(a.success_rate < b.success_rate);
    assert_eq!(
        app.household().unwrap(),
        before_saved,
        "preview must not change saved data"
    );
}

#[test]
fn keeping_a_what_if_saves_it_and_the_next_plan_reflects_it() {
    let app = app();
    let prep = app
        .prepare_whatif(
            "retire at 65 and move to Florida and a bonus of $50k in 2028",
            TODAY(),
        )
        .unwrap();
    let changes = prep.changes.clone();
    assert_eq!(changes.len(), 3);
    let h = app.apply_changes(&changes).unwrap();
    assert_eq!(h.people[0].retire_age, 65);
    assert_eq!(h.state, "FL");
    assert!(h
        .streams
        .iter()
        .any(|s| s.label == "Bonus" && s.start_year == Some(2028)));
}

#[test]
fn an_impossible_what_if_explains_itself() {
    let app = app();
    // 40 is below the sample's age (45): the words parse, but the plan rejects it.
    let res = app.prepare_whatif("retire at 40", TODAY());
    assert!(res.is_err());
    assert!(res.err().unwrap().contains("doesn't work"));
}

#[test]
fn a_custom_interpreter_can_replace_the_rules() {
    struct Fixed;
    impl Interpreter for Fixed {
        fn interpret(&self, _: &str, _: &Household, _: NaiveDate) -> Interpretation {
            Interpretation {
                changes: vec![Change::Spending { cents: 1_000_000 }],
            }
        }
    }
    let app = app().with_interpreter(Box::new(Fixed));
    let r = app.prepare_whatif("anything at all", TODAY()).unwrap();
    assert_eq!(r.changes, vec![Change::Spending { cents: 1_000_000 }]);
}

#[test]
fn changes_survive_a_json_round_trip_for_the_ui() {
    let c = Change::OneTime {
        label: "Bonus".into(),
        person: 0,
        cents: 5,
        year: 2028,
    };
    let json = serde_json::to_string(&c).unwrap();
    assert!(
        json.contains("\"kind\":\"oneTime\"") || json.contains("\"kind\":\"OneTime\""),
        "{json}"
    );
    assert_eq!(serde_json::from_str::<Change>(&json).unwrap(), c);
}

#[test]
fn absurdly_large_amounts_are_not_understood() {
    assert_eq!(parse_amount("$1e30"), None);
    assert!(interpret("spend $1e30 a year").is_empty());
}

#[test]
fn moving_is_understood_when_to_appears_earlier_in_the_sentence() {
    for t in [
        "I want to move to Florida",
        "what if we decide to relocate to Texas",
    ] {
        assert!(
            matches!(interpret(t).as_slice(), [Change::MoveTo { .. }]),
            "{t}: {:?}",
            interpret(t)
        );
    }
    assert_eq!(
        interpret("I want to move to Florida"),
        vec![Change::MoveTo { state: "FL".into() }]
    );
}

#[test]
fn a_bonus_amount_written_before_the_word_is_not_read_as_the_year() {
    assert_eq!(
        interpret("what if I get a $50k bonus in 2028"),
        vec![Change::OneTime {
            label: "Bonus".into(),
            person: 0,
            cents: 5_000_000,
            year: 2028,
        }]
    );
}

#[test]
fn the_claim_age_comes_from_the_social_security_part_not_the_retirement_age() {
    assert_eq!(
        interpret("retire at 67 and claim social security at 70"),
        vec![
            Change::RetireAt { person: 0, age: 67 },
            Change::SetPick {
                key: "ss_claim_age".into(),
                label: "Age 70".into(),
            }
        ]
    );
}

#[test]
fn a_spending_cap_is_not_a_plan_horizon() {
    for text in [
        "what if we spend up to 90k a year",
        "spend up to 90,000",
        "what if we spend up to 95k",
        "cut spending to 90k",
    ] {
        assert!(
            !interpret(text)
                .iter()
                .any(|c| matches!(c, Change::SetPick { key, .. } if key == "horizon_age")),
            "{text}"
        );
    }
    assert_eq!(
        interpret("plan to age 90"),
        vec![Change::SetPick {
            key: "horizon_age".into(),
            label: "Age 90".into()
        }]
    );
}

#[test]
fn new_pensions_and_military_pay_become_lasting_income() {
    let inc = |label: &str, cents: i64, kind: &str, start: Option<i32>| Change::Income {
        label: label.into(),
        person: 0,
        cents,
        income_kind: kind.into(),
        start_year: start,
    };
    assert_eq!(
        interpret("what if I get a pension of $2,000 a month"),
        vec![inc("Pension", 2_400_000, "pension", None)]
    );
    assert_eq!(
        interpret("add $30k a year of military retired pay"),
        vec![inc(
            "Military retired pay",
            3_000_000,
            "military_pension",
            None
        )]
    );
    assert_eq!(
        interpret("a state pension of $40,000 a year starting in 2032"),
        vec![inc(
            "Government pension",
            4_000_000,
            "public_pension",
            Some(2032)
        )]
    );
    // An annuity from a year: not a planned cost in that year.
    assert_eq!(
        interpret("an annuity of $12k a year from 2030"),
        vec![inc("Annuity", 1_200_000, "pension", Some(2030))]
    );
    // Rental income is taxable income with no payroll tax, from now.
    assert_eq!(
        interpret("what if we get rental income of $1,500 a month"),
        vec![inc("Rental income", 1_800_000, "other", None)]
    );
    // Paying rent is not income.
    assert!(!interpret("we pay rent of $2,000 a month")
        .iter()
        .any(|c| matches!(c, Change::Income { .. })));
    // No amount, no change; and a year is not an amount.
    assert!(interpret("what about a pension").is_empty());
    assert!(interpret("pension in 2030").is_empty());
}

#[test]
fn an_income_change_adds_a_stream_of_its_kind() {
    let mut h = Household::sample();
    let n = h.streams.len();
    let c = Change::Income {
        label: "Military retired pay".into(),
        person: 0,
        cents: 3_000_000,
        income_kind: "military_pension".into(),
        start_year: None,
    };
    assert!(c
        .describe(&h)
        .contains("Military retired pay of $30,000 a year"));
    c.apply(&mut h).unwrap();
    let s = h.streams.last().unwrap();
    assert_eq!(h.streams.len(), n + 1);
    assert_eq!(s.kind.as_deref(), Some("military_pension"));
    assert_eq!(s.end_year, None);
    let bad = Change::Income {
        label: "x".into(),
        person: 0,
        cents: 1,
        income_kind: "lottery".into(),
        start_year: None,
    };
    assert!(bad.apply(&mut h).is_err());
}
