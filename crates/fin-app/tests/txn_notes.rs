use fin_app::App;
use fin_core::NaiveDate;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

const CSV: &str =
    "Date,Description,Amount\n10/06/2026,CORNER CAFE,-10.50\n10/10/2026,NETFLIX.COM,-15.49\n";

fn loaded() -> App {
    let mut app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 16));
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    app
}

fn rows(app: &App) -> Vec<fin_app::transactions::TxnRow> {
    app.transactions("2026-10-01", "2026-10-31", false).unwrap()
}

#[test]
fn rows_start_without_notes_or_tags() {
    let app = loaded();
    assert!(rows(&app)
        .iter()
        .all(|r| r.note.is_empty() && r.tags.is_empty()));
}

#[test]
fn note_and_tags_are_saved_cleaned_and_listed() {
    let mut app = loaded();
    let id = rows(&app)
        .iter()
        .find(|r| r.payee.contains("CAFE"))
        .unwrap()
        .id;
    app.set_note_tags(id, "  lunch with Sam ", "Work, work , Trip,,")
        .unwrap();
    let r = rows(&app).into_iter().find(|r| r.id == id).unwrap();
    assert_eq!(r.note, "lunch with Sam");
    assert_eq!(r.tags, vec!["work", "trip"]);
    let other = rows(&app).into_iter().find(|r| r.id != id).unwrap();
    assert!(other.note.is_empty() && other.tags.is_empty());
}

#[test]
fn clearing_removes_them_and_unknown_ids_fail() {
    let mut app = loaded();
    let id = rows(&app)[0].id;
    app.set_note_tags(id, "x", "a").unwrap();
    app.set_note_tags(id, "", "").unwrap();
    let r = rows(&app).into_iter().find(|r| r.id == id).unwrap();
    assert!(r.note.is_empty() && r.tags.is_empty());
    assert!(app.set_note_tags(9999, "x", "").is_err());
}

use fin_app::transactions::SplitPart;

fn part(c: &str, cents: i64) -> SplitPart {
    SplitPart {
        category: c.into(),
        cents,
    }
}

#[test]
fn a_split_must_add_up_and_point_the_same_way() {
    let mut app = loaded();
    let id = rows(&app)
        .iter()
        .find(|r| r.payee.contains("CAFE"))
        .unwrap()
        .id; // -1050
    assert!(app.set_splits(id, vec![part("Food", -1050)]).is_err());
    assert!(app
        .set_splits(id, vec![part("Food", -500), part("Fun", -500)])
        .is_err());
    assert!(app
        .set_splits(id, vec![part("Food", -2000), part("Fun", 950)])
        .is_err());
    assert!(app
        .set_splits(id, vec![part("", -500), part("Fun", -550)])
        .is_err());
    app.set_splits(id, vec![part("Food", -750), part("Fun", -300)])
        .unwrap();
    let r = rows(&app).into_iter().find(|r| r.id == id).unwrap();
    assert_eq!(r.splits, vec![part("Food", -750), part("Fun", -300)]);
}

#[test]
fn split_parts_feed_the_category_totals_instead_of_the_line_category() {
    let mut app = loaded();
    let id = rows(&app)
        .iter()
        .find(|r| r.payee.contains("CAFE"))
        .unwrap()
        .id;
    app.set_splits(id, vec![part("Gifts", -750), part("Fun", -300)])
        .unwrap();
    let spend = app
        .categories(d(2026, 10, 15), "month", false)
        .unwrap()
        .into_iter()
        .map(|r| (r.category, r.cents))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(spend.get("Gifts"), Some(&750));
    assert_eq!(spend.get("Fun"), Some(&300));
    assert_eq!(spend.values().sum::<i64>(), 1050 + 1549);
}

#[test]
fn clearing_a_split_restores_the_line_category() {
    let mut app = loaded();
    let id = rows(&app)
        .iter()
        .find(|r| r.payee.contains("CAFE"))
        .unwrap()
        .id;
    app.set_splits(id, vec![part("Gifts", -750), part("Fun", -300)])
        .unwrap();
    app.set_splits(id, vec![]).unwrap();
    assert!(rows(&app)
        .into_iter()
        .find(|r| r.id == id)
        .unwrap()
        .splits
        .is_empty());
}
