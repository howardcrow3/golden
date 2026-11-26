use fin_app::watchlists::WatchList;
use fin_app::App;
use fin_core::NaiveDate;

fn app() -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap())
}

fn wl(n: &str, q: &str) -> WatchList {
    WatchList {
        name: n.into(),
        query: q.into(),
    }
}

#[test]
fn lists_are_saved_trimmed_and_read_back() {
    let app = app();
    assert!(app.watch_lists().unwrap().is_empty());
    app.save_watch_lists(vec![wl(" Coffee ", " cafe starbucks "), wl("Trip", "trip")])
        .unwrap();
    assert_eq!(
        app.watch_lists().unwrap(),
        vec![wl("Coffee", "cafe starbucks"), wl("Trip", "trip")]
    );
    app.save_watch_lists(vec![]).unwrap();
    assert!(app.watch_lists().unwrap().is_empty());
}

#[test]
fn blank_duplicate_and_too_many_are_refused() {
    let app = app();
    assert!(app.save_watch_lists(vec![wl("", "x")]).is_err());
    assert!(app.save_watch_lists(vec![wl("A", " ")]).is_err());
    assert!(app
        .save_watch_lists(vec![wl("A", "x"), wl("a", "y")])
        .unwrap_err()
        .contains("already"));
    let many: Vec<_> = (0..21).map(|i| wl(&format!("L{i}"), "x")).collect();
    assert!(app.save_watch_lists(many).is_err());
    assert!(app.watch_lists().unwrap().is_empty());
}
