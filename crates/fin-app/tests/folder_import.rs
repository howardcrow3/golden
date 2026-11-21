use fin_app::folder::ofx_account;
use fin_app::App;
use fin_core::NaiveDate;
use std::fs;

fn app() -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap())
}

const OFX: &str = "OFXHEADER:100\n<OFX><SIGNONMSGSRSV1><SONRS><FI><ORG>First Bank<FID>1</FI></SONRS></SIGNONMSGSRSV1><BANKMSGSRSV1><STMTTRNRS><STMTRS><BANKACCTFROM><ACCTID>000012345678<ACCTTYPE>CHECKING</BANKACCTFROM><BANKTRANLIST>\
<STMTTRN><DTPOSTED>20261001<TRNAMT>5000.00<NAME>PAYROLL</STMTTRN>\
<STMTTRN><DTPOSTED>20261003<TRNAMT>-1500.00<NAME>LANDLORD</STMTTRN>\
</BANKTRANLIST></STMTRS></STMTTRNRS></BANKMSGSRSV1></OFX>";
const CSV: &str = "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n";

#[test]
fn ofx_names_its_own_account() {
    assert_eq!(
        ofx_account(OFX.as_bytes()),
        Some(("First Bank 5678".to_string(), "checking"))
    );
    let card = "<OFX><CREDITCARDMSGSRSV1><CCSTMTRS><CCACCTFROM><ACCTID>4111</CCACCTFROM></CCSTMTRS></CREDITCARDMSGSRSV1></OFX>";
    assert_eq!(
        ofx_account(card.as_bytes()),
        Some(("Bank 4111".into(), "credit"))
    );
    assert_eq!(ofx_account(CSV.as_bytes()), None);
}

#[test]
fn imports_new_files_once_and_skips_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.ofx"), OFX).unwrap();
    fs::write(dir.path().join("b.csv"), CSV).unwrap();
    fs::write(dir.path().join("notes.txt"), "ignore me").unwrap();
    fs::create_dir(dir.path().join("sub.csv")).unwrap();
    let mut a = app();
    let r = a
        .import_folder(dir.path(), "Savings jar", "savings")
        .unwrap();
    assert_eq!(r.found, 2);
    assert_eq!(r.imported.len(), 2);
    assert!(r.failed.is_empty());
    let names: Vec<_> = a.accounts().unwrap().into_iter().map(|x| x.name).collect();
    assert!(names.contains(&"First Bank 5678".to_string()));
    assert!(names.contains(&"Savings jar".to_string()));
    // Again: nothing new, nothing double-counted.
    let again = a
        .import_folder(dir.path(), "Savings jar", "savings")
        .unwrap();
    assert_eq!(again.already_imported, 2);
    assert!(again.imported.is_empty());
    // A new file shows up next time.
    fs::write(
        dir.path().join("c.csv"),
        "Date,Description,Amount\n11/01/2026,X,-5.00\n",
    )
    .unwrap();
    let third = a
        .import_folder(dir.path(), "Savings jar", "savings")
        .unwrap();
    assert_eq!((third.already_imported, third.imported.len()), (2, 1));
}

#[test]
fn a_file_imported_by_hand_is_not_imported_again() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("b.csv"), CSV).unwrap();
    let mut a = app();
    a.import_file("b.csv", CSV.as_bytes(), "Jar", "checking")
        .unwrap();
    assert!(a.file_already_imported(CSV.as_bytes()));
    let r = a.import_folder(dir.path(), "Jar", "checking").unwrap();
    assert_eq!(r.already_imported, 1);
    assert!(r.imported.is_empty());
}

#[test]
fn csv_without_an_account_is_reported_not_guessed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("b.csv"), CSV).unwrap();
    let mut a = app();
    let r = a.import_folder(dir.path(), "  ", "checking").unwrap();
    assert_eq!(r.failed.len(), 1);
    assert_eq!(r.failed[0].name, "b.csv");
    assert!(r.failed[0].error.contains("account"));
    assert!(a.accounts().unwrap().is_empty());
}

#[test]
fn a_bad_file_does_not_stop_the_others() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.csv"), "not,a,statement\n").unwrap();
    fs::write(dir.path().join("b.ofx"), OFX).unwrap();
    let mut a = app();
    let r = a.import_folder(dir.path(), "Jar", "checking").unwrap();
    assert_eq!(r.failed.len(), 1);
    assert_eq!(r.imported.len(), 1);
}

#[test]
fn a_missing_folder_is_an_error() {
    let mut a = app();
    assert!(a
        .import_folder(
            std::path::Path::new("/definitely/not/here"),
            "A",
            "checking"
        )
        .is_err());
}
