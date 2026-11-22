use fin_app::App;

const W2: &str = "Form W-2 Wage and Tax Statement 2025\nWages, tips, other compensation 85,000.00\nFederal income tax withheld 12,000.00\n";

#[test]
fn a_document_is_stored_recognised_and_read_back_byte_for_byte() {
    let a = App::in_memory().unwrap();
    let d = a.vault_add("My W-2.txt", W2.as_bytes()).unwrap();
    assert_eq!(d.kind, "W-2");
    assert_eq!(d.year, Some(2025));
    assert_eq!(d.size, W2.len() as i64);
    assert!(d
        .fields
        .iter()
        .any(|f| f.label == "Wages" && f.cents == 8_500_000));
    let (name, bytes) = a.vault_get(d.id).unwrap();
    assert_eq!(name, "My W-2.txt");
    assert_eq!(bytes, W2.as_bytes());
}

#[test]
fn binary_files_are_kept_without_text_and_listed_as_documents() {
    let a = App::in_memory().unwrap();
    let d = a
        .vault_add("scan.png", &[0x89, b'P', b'N', b'G', 0, 1, 2])
        .unwrap();
    assert_eq!(d.kind, "Document");
    assert!(d.fields.is_empty());
}

#[test]
fn search_matches_names_kinds_and_inner_text_and_delete_removes() {
    let a = App::in_memory().unwrap();
    let w2 = a.vault_add("w2.txt", W2.as_bytes()).unwrap();
    a.vault_add("note.txt", b"keep the roof receipts").unwrap();
    assert_eq!(a.vault_list("").unwrap().len(), 2);
    assert_eq!(a.vault_list("wage and tax").unwrap().len(), 1);
    assert_eq!(a.vault_list("W-2").unwrap()[0].id, w2.id);
    assert_eq!(a.vault_list("roof").unwrap().len(), 1);
    assert!(a.vault_list("nothing like this").unwrap().is_empty());
    a.vault_delete(w2.id).unwrap();
    assert_eq!(a.vault_list("").unwrap().len(), 1);
    assert!(a
        .vault_delete(w2.id)
        .unwrap_err()
        .contains("isn't in the vault"));
    assert!(a.vault_get(w2.id).is_err());
}

#[test]
fn bad_input_is_a_readable_error() {
    let a = App::in_memory().unwrap();
    assert!(a.vault_add(" ", b"x").unwrap_err().contains("name"));
    assert!(a.vault_add("a", b"").unwrap_err().contains("empty"));
    let big = vec![b'a'; fin_app::vault::MAX_BYTES + 1];
    assert!(a.vault_add("big", &big).unwrap_err().contains("25 MB"));
}
