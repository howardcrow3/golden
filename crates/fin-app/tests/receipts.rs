use fin_app::App;
use fin_core::NaiveDate;

fn app() -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 20).unwrap())
}

fn receipt(shop: &str, date: &str, total: &str) -> String {
    format!(
        "{shop}\n12 Main St\n{date} 4:32 PM\nItem one 1.00\nSUBTOTAL {total}\nTAX 0.00\nTOTAL ${total}\nVISA ****1234\nThank you for shopping"
    )
}

fn statement(a: &mut App) {
    a.import_statement(
        b"Date,Description,Amount\n10/17/2026,TRADER JOES #55,-24.18\n10/19/2026,SHELL OIL 5744,-41.00\n10/20/2026,ACME PAYROLL,2500.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
}

#[test]
fn a_receipt_is_recognised_with_its_total() {
    let a = app();
    let d = a
        .vault_add(
            "tj.txt",
            receipt("TRADER JOE'S #123", "10/17/2026", "24.18").as_bytes(),
        )
        .unwrap();
    assert_eq!(d.kind, "Receipt");
    assert!(d
        .fields
        .iter()
        .any(|f| f.label == "Total" && f.cents == 2_418));
}

#[test]
fn a_receipt_links_to_its_one_matching_charge() {
    let mut a = app();
    statement(&mut a);
    a.vault_add(
        "tj.txt",
        receipt("TRADER JOE'S #123", "10/17/2026", "24.18").as_bytes(),
    )
    .unwrap();
    let r = a.match_receipts().unwrap();
    assert_eq!(r.auto_linked, 1);
    let item = &r.items[0];
    assert_eq!(item.status, "linked");
    assert_eq!(item.linked.as_ref().unwrap().cents, -2_418);
    // Running it again changes nothing.
    let again = a.match_receipts().unwrap();
    assert_eq!(again.auto_linked, 0);
    assert_eq!(again.items[0].status, "linked");
}

#[test]
fn a_receipt_adds_no_expense_of_its_own() {
    let mut a = app();
    statement(&mut a);
    let before = a
        .transactions("2026-10-01", "2026-10-31", false)
        .unwrap()
        .len();
    a.vault_add(
        "tj.txt",
        receipt("Trader Joe's", "10/17/2026", "24.18").as_bytes(),
    )
    .unwrap();
    a.match_receipts().unwrap();
    let after = a
        .transactions("2026-10-01", "2026-10-31", false)
        .unwrap()
        .len();
    assert_eq!(before, after);
}

#[test]
fn a_receipt_before_its_statement_waits_unmatched_then_links() {
    let mut a = app();
    a.vault_add(
        "tj.txt",
        receipt("TRADER JOE'S #123", "10/17/2026", "24.18").as_bytes(),
    )
    .unwrap();
    assert_eq!(a.match_receipts().unwrap().items[0].status, "unmatched");
    statement(&mut a);
    let r = a.match_receipts().unwrap();
    assert_eq!((r.auto_linked, r.items[0].status.as_str()), (1, "linked"));
}

#[test]
fn the_same_receipt_twice_is_flagged_not_linked_twice() {
    let mut a = app();
    statement(&mut a);
    let text = receipt("TRADER JOE'S #123", "10/17/2026", "24.18");
    a.vault_add("one.txt", text.as_bytes()).unwrap();
    a.vault_add("two.txt", text.as_bytes()).unwrap();
    let r = a.match_receipts().unwrap();
    let status: Vec<&str> = r.items.iter().map(|i| i.status.as_str()).collect();
    assert_eq!(status, ["linked", "duplicate"]);
}

#[test]
fn two_possible_charges_are_suggested_not_guessed() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n10/17/2026,CORNER CAFE,-9.50\n10/18/2026,CORNER CAFE,-9.50\n",
        "Checking",
        "checking",
    )
    .unwrap();
    a.vault_add(
        "cafe.txt",
        receipt("Corner Cafe", "10/17/2026", "9.50").as_bytes(),
    )
    .unwrap();
    let item = &a.match_receipts().unwrap().items[0];
    assert_eq!(item.status, "suggested");
    assert_eq!(item.candidates.len(), 2);
    // Confirming one links it; unlinking puts it back.
    a.link_receipt(item.document_id, item.candidates[0].id)
        .unwrap();
    assert_eq!(a.match_receipts().unwrap().items[0].status, "linked");
    a.unlink_receipt(item.document_id).unwrap();
    assert_ne!(a.match_receipts().unwrap().items[0].status, "linked");
}

#[test]
fn wrong_amount_or_date_is_not_matched() {
    let mut a = app();
    statement(&mut a);
    a.vault_add(
        "a.txt",
        receipt("Trader Joe's", "10/17/2026", "24.19").as_bytes(),
    )
    .unwrap();
    a.vault_add(
        "b.txt",
        receipt("Trader Joe's", "09/01/2026", "24.18").as_bytes(),
    )
    .unwrap();
    let r = a.match_receipts().unwrap();
    assert!(r.items.iter().all(|i| i.status == "unmatched"));
}
