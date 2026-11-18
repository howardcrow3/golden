use crate::dates::parse_date;
use crate::{IngestError, ParsedTxn};
use fin_core::Money;

struct Cols {
    date: usize,
    desc: Option<usize>,
    memo: Option<usize>,
    amount: Option<usize>,
    debit: Option<usize>,
    credit: Option<usize>,
    kind: Option<usize>,
}

fn find(headers: &[String], names: &[&str]) -> Option<usize> {
    headers.iter().position(|h| names.iter().any(|n| h == n))
}

fn columns(headers: &[String]) -> Result<Cols, IngestError> {
    let h: Vec<String> = headers
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .collect();
    let date = find(
        &h,
        &[
            "date",
            "posting date",
            "post date",
            "posted date",
            "transaction date",
            "trans date",
            "trans. date",
        ],
    )
    .ok_or(IngestError::NoDateColumn)?;
    let amount = find(&h, &["amount", "transaction amount", "amt"]);
    let debit = find(
        &h,
        &["debit", "withdrawal", "withdrawals", "money out", "charges"],
    );
    let credit = find(
        &h,
        &["credit", "deposit", "deposits", "money in", "payments"],
    );
    if amount.is_none() && debit.is_none() && credit.is_none() {
        return Err(IngestError::NoAmountColumn);
    }
    Ok(Cols {
        date,
        desc: find(
            &h,
            &[
                "description",
                "payee",
                "name",
                "merchant",
                "details",
                "original description",
            ],
        ),
        memo: find(&h, &["memo", "notes", "note", "reference"]),
        amount,
        debit,
        credit,
        kind: find(&h, &["type", "transaction type", "dr/cr"]),
    })
}

/// Comma unless the header line clearly uses semicolons or tabs (common in European exports).
fn sniff_delimiter(header: &str) -> u8 {
    let count = |c: char| header.matches(c).count();
    let (commas, semis, tabs) = (count(','), count(';'), count('\t'));
    if semis > commas && semis >= tabs {
        b';'
    } else if tabs > commas {
        b'\t'
    } else {
        b','
    }
}

/// Make an amount cell readable by `Money::parse`: a trailing minus ("5.00-") becomes a leading
/// one, and with semicolon files a decimal comma ("1.234,56") becomes a decimal point.
fn normalize_amount(raw: &str, decimal_comma: bool) -> String {
    let mut s = raw.trim().to_string();
    if s.len() > 1 && s.ends_with('-') {
        s.pop();
        s = format!("-{}", s.trim());
    }
    if decimal_comma && s.contains(',') {
        s = s.replace('.', "").replace(',', ".");
    }
    s
}

/// Parse a bank CSV with a header row. Handles single signed Amount columns, split
/// Debit/Credit columns, a leading preamble before the header, and quoted fields.
pub fn parse(text: &str) -> Result<Vec<ParsedTxn>, IngestError> {
    if text.trim().is_empty() {
        return Err(IngestError::Empty);
    }
    // Skip any preamble lines until one looks like a header (contains "date" and has a comma).
    let mut start = 0usize;
    for (i, line) in text.lines().enumerate() {
        let l = line.to_ascii_lowercase();
        if l.contains("date") && (l.contains(',') || l.contains(';') || l.contains('\t')) {
            start = text.lines().take(i).map(|x| x.len() + 1).sum();
            break;
        }
    }
    let body = &text[start.min(text.len())..];
    let delimiter = sniff_delimiter(body.lines().next().unwrap_or(""));
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(body.as_bytes());
    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| IngestError::Csv(e.to_string()))?
        .iter()
        .map(String::from)
        .collect();
    let cols = columns(&headers)?;
    let mut out = vec![];
    for (i, rec) in rdr.records().enumerate() {
        let rec = rec.map_err(|e| IngestError::Csv(e.to_string()))?;
        let row = i + 2;
        let get = |c: Option<usize>| c.and_then(|c| rec.get(c)).unwrap_or("").to_string();
        let date_s = rec.get(cols.date).unwrap_or("");
        if date_s.trim().is_empty() {
            continue; // blank or footer line
        }
        let day = parse_date(date_s).ok_or_else(|| IngestError::Row {
            row,
            msg: format!("bad date '{date_s}'"),
        })?;
        let money = |s: &str| {
            Money::parse(&normalize_amount(s, delimiter == b';')).map_err(|e| IngestError::Row {
                row,
                msg: e.to_string(),
            })
        };
        let amount = if let Some(a) = cols.amount {
            let mut m = money(rec.get(a).unwrap_or(""))?;
            // Some exports mark direction in a Type column instead of the sign.
            if let Some(k) = cols.kind {
                let kind = rec.get(k).unwrap_or("").to_ascii_lowercase();
                if (kind == "debit" || kind == "dr") && m > Money::ZERO {
                    m = -m;
                }
            }
            m
        } else {
            let debit = rec
                .get(cols.debit.unwrap_or(usize::MAX))
                .filter(|s| !s.is_empty())
                .map(money)
                .transpose()?;
            let credit = rec
                .get(cols.credit.unwrap_or(usize::MAX))
                .filter(|s| !s.is_empty())
                .map(money)
                .transpose()?;
            credit.unwrap_or(Money::ZERO) - debit.map(|d| d.abs()).unwrap_or(Money::ZERO)
        };
        out.push(ParsedTxn {
            day,
            amount,
            payee: get(cols.desc),
            memo: get(cols.memo),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semicolon_files_with_decimal_commas() {
        let t =
            parse("Date;Description;Amount\n10/01/2026;Rent;-1.234,56\n10/02/2026;Pay;2500,00\n")
                .unwrap();
        assert_eq!(
            (t[0].amount.cents(), t[1].amount.cents()),
            (-123_456, 250_000)
        );
    }

    #[test]
    fn tab_separated_files() {
        let t = parse("Date\tDescription\tAmount\n10/01/2026\tCoffee, large\t-4.50\n").unwrap();
        assert_eq!(
            (t[0].payee.as_str(), t[0].amount.cents()),
            ("Coffee, large", -450)
        );
    }

    #[test]
    fn trailing_minus_means_money_out() {
        let t = parse("Date,Description,Amount\n10/01/2026,Fee,12.00-\n10/02/2026,Pay,12.00\n")
            .unwrap();
        assert_eq!((t[0].amount.cents(), t[1].amount.cents()), (-1200, 1200));
    }

    #[test]
    fn commas_inside_a_comma_file_still_mean_thousands() {
        let t = parse("Date,Description,Amount\n10/01/2026,Pay,\"1,234.56\"\n").unwrap();
        assert_eq!(t[0].amount.cents(), 123_456);
    }

    #[test]
    fn signed_amount_column() {
        let t = parse("Date,Description,Amount\n10/01/2026,ACME PAYROLL,\"5,000.00\"\n10/03/2026,LANDLORD,-1500.00\n").unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].amount, Money::from_dollars(5000));
        assert_eq!(t[1].amount, Money::from_dollars(-1500));
        assert_eq!(t[1].payee, "LANDLORD");
    }

    #[test]
    fn debit_credit_columns() {
        let t = parse("Posting Date,Details,Debit,Credit\n2026-10-01,Paycheck,,2500.00\n2026-10-02,Grocer,63.20,\n").unwrap();
        assert_eq!(t[0].amount, Money::from_cents(250_000));
        assert_eq!(t[1].amount, Money::from_cents(-6_320));
    }

    #[test]
    fn type_column_flips_sign() {
        let t = parse("Date,Description,Amount,Type\n10/01/2026,Store,20.00,DEBIT\n10/02/2026,Refund,5.00,CREDIT\n").unwrap();
        assert_eq!(t[0].amount, Money::from_dollars(-20));
        assert_eq!(t[1].amount, Money::from_dollars(5));
    }

    #[test]
    fn preamble_and_blank_footer_are_skipped() {
        let t = parse("Account: Checking 4821\nGenerated 10/31/2026\nDate,Description,Amount\n10/01/2026,A,1.00\n,,\n").unwrap();
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn quoted_commas_in_descriptions() {
        let t = parse("Date,Description,Amount\n10/01/2026,\"SMITH, JOHN\",-3.00\n").unwrap();
        assert_eq!(t[0].payee, "SMITH, JOHN");
    }

    #[test]
    fn errors_are_specific() {
        assert_eq!(
            parse("Description,Amount\nx,1\n"),
            Err(IngestError::NoDateColumn)
        );
        assert_eq!(
            parse("Date,Description\n10/01/2026,x\n"),
            Err(IngestError::NoAmountColumn)
        );
        assert!(matches!(
            parse("Date,Amount\nnope,1\n"),
            Err(IngestError::Row { row: 2, .. })
        ));
        assert!(matches!(
            parse("Date,Amount\n10/01/2026,abc\n"),
            Err(IngestError::Row { row: 2, .. })
        ));
        assert_eq!(parse("  \n"), Err(IngestError::Empty));
    }
}
