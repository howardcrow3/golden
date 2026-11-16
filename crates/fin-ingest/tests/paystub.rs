use fin_core::{Money, NaiveDate};
use fin_ingest::paystub::{parse, PaystubError};

const STUB: &str = "Employer: Acme Corp
Pay Period Beginning: 10/01/2026
Pay Period Ending: 10/15/2026
Pay Date: 10/20/2026
Earnings            Current      YTD
Regular             4,000.00   80,000.00
Gross Pay           4,000.00   80,000.00
Federal Income Tax    420.00    8,400.00
State Income Tax      180.00    3,600.00
Social Security       248.00    4,960.00
Medicare               58.00    1,160.00
401(k)                320.00    6,400.00
Health Insurance      110.00    2,200.00
Net Pay             2,664.00   53,280.00
";

fn m(c: i64) -> Money {
    Money::from_cents(c)
}

#[test]
fn reads_every_field_from_a_typical_stub() {
    let s = parse(STUB).unwrap();
    assert_eq!(s.employer, "Acme Corp");
    assert_eq!(s.pay_date, NaiveDate::from_ymd_opt(2026, 10, 20));
    assert_eq!(s.period_start, NaiveDate::from_ymd_opt(2026, 10, 1));
    assert_eq!(s.period_end, NaiveDate::from_ymd_opt(2026, 10, 15));
    assert_eq!(s.gross, m(400_000));
    assert_eq!(s.federal_tax, m(42_000));
    assert_eq!(s.state_tax, m(18_000));
    assert_eq!(s.social_security, m(24_800));
    assert_eq!(s.medicare, m(5_800));
    assert_eq!(s.retirement, m(32_000));
    assert_eq!(s.benefits, m(11_000));
    assert_eq!(s.net, m(266_400));
}

#[test]
fn current_column_wins_over_year_to_date() {
    let s = parse(STUB).unwrap();
    assert_ne!(s.gross, m(8_000_000));
}

#[test]
fn dollar_signs_and_other_labels_work() {
    let s =
        parse("Total Gross $1,000.00\nFederal Withholding $100.00\nTake Home $900.00\n").unwrap();
    assert_eq!(s.gross, m(100_000));
    assert_eq!(s.federal_tax, m(10_000));
    assert_eq!(s.net, m(90_000));
}

#[test]
fn missing_gross_or_net_is_refused() {
    assert_eq!(parse("Net Pay 100.00"), Err(PaystubError::NoGross));
    assert_eq!(parse("Gross Pay 100.00"), Err(PaystubError::NoNet));
    assert_eq!(parse(""), Err(PaystubError::NoGross));
}

#[test]
fn numbers_that_cannot_be_right_are_refused() {
    assert_eq!(
        parse("Gross Pay 1,000.00\nFederal Income Tax 900.00\nNet Pay 800.00\n"),
        Err(PaystubError::DoesNotReconcile)
    );
    assert_eq!(
        parse("Gross Pay 100.00\nNet Pay 200.00\n"),
        Err(PaystubError::DoesNotReconcile)
    );
}

#[test]
fn unlisted_deductions_are_tolerated() {
    // Only the net is lower than gross: after-tax items are not itemised.
    let s = parse("Gross Pay 1,000.00\nNet Pay 700.00\n").unwrap();
    assert_eq!(s.net, m(70_000));
}
