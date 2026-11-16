//! Balance sheet: what you have, what you owe, and what is left (net worth).
//!
//! Balances are the net of imported activity (there are no opening balances yet), so a credit
//! card with more spending than payments shows as a negative balance, which is a debt.
use crate::App;
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SheetLine {
    pub name: String,
    pub kind: String,
    /// Positive for what you have, positive for what you owe on the debts side.
    pub cents: i64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BalanceSheet {
    pub assets: Vec<SheetLine>,
    pub debts: Vec<SheetLine>,
    pub assets_cents: i64,
    pub debts_cents: i64,
    pub net_worth_cents: i64,
}

/// Account kinds that are money owed. Anything else counts as something you hold.
pub fn is_debt_kind(kind: &str) -> bool {
    let k = kind.to_ascii_lowercase();
    [
        "credit",
        "card",
        "loan",
        "mortgage",
        "line of credit",
        "heloc",
        "debt",
    ]
    .iter()
    .any(|w| k.contains(w))
}

impl App {
    pub fn balance_sheet(&self) -> Result<BalanceSheet, String> {
        let mut assets = vec![];
        let mut debts = vec![];
        for a in self.accounts()? {
            if is_debt_kind(&a.kind) {
                // A debt account that is in credit (overpaid) is an asset, so it nets correctly.
                if a.balance_cents <= 0 {
                    debts.push(SheetLine {
                        name: a.name,
                        kind: a.kind,
                        cents: -a.balance_cents,
                    });
                } else {
                    assets.push(SheetLine {
                        name: a.name,
                        kind: a.kind,
                        cents: a.balance_cents,
                    });
                }
            } else if a.balance_cents >= 0 {
                assets.push(SheetLine {
                    name: a.name,
                    kind: a.kind,
                    cents: a.balance_cents,
                });
            } else {
                // Overdrawn cash account is money owed to the bank.
                debts.push(SheetLine {
                    name: a.name,
                    kind: a.kind,
                    cents: -a.balance_cents,
                });
            }
        }
        assets.sort_by(|a, b| b.cents.cmp(&a.cents).then(a.name.cmp(&b.name)));
        debts.sort_by(|a, b| b.cents.cmp(&a.cents).then(a.name.cmp(&b.name)));
        let assets_cents: i64 = assets.iter().map(|l| l.cents).sum();
        let debts_cents: i64 = debts.iter().map(|l| l.cents).sum();
        Ok(BalanceSheet {
            assets,
            debts,
            assets_cents,
            debts_cents,
            net_worth_cents: assets_cents - debts_cents,
        })
    }
}
