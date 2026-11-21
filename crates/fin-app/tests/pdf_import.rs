use fin_app::App;
use fin_core::NaiveDate;

/// A one-page PDF with each line of `lines` set in Helvetica, built by hand.
fn make_pdf(lines: &[&str]) -> Vec<u8> {
    let mut content = String::from("BT /F1 10 Tf 12 TL 40 760 Td\n");
    for l in lines {
        let esc = l
            .replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)");
        content.push_str(&format!("({esc}) Tj T*\n"));
    }
    content.push_str("ET");
    let objs = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![];
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n{}\nendobj\n", i + 1, o).as_bytes());
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objs.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn app() -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap())
}

#[test]
fn a_pdf_statement_imports_and_reconciles_against_its_ending_balance() {
    let pdf = make_pdf(&[
        "First Bank Checking Statement October 2026",
        "Beginning balance $0.00",
        "10/01/2026 PAYROLL ACME 5,000.00 5,000.00",
        "10/06/2026 CAFE LUNCH 18.40 4,981.60",
        "Ending balance $4,981.60",
    ]);
    let mut a = app();
    let r = a.import_statement(&pdf, "Checking", "checking").unwrap();
    assert_eq!(r.added, 2);
    assert_eq!(r.statement_balance_cents, Some(498_160));
    assert_eq!(r.reconciled, Some(true));
    // Importing it again adds nothing.
    let again = a.import_statement(&pdf, "Checking", "checking").unwrap();
    assert_eq!(again.added, 0);
}

#[test]
fn a_scanned_pdf_says_why_it_cannot_be_read() {
    let mut a = app();
    let e = a
        .import_statement(&make_pdf(&[]), "Checking", "checking")
        .unwrap_err();
    assert!(e.contains("scanned"), "{e}");
}
