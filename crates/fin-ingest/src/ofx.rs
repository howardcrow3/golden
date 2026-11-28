use crate::dates::parse_date;
use crate::{IngestError, ParsedTxn};
use fin_core::Money;

/// Value of the first `<TAG>value` after `from` inside `block` (OFX 1.x has no closing tags).
fn tag<'a>(block: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let start = block.to_ascii_uppercase().find(&open)? + open.len();
    let rest = &block[start..];
    let end = rest.find('<').unwrap_or(rest.len());
    let v = rest[..end].trim();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

/// Read an OFX amount. The spec allows a comma for the decimal point and has no thousands
/// separators, so "-20,50" is twenty dollars fifty, not two thousand and fifty.
fn amount(raw: &str) -> Result<Money, fin_core::MoneyError> {
    if raw.contains(',') && !raw.contains('.') {
        Money::parse(&raw.replace(',', "."))
    } else {
        Money::parse(raw)
    }
}

/// Parse OFX/QFX (SGML 1.x or XML 2.x) statement transactions.
pub fn parse(text: &str) -> Result<Vec<ParsedTxn>, IngestError> {
    let upper = text.to_ascii_uppercase();
    if !upper.contains("<OFX>") {
        return Err(IngestError::NotOfx);
    }
    let mut out = vec![];
    let mut pos = 0;
    while let Some(i) = upper[pos..].find("<STMTTRN>") {
        let s = pos + i + "<STMTTRN>".len();
        let e = upper[s..]
            .find("</STMTTRN>")
            .map(|x| s + x)
            .or_else(|| upper[s..].find("<STMTTRN>").map(|x| s + x))
            .unwrap_or(text.len());
        let block = &text[s..e];
        let row = out.len() + 1;
        let date_raw = tag(block, "DTPOSTED").ok_or_else(|| IngestError::Row {
            row,
            msg: "missing DTPOSTED".into(),
        })?;
        let day =
            parse_date(&date_raw[..date_raw.len().min(8)]).ok_or_else(|| IngestError::Row {
                row,
                msg: format!("bad date '{date_raw}'"),
            })?;
        let amt = tag(block, "TRNAMT").ok_or_else(|| IngestError::Row {
            row,
            msg: "missing TRNAMT".into(),
        })?;
        let amount = amount(amt).map_err(|e| IngestError::Row {
            row,
            msg: e.to_string(),
        })?;
        let payee = tag(block, "NAME")
            .or_else(|| tag(block, "PAYEE"))
            .unwrap_or("")
            .to_string();
        let memo = tag(block, "MEMO").unwrap_or("").to_string();
        out.push(ParsedTxn {
            day,
            amount,
            payee,
            memo,
        });
        pos = e;
    }
    Ok(out)
}

/// The closing (ledger) balance an OFX statement prints, with the date it applies to.
/// None when the file has no `<LEDGERBAL>` block or its amount is unreadable.
pub fn ledger_balance(text: &str) -> Option<(fin_core::NaiveDate, Money)> {
    let upper = text.to_ascii_uppercase();
    let s = upper.find("<LEDGERBAL>")? + "<LEDGERBAL>".len();
    let e = upper[s..]
        .find("</LEDGERBAL>")
        .map(|x| s + x)
        .unwrap_or(text.len());
    let block = &text[s..e];
    let amount = amount(tag(block, "BALAMT")?).ok()?;
    let raw = tag(block, "DTASOF")?;
    let day = parse_date(&raw[..raw.len().min(8)])?;
    Some((day, amount))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ledger_balance() {
        let x = "<OFX><LEDGERBAL><BALAMT>1234.56<DTASOF>20261031120000</LEDGERBAL></OFX>";
        let (d, m) = ledger_balance(x).unwrap();
        assert_eq!(d.to_string(), "2026-10-31");
        assert_eq!(m, Money::from_cents(123_456));
        let x =
            "<OFX><LEDGERBAL><BALAMT>-50.00</BALAMT><DTASOF>20261031</DTASOF></LEDGERBAL></OFX>";
        assert_eq!(ledger_balance(x).unwrap().1, Money::from_cents(-5000));
    }

    #[test]
    fn missing_or_bad_ledger_balance_is_none() {
        assert!(ledger_balance("<OFX></OFX>").is_none());
        assert!(
            ledger_balance("<OFX><LEDGERBAL><BALAMT>x<DTASOF>20261031</LEDGERBAL></OFX>").is_none()
        );
        assert!(ledger_balance("<OFX><LEDGERBAL><BALAMT>5.00</LEDGERBAL></OFX>").is_none());
    }

    const SGML: &str = "OFXHEADER:100\nDATA:OFXSGML\n\n<OFX><BANKMSGSRSV1><STMTTRNRS><STMTRS><BANKTRANLIST>\n<STMTTRN>\n<TRNTYPE>DEBIT\n<DTPOSTED>20261003120000[-5:EST]\n<TRNAMT>-1500.00\n<FITID>1\n<NAME>LANDLORD\n<MEMO>Rent Nov\n</STMTTRN>\n<STMTTRN>\n<TRNTYPE>CREDIT\n<DTPOSTED>20261001\n<TRNAMT>5000.00\n<FITID>2\n<NAME>ACME PAYROLL\n</STMTTRN>\n</BANKTRANLIST></STMTRS></STMTTRNRS></BANKMSGSRSV1></OFX>";

    #[test]
    fn parses_sgml_ofx() {
        let t = parse(SGML).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].amount, Money::from_dollars(-1500));
        assert_eq!(t[0].payee, "LANDLORD");
        assert_eq!(t[0].memo, "Rent Nov");
        assert_eq!(t[1].day.to_string(), "2026-10-01");
    }

    #[test]
    fn parses_xml_ofx() {
        let x = "<OFX><STMTTRN><DTPOSTED>20261015</DTPOSTED><TRNAMT>-20.50</TRNAMT><NAME>Cafe</NAME></STMTTRN></OFX>";
        let t = parse(x).unwrap();
        assert_eq!(t[0].amount, Money::from_cents(-2050));
    }

    #[test]
    fn a_decimal_comma_is_the_cents() {
        let x = "<OFX><STMTTRN><DTPOSTED>20261015<TRNAMT>-20,50<NAME>Cafe</STMTTRN>\
            <LEDGERBAL><BALAMT>1234,56<DTASOF>20261031</LEDGERBAL></OFX>";
        assert_eq!(parse(x).unwrap()[0].amount, Money::from_cents(-2050));
        assert_eq!(ledger_balance(x).unwrap().1, Money::from_cents(123_456));
    }

    #[test]
    fn rejects_non_ofx_and_bad_rows() {
        assert_eq!(parse("hello"), Err(IngestError::NotOfx));
        assert!(matches!(
            parse("<OFX><STMTTRN><TRNAMT>1</STMTTRN></OFX>"),
            Err(IngestError::Row { .. })
        ));
    }
}
