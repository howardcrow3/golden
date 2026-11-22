//! Recognise common tax and financial documents from their text and pull out the figures that
//! matter. Everything is label matching on text: nothing is sent anywhere and no model is
//! needed, so a scan without a text layer (an image) is stored but not recognised.
use fin_core::Money;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    W2,
    Form1099Int,
    Form1099Div,
    Form1099B,
    Form1099R,
    Form1099Nec,
    SsaStatement1099,
    Form1098,
    Form1040,
    Paystub,
    BankStatement,
    BrokerageStatement,
    Other,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::W2 => "W-2",
            Kind::Form1099Int => "1099-INT",
            Kind::Form1099Div => "1099-DIV",
            Kind::Form1099B => "1099-B",
            Kind::Form1099R => "1099-R",
            Kind::Form1099Nec => "1099-NEC",
            Kind::SsaStatement1099 => "SSA-1099",
            Kind::Form1098 => "1098",
            Kind::Form1040 => "1040",
            Kind::Paystub => "Paystub",
            Kind::BankStatement => "Bank statement",
            Kind::BrokerageStatement => "Brokerage statement",
            Kind::Other => "Document",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub label: &'static str,
    pub amount: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recognised {
    pub kind: Kind,
    /// Tax year, when the document states one.
    pub year: Option<i32>,
    pub fields: Vec<Field>,
}

fn has(t: &str, needle: &str) -> bool {
    t.contains(needle)
}

pub fn classify(text: &str) -> Kind {
    let t = text.to_lowercase();
    // Specific forms first; a 1040 mentions most of the others.
    if has(&t, "form 1040") && has(&t, "filing status") {
        return Kind::Form1040;
    }
    if has(&t, "wage and tax statement") || has(&t, "form w-2") {
        return Kind::W2;
    }
    if has(&t, "ssa-1099") || has(&t, "social security benefit statement") {
        return Kind::SsaStatement1099;
    }
    if has(&t, "1099-int") {
        return Kind::Form1099Int;
    }
    if has(&t, "1099-div") {
        return Kind::Form1099Div;
    }
    if has(&t, "1099-b") || has(&t, "proceeds from broker") {
        return Kind::Form1099B;
    }
    if has(&t, "1099-r") || has(&t, "distributions from pensions") {
        return Kind::Form1099R;
    }
    if has(&t, "1099-nec") || has(&t, "nonemployee compensation") {
        return Kind::Form1099Nec;
    }
    if has(&t, "form 1098") || has(&t, "mortgage interest statement") {
        return Kind::Form1098;
    }
    if has(&t, "net pay") && (has(&t, "gross pay") || has(&t, "earnings")) {
        return Kind::Paystub;
    }
    if has(&t, "statement period") || has(&t, "beginning balance") {
        if has(&t, "holdings") || has(&t, "securities") || has(&t, "portfolio") {
            return Kind::BrokerageStatement;
        }
        return Kind::BankStatement;
    }
    Kind::Other
}

/// Amounts on a line, as cents: 1,234.56 or $1,234.56.
fn amounts(s: &str) -> Vec<Money> {
    s.split_whitespace()
        .filter_map(|tok| {
            let t = tok
                .trim_matches(|c: char| matches!(c, '$' | '(' | ')' | ','))
                .replace(',', "");
            (t.contains('.') && t.chars().all(|c| c.is_ascii_digit() || c == '.'))
                .then(|| Money::parse(&t).ok())
                .flatten()
        })
        .collect()
}

/// The first amount after a label, on its line or the next one.
fn after(text: &str, labels: &[&str]) -> Option<Money> {
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let l = line.to_lowercase();
        for label in labels {
            if let Some(pos) = l.find(label) {
                let rest = &line[(pos + label.len()).min(line.len())..];
                if let Some(a) = amounts(rest).first() {
                    return Some(*a);
                }
                if let Some(a) = lines.get(i + 1).and_then(|n| amounts(n).first().copied()) {
                    return Some(a);
                }
            }
        }
    }
    None
}

fn year(text: &str) -> Option<i32> {
    let t = text.to_lowercase();
    for key in ["tax year", "for calendar year", "form w-2", "form 1099"] {
        if let Some(p) = t.find(key) {
            let tail = &t[p + key.len()..];
            let digits: String = tail
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take(4)
                .collect();
            if let Ok(y) = digits.parse::<i32>() {
                if (1990..=2100).contains(&y) {
                    return Some(y);
                }
            }
        }
    }
    None
}

pub fn recognise(text: &str) -> Recognised {
    let kind = classify(text);
    let wanted: &[(&'static str, &[&str])] = match kind {
        Kind::W2 => &[
            ("Wages", &["wages, tips, other compensation"]),
            ("Federal tax withheld", &["federal income tax withheld"]),
            ("Social Security wages", &["social security wages"]),
            ("State tax withheld", &["state income tax"]),
        ],
        Kind::Form1099Int => &[
            ("Interest income", &["interest income"]),
            ("Federal tax withheld", &["federal income tax withheld"]),
        ],
        Kind::Form1099Div => &[
            (
                "Ordinary dividends",
                &["total ordinary dividends", "ordinary dividends"],
            ),
            ("Qualified dividends", &["qualified dividends"]),
            ("Capital gain distributions", &["total capital gain distr"]),
        ],
        Kind::Form1099B => &[
            ("Proceeds", &["proceeds"]),
            ("Cost basis", &["cost or other basis", "cost basis"]),
        ],
        Kind::Form1099R => &[
            ("Gross distribution", &["gross distribution"]),
            ("Taxable amount", &["taxable amount"]),
            ("Federal tax withheld", &["federal income tax withheld"]),
        ],
        Kind::Form1099Nec => &[("Nonemployee compensation", &["nonemployee compensation"])],
        Kind::SsaStatement1099 => &[
            ("Net benefits", &["net benefits"]),
            ("Benefits paid", &["total benefits paid", "benefits paid"]),
        ],
        Kind::Form1098 => &[("Mortgage interest", &["mortgage interest received"])],
        Kind::Form1040 => &[
            ("Total income", &["total income"]),
            ("Adjusted gross income", &["adjusted gross income"]),
            ("Taxable income", &["taxable income"]),
            ("Total tax", &["total tax"]),
        ],
        Kind::Paystub => &[("Gross pay", &["gross pay"]), ("Net pay", &["net pay"])],
        Kind::BankStatement | Kind::BrokerageStatement => &[
            ("Beginning balance", &["beginning balance"]),
            ("Ending balance", &["ending balance", "closing balance"]),
        ],
        Kind::Other => &[],
    };
    let fields = wanted
        .iter()
        .filter_map(|(label, keys)| after(text, keys).map(|amount| Field { label, amount }))
        .collect();
    Recognised {
        kind,
        year: year(text),
        fields,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W2: &str = "Form W-2 Wage and Tax Statement 2025\nWages, tips, other compensation 85,000.00\nFederal income tax withheld\n12,345.67\nSocial Security wages 85,000.00\n";

    #[test]
    fn recognises_a_w2_and_reads_its_boxes() {
        let r = recognise(W2);
        assert_eq!(r.kind, Kind::W2);
        assert_eq!(r.year, Some(2025));
        let get = |l: &str| {
            r.fields
                .iter()
                .find(|f| f.label == l)
                .map(|f| f.amount.cents())
        };
        assert_eq!(get("Wages"), Some(8_500_000));
        // The amount can sit on the line after its label.
        assert_eq!(get("Federal tax withheld"), Some(1_234_567));
        assert_eq!(get("State tax withheld"), None);
    }

    #[test]
    fn tells_the_forms_apart() {
        assert_eq!(
            classify("Form 1099-INT Interest Income 1,000.00"),
            Kind::Form1099Int
        );
        assert_eq!(classify("1099-DIV Dividends"), Kind::Form1099Div);
        assert_eq!(
            classify("Form 1099-R Distributions from Pensions"),
            Kind::Form1099R
        );
        assert_eq!(
            classify("SSA-1099 Social Security Benefit Statement"),
            Kind::SsaStatement1099
        );
        assert_eq!(
            classify("Form 1098 Mortgage Interest Statement"),
            Kind::Form1098
        );
        assert_eq!(
            classify("Gross Pay 3,000.00 Net Pay 2,300.00"),
            Kind::Paystub
        );
        assert_eq!(
            classify("Statement period Jan 1 Beginning balance 10.00"),
            Kind::BankStatement
        );
        assert_eq!(
            classify("Statement period Holdings Beginning balance"),
            Kind::BrokerageStatement
        );
        assert_eq!(classify("a recipe for soup"), Kind::Other);
    }

    #[test]
    fn a_1040_is_not_mistaken_for_the_forms_it_mentions() {
        let t = "Form 1040 U.S. Individual Income Tax Return Filing Status\nIncluding W-2 and 1099-INT\nAdjusted gross income 120,000.00\nTotal tax 15,000.00";
        let r = recognise(t);
        assert_eq!(r.kind, Kind::Form1040);
        assert_eq!(
            r.fields
                .iter()
                .find(|f| f.label == "Total tax")
                .unwrap()
                .amount
                .cents(),
            1_500_000
        );
    }

    #[test]
    fn unknown_text_yields_no_fields_and_labels_are_readable() {
        let r = recognise("nothing to see");
        assert_eq!(r.kind, Kind::Other);
        assert!(r.fields.is_empty());
        assert_eq!(r.year, None);
        assert_eq!(Kind::Form1099Nec.label(), "1099-NEC");
    }
}
