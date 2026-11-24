use fin_db::documents::NewDoc;
use fin_db::Db;

fn doc<'a>(name: &'a str, kind: &'a str, added: &'a str, body: &'a str) -> NewDoc<'a> {
    NewDoc {
        name,
        kind,
        year: Some(2025),
        added,
        fields: "{}",
        body,
        content: b"pdf-bytes",
    }
}

#[test]
fn add_returns_the_stored_row_with_its_size() {
    let db = Db::open_in_memory().unwrap();
    let row = db
        .add_document(&doc("w2.pdf", "W-2", "2026-01-05", "wages"))
        .unwrap();
    assert_eq!(row.name, "w2.pdf");
    assert_eq!(row.size, 9);
    assert_eq!(row.year, Some(2025));
}

#[test]
fn list_is_newest_first_and_searches_name_kind_and_text() {
    let db = Db::open_in_memory().unwrap();
    db.add_document(&doc("old.pdf", "1099", "2026-01-01", "interest"))
        .unwrap();
    db.add_document(&doc("new.pdf", "W-2", "2026-02-01", "wages"))
        .unwrap();
    let all = db.list_documents("").unwrap();
    assert_eq!(all[0].name, "new.pdf");
    assert_eq!(db.list_documents("1099").unwrap().len(), 1);
    assert_eq!(db.list_documents("wages").unwrap()[0].name, "new.pdf");
    assert_eq!(db.list_documents("old").unwrap()[0].kind, "1099");
    assert!(db.list_documents("nothing-here").unwrap().is_empty());
}

#[test]
fn like_wildcards_in_a_search_are_not_special() {
    let db = Db::open_in_memory().unwrap();
    db.add_document(&doc("a.pdf", "W-2", "2026-01-01", "x"))
        .unwrap();
    assert_eq!(db.list_documents("%").unwrap().len(), 1);
    assert_eq!(db.list_documents("_").unwrap().len(), 1);
}

#[test]
fn get_and_delete_document() {
    let db = Db::open_in_memory().unwrap();
    let row = db
        .add_document(&doc("a.pdf", "W-2", "2026-01-01", "x"))
        .unwrap();
    let (name, bytes) = db.get_document(row.id).unwrap().unwrap();
    assert_eq!(
        (name.as_str(), bytes.as_slice()),
        ("a.pdf", &b"pdf-bytes"[..])
    );
    assert!(db.delete_document(row.id).unwrap());
    assert!(!db.delete_document(row.id).unwrap());
    assert!(db.get_document(row.id).unwrap().is_none());
}

#[test]
fn erase_personal_data_keeps_named_settings_and_removes_documents() {
    let db = Db::open_in_memory().unwrap();
    db.set_setting("trial.start", "2026-11-01").unwrap();
    db.set_setting("note", "private").unwrap();
    db.add_document(&doc("a.pdf", "W-2", "2026-01-01", "x"))
        .unwrap();
    db.erase_personal_data(&["trial.start"]).unwrap();
    assert_eq!(
        db.get_setting("trial.start").unwrap().as_deref(),
        Some("2026-11-01")
    );
    assert_eq!(db.get_setting("note").unwrap(), None);
    assert!(db.list_documents("").unwrap().is_empty());
}
