use fin_ingest::{read_statement, statement_balance};

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

const LINES: [&str; 7] = [
    "First Bank Checking Statement October 2026",
    "Beginning balance $1,000.00",
    "10/01/2026 PAYROLL ACME 5,000.00 6,000.00",
    "10/06/2026 CAFE LUNCH 18.40 5,981.60",
    "10/09/2026 RENT 1,500.00 4,481.60",
    "Ending balance $4,481.60",
    "Thank you for banking with us, this statement covers October",
];

#[test]
fn a_text_pdf_is_read_like_any_other_statement() {
    let pdf = make_pdf(&LINES);
    let t = read_statement(&pdf).unwrap();
    let amounts: Vec<i64> = t.iter().map(|t| t.amount.cents()).collect();
    assert_eq!(amounts, vec![500_000, -1_840, -150_000]);
    assert_eq!(t[1].payee, "CAFE LUNCH");
    let (day, bal) = statement_balance(&pdf).unwrap();
    assert_eq!(bal.cents(), 448_160);
    assert_eq!(day.to_string(), "2026-10-09");
}

#[test]
fn a_pdf_with_no_text_is_explained() {
    let pdf = make_pdf(&[]);
    let e = read_statement(&pdf).unwrap_err().to_string();
    assert!(e.contains("scanned") || e.contains("OCR"), "{e}");
}
