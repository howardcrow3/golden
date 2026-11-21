use fin_app::chat::Change;
use fin_app::App;
use fin_core::NaiveDate;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}
fn app() -> App {
    App::in_memory().unwrap().with_clock(today)
}
fn retire(age: i32) -> Vec<Change> {
    vec![Change::RetireAt { person: 0, age }]
}

#[test]
fn saves_lists_and_deletes() {
    let a = app();
    assert!(a.list_scenarios().unwrap().is_empty());
    let s = a
        .save_scenario("Retire at 62", &retire(62), today())
        .unwrap();
    assert_eq!(s.id, 1);
    assert!(s.descriptions[0].contains("62"));
    let t = a
        .save_scenario("Retire at 67", &retire(67), today())
        .unwrap();
    assert_eq!(t.id, 2);
    assert_eq!(a.list_scenarios().unwrap().len(), 2);
    a.delete_scenario(1).unwrap();
    assert_eq!(a.list_scenarios().unwrap()[0].name, "Retire at 67");
    assert!(a.delete_scenario(1).is_err());
    // Ids are never reused while a higher one exists.
    assert_eq!(
        a.save_scenario("Again", &retire(64), today()).unwrap().id,
        3
    );
}

#[test]
fn refuses_bad_saves() {
    let a = app();
    assert!(a.save_scenario("  ", &retire(62), today()).is_err());
    assert!(a
        .save_scenario(&"x".repeat(61), &retire(62), today())
        .is_err());
    assert!(a.save_scenario("Empty", &[], today()).is_err());
    a.save_scenario("One", &retire(62), today()).unwrap();
    assert!(a.save_scenario("one", &retire(63), today()).is_err());
    // Retiring before today's age can't run.
    assert!(a.save_scenario("Too early", &retire(5), today()).is_err());
    assert_eq!(a.list_scenarios().unwrap().len(), 1);
}

#[test]
fn compares_up_to_three_beside_the_current_plan() {
    let a = app();
    for (n, age) in [("A", 60), ("B", 65), ("C", 70), ("D", 75)] {
        a.save_scenario(n, &retire(age), today()).unwrap();
    }
    let reply = a
        .prepare_compare(&[1, 2, 3], today())
        .unwrap()
        .run(50)
        .unwrap();
    let names: Vec<_> = reply.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["Current plan", "A", "B", "C"]);
    assert!(reply.columns[0].descriptions.is_empty());
    assert!(a.prepare_compare(&[1, 2, 3, 4], today()).is_err());
    assert!(a.prepare_compare(&[], today()).is_err());
    assert!(a.prepare_compare(&[99], today()).is_err());
}

#[test]
fn make_plan_changes_the_household_and_undo_restores_it() {
    let a = app();
    let before = a.household().unwrap();
    let s = a
        .save_scenario("Retire at 58", &retire(58), today())
        .unwrap();
    assert!(a.undoable_plan().unwrap().is_none());
    let after = a.make_plan(s.id).unwrap();
    assert_ne!(
        serde_json::to_string(&before).unwrap(),
        serde_json::to_string(&after).unwrap()
    );
    assert_eq!(a.undoable_plan().unwrap().as_deref(), Some("Retire at 58"));
    let back = a.undo_make_plan().unwrap();
    assert_eq!(
        serde_json::to_string(&before).unwrap(),
        serde_json::to_string(&back).unwrap()
    );
    assert!(a.undoable_plan().unwrap().is_none());
    assert!(a.undo_make_plan().is_err());
}

#[test]
fn stacked_changes_apply_in_order() {
    let a = app();
    let changes = vec![
        Change::RetireAt { person: 0, age: 58 },
        Change::Spending { cents: 6_000_000 },
    ];
    let s = a
        .save_scenario("Lean and early", &changes, today())
        .unwrap();
    assert_eq!(s.descriptions.len(), 2);
    let h = a.make_plan(s.id).unwrap();
    assert_eq!(h.people[0].retire_age, 58);
}

#[test]
fn a_what_if_can_be_stacked_on_a_saved_scenario() {
    let a = app();
    a.save_scenario("Retire at 62", &retire(62), today())
        .unwrap();
    let prep = a
        .prepare_whatif_on("spend $80k a year", today(), Some(1))
        .unwrap();
    // The stack carries the scenario's change first, then the new one.
    let reply = prep.run(30).unwrap();
    assert!(reply.understood);
    assert_eq!(reply.changes.len(), 2);
    assert_eq!(reply.changes[0], retire(62)[0]);
    assert_eq!(reply.descriptions.len(), 1);
    // The whole stack can be saved as its own scenario.
    let s = a
        .save_scenario("Retire at 62, spend less", &reply.changes, today())
        .unwrap();
    assert_eq!(s.changes.len(), 2);
    // Without a base it behaves as before.
    let plain = a
        .prepare_whatif_on("spend $80k a year", today(), None)
        .unwrap()
        .run(30)
        .unwrap();
    assert_eq!(plain.changes.len(), 1);
    assert!(a
        .prepare_whatif_on("spend $80k a year", today(), Some(99))
        .is_err());
}
