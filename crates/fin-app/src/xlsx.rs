//! A small Excel workbook writer: plain sheets of text and numbers, no dependencies.
//!
//! The file is a zip of a few XML parts, each stored without compression, which every
//! spreadsheet app opens. Text is written as inline strings, so a payee like "=cmd()" is never
//! read as a formula.

/// One cell.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Text(String),
    Number(f64),
}

/// A named sheet: the first row is the header.
pub struct Sheet {
    pub name: String,
    pub rows: Vec<Vec<Cell>>,
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            // XML 1.0 has no place for most control characters.
            '\t' | '\n' | '\r' => out.push(ch),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

/// "A", "B", ..., "Z", "AA", ...
fn column(mut i: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'A' + (i % 26) as u8);
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

/// Sheet names: at most 31 characters, none of `[]:*?/\`, unique.
fn sheet_name(raw: &str, taken: &[String]) -> String {
    let clean: String = raw
        .chars()
        .filter(|c| !"[]:*?/\\".contains(*c))
        .take(31)
        .collect();
    let base = if clean.trim().is_empty() {
        "Sheet".to_string()
    } else {
        clean
    };
    let mut name = base.clone();
    let mut n = 2;
    while taken.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
        let suffix = format!(" ({n})");
        let keep: String = base.chars().take(31 - suffix.len()).collect();
        name = format!("{keep}{suffix}");
        n += 1;
    }
    name
}

fn sheet_xml(rows: &[Vec<Cell>]) -> String {
    let mut x = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews><sheetData>"#,
    );
    for (r, row) in rows.iter().enumerate() {
        x.push_str(&format!("<row r=\"{}\">", r + 1));
        for (c, cell) in row.iter().enumerate() {
            let at = format!("{}{}", column(c), r + 1);
            match cell {
                Cell::Text(t) => x.push_str(&format!(
                    "<c r=\"{at}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
                    esc(t)
                )),
                Cell::Number(n) if n.is_finite() => {
                    x.push_str(&format!("<c r=\"{at}\"><v>{n}</v></c>"))
                }
                Cell::Number(_) => {}
            }
        }
        x.push_str("</row>");
    }
    x.push_str("</sheetData></worksheet>");
    x
}

/// The workbook's bytes.
pub fn workbook(sheets: &[Sheet]) -> Vec<u8> {
    let mut names: Vec<String> = Vec::new();
    for s in sheets {
        let n = sheet_name(&s.name, &names);
        names.push(n);
    }
    if names.is_empty() {
        names.push("Sheet".into());
    }
    let mut types = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#,
    );
    let mut book = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>"#,
    );
    let mut rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let n = i + 1;
        types.push_str(&format!(
            "<Override PartName=\"/xl/worksheets/sheet{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"
        ));
        book.push_str(&format!(
            "<sheet name=\"{}\" sheetId=\"{n}\" r:id=\"rId{n}\"/>",
            esc(name)
        ));
        rels.push_str(&format!(
            "<Relationship Id=\"rId{n}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{n}.xml\"/>"
        ));
        let rows = sheets.get(i).map(|s| s.rows.as_slice()).unwrap_or(&[]);
        parts.push((
            format!("xl/worksheets/sheet{n}.xml"),
            sheet_xml(rows).into_bytes(),
        ));
    }
    types.push_str("</Types>");
    book.push_str("</sheets></workbook>");
    rels.push_str("</Relationships>");
    let root_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;
    let mut all = vec![
        ("[Content_Types].xml".to_string(), types.into_bytes()),
        ("_rels/.rels".to_string(), root_rels.as_bytes().to_vec()),
        ("xl/workbook.xml".to_string(), book.into_bytes()),
        ("xl/_rels/workbook.xml.rels".to_string(), rels.into_bytes()),
    ];
    all.extend(parts);
    zip_stored(&all)
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// A zip archive with every entry stored (no compression).
pub(crate) fn zip_stored(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    // 1980-01-01 00:00, the earliest zip time; spreadsheet apps ignore it.
    let (time, date) = (0u16, (1 << 5) | 1u16);
    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let size = data.len() as u32;
        let n = name.as_bytes();
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 names
        out.extend_from_slice(&0u16.to_le_bytes()); // stored
        out.extend_from_slice(&time.to_le_bytes());
        out.extend_from_slice(&date.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&(n.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(n);
        out.extend_from_slice(data);

        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes()); // made by
        central.extend_from_slice(&20u16.to_le_bytes()); // needed
        central.extend_from_slice(&0x0800u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&time.to_le_bytes());
        central.extend_from_slice(&date.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&(n.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0; 8]); // extra and comment lengths, disk, internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(n);
    }
    let start = out.len() as u32;
    let len = central.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]); // disk numbers
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn columns_run_past_z() {
        assert_eq!(column(0), "A");
        assert_eq!(column(25), "Z");
        assert_eq!(column(26), "AA");
        assert_eq!(column(27), "AB");
        assert_eq!(column(701), "ZZ");
        assert_eq!(column(702), "AAA");
    }

    #[test]
    fn sheet_names_are_cleaned_shortened_and_unique() {
        assert_eq!(
            sheet_name("By category: 2026/27", &[]),
            "By category 202627"
        );
        let long = "x".repeat(40);
        assert_eq!(sheet_name(&long, &[]).chars().count(), 31);
        let taken = vec!["Data".to_string()];
        assert_eq!(sheet_name("data", &taken), "data (2)");
        assert_eq!(sheet_name("", &[]), "Sheet");
    }

    #[test]
    fn text_is_escaped_and_never_a_formula() {
        let x = sheet_xml(&[vec![
            Cell::Text("=cmd()&<b>\u{1}".into()),
            Cell::Number(-12.5),
            Cell::Number(f64::NAN),
        ]]);
        assert!(x.contains("<t xml:space=\"preserve\">=cmd()&amp;&lt;b&gt;</t>"));
        assert!(x.contains("<c r=\"B1\"><v>-12.5</v></c>"));
        assert!(!x.contains("C1"));
        assert!(!x.contains("<f>"));
    }

    #[test]
    fn the_zip_lists_every_part_with_its_size_and_crc() {
        let book = workbook(&[Sheet {
            name: "T".into(),
            rows: vec![vec![Cell::Text("a".into())]],
        }]);
        assert_eq!(&book[..4], b"PK\x03\x04");
        // End of central directory: 5 entries.
        let end = book.len() - 22;
        assert_eq!(&book[end..end + 4], b"PK\x05\x06");
        assert_eq!(u16::from_le_bytes([book[end + 10], book[end + 11]]), 5);
        // Walk the local headers and check each stored entry's CRC.
        let mut at = 0;
        let mut names = Vec::new();
        while book[at..at + 4] == *b"PK\x03\x04" {
            let rd16 = |i: usize| u16::from_le_bytes([book[i], book[i + 1]]) as usize;
            let rd32 =
                |i: usize| u32::from_le_bytes([book[i], book[i + 1], book[i + 2], book[i + 3]]);
            let crc = rd32(at + 14);
            let size = rd32(at + 18) as usize;
            let nlen = rd16(at + 26);
            let name = String::from_utf8(book[at + 30..at + 30 + nlen].to_vec()).unwrap();
            let data = &book[at + 30 + nlen..at + 30 + nlen + size];
            assert_eq!(crc32(data), crc, "{name}");
            names.push(name);
            at += 30 + nlen + size;
        }
        // The central directory starts where the end record says and its entries, walked by
        // their own lengths, end exactly at the end record.
        let rd32 = |i: usize| u32::from_le_bytes([book[i], book[i + 1], book[i + 2], book[i + 3]]);
        let rd16 = |i: usize| u16::from_le_bytes([book[i], book[i + 1]]) as usize;
        let mut c = rd32(end + 16) as usize;
        assert_eq!(c, at);
        for name in &names {
            assert_eq!(&book[c..c + 4], b"PK\x01\x02");
            let nlen = rd16(c + 28);
            let extra = rd16(c + 30);
            let comment = rd16(c + 32);
            assert_eq!(&book[c + 46..c + 46 + nlen], name.as_bytes());
            c += 46 + nlen + extra + comment;
        }
        assert_eq!(c, end);
        assert_eq!(
            names,
            [
                "[Content_Types].xml",
                "_rels/.rels",
                "xl/workbook.xml",
                "xl/_rels/workbook.xml.rels",
                "xl/worksheets/sheet1.xml"
            ]
        );
    }
}
