use fin_app::App;
use fin_core::NaiveDate;

const CSV: &str = "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n10/06/2026,MYSTERY VENDOR 22,-10.00\n10/09/2026,MYSTERY VENDOR 31 PORTLAND,-4.00\n10/10/2026,NETFLIX.COM,-15.49\n";

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// The "MYSTERY VENDOR 22" line (the shortest description, so its stem is just the merchant).
fn mystery_22(app: &App) -> i64 {
    app.transactions("2026-10-01", "2026-10-31", true)
        .unwrap()
        .into_iter()
        .find(|t| t.payee == "MYSTERY VENDOR 22")
        .unwrap()
        .id
}

fn loaded() -> App {
    let mut app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 16));
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    app
}

#[test]
fn lists_newest_first_and_can_filter_to_uncategorized() {
    let app = loaded();
    let all = app.transactions("2026-10-01", "2026-10-31", false).unwrap();
    assert_eq!(all.len(), 4);
    assert_eq!(all[0].day, "2026-10-10");
    assert_eq!(all[0].account, "Checking");
    let todo = app.transactions("2026-10-01", "2026-10-31", true).unwrap();
    assert_eq!(todo.len(), 2);
    assert!(todo.iter().all(|t| t.payee.contains("MYSTERY")));
    assert!(app.transactions("bad", "2026-10-31", false).is_err());
}

#[test]
fn one_off_change_touches_only_that_transaction() {
    let mut app = loaded();
    let todo = app.transactions("2026-10-01", "2026-10-31", true).unwrap();
    let r = app.set_category(todo[0].id, "Hobbies", false).unwrap();
    assert_eq!(r.also_updated, 0);
    assert_eq!(
        app.transactions("2026-10-01", "2026-10-31", true)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn remembering_fixes_the_matching_lines_now_and_next_time() {
    let mut app = loaded();
    let id = mystery_22(&app);
    let r = app.set_category(id, "Hobbies", true).unwrap();
    assert_eq!(
        r.also_updated, 1,
        "the other MYSTERY VENDOR line is filed too"
    );
    assert!(app
        .transactions("2026-10-01", "2026-10-31", true)
        .unwrap()
        .is_empty());
    let more = "Date,Description,Amount\n11/03/2026,MYSTERY VENDOR 77,-8.00\n";
    app.import_statement(more.as_bytes(), "Checking", "checking")
        .unwrap();
    let nov = app.transactions("2026-11-01", "2026-11-30", false).unwrap();
    assert_eq!(nov[0].category.as_deref(), Some("Hobbies"));
}

#[test]
fn what_you_taught_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("g.db");
    {
        let mut app = App::new(fin_db::Db::open(&path).unwrap()).with_clock(|| d(2026, 11, 16));
        app.import_statement(CSV.as_bytes(), "Checking", "checking")
            .unwrap();
        let id = mystery_22(&app);
        app.set_category(id, "Hobbies", true).unwrap();
    }
    let mut app = App::new(fin_db::Db::open(&path).unwrap()).with_clock(|| d(2026, 11, 16));
    let more = "Date,Description,Amount\n11/03/2026,MYSTERY VENDOR 90,-8.00\n";
    app.import_statement(more.as_bytes(), "Checking", "checking")
        .unwrap();
    let nov = app.transactions("2026-11-01", "2026-11-30", false).unwrap();
    assert_eq!(nov[0].category.as_deref(), Some("Hobbies"));
}

#[test]
fn category_totals_follow_the_change() {
    let mut app = loaded();
    let id = mystery_22(&app);
    app.set_category(id, "Hobbies", true).unwrap();
    let cats = app.categories(d(2026, 10, 15), "month", false).unwrap();
    assert!(cats
        .iter()
        .any(|c| c.category == "Hobbies" && c.cents == 1_400));
    assert!(!cats.iter().any(|c| c.category == "Uncategorized"));
}

#[test]
fn choices_include_builtin_and_new_categories() {
    let mut app = loaded();
    let t = app.transactions("2026-10-01", "2026-10-31", true).unwrap();
    app.set_category(t[0].id, "Hobbies", false).unwrap();
    let c = app.category_choices().unwrap();
    assert!(c.contains(&"Groceries".to_string()));
    assert!(c.contains(&"Hobbies".to_string()));
    let mut sorted = c.clone();
    sorted.sort();
    assert_eq!(c, sorted);
}

#[test]
fn errors_are_readable_and_nothing_changes() {
    let mut app = loaded();
    assert!(app.set_category(9999, "X", false).is_err());
    let t = app.transactions("2026-10-01", "2026-10-31", true).unwrap();
    assert!(app
        .set_category(t[0].id, "  ", false)
        .unwrap_err()
        .contains("category"));
    assert_eq!(
        app.transactions("2026-10-01", "2026-10-31", true)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn an_expired_trial_cannot_recategorize() {
    let mut app = loaded();
    app.db().set_setting("first_run", "2026-01-01").unwrap(); // trial began long ago
    let t = app.transactions("2026-10-01", "2026-10-31", true).unwrap();
    assert!(app
        .set_category(t[0].id, "Hobbies", true)
        .unwrap_err()
        .contains("trial"));
}
