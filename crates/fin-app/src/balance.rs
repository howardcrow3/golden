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

/// Things you own that no bank statement will show, entered by hand and kept current by you.
pub const HOLDING_KINDS: [&str; 7] = [
    "Vehicle",
    "Crypto",
    "Private stock",
    "Collectible",
    "Precious metals",
    "Property",
    "Other asset",
];

impl App {
    /// Add something you own to the balance sheet at its current value.
    pub fn add_holding(&self, name: &str, kind: &str, value_cents: i64) -> Result<(), String> {
        self.require_edit()?;
        let name = name.trim();
        if name.is_empty() {
            return Err("Give it a name, such as \"2019 Honda Civic\".".into());
        }
        let kind = HOLDING_KINDS
            .iter()
            .find(|k| k.eq_ignore_ascii_case(kind.trim()))
            .ok_or_else(|| format!("Choose one of: {}.", HOLDING_KINDS.join(", ")))?;
        if value_cents < 0 {
            return Err("A value can't be negative. Debts go in as loans or cards.".into());
        }
        if self
            .accounts()?
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case(name))
        {
            return Err(format!(
                "You already have an account or holding called {name}."
            ));
        }
        let db_kind = if *kind == "Property" {
            "property"
        } else {
            "other"
        };
        let id = self
            .db
            .add_account(name, db_kind, Some(kind), None)
            .map_err(|e| e.to_string())?;
        self.set_opening_balance(id.0, value_cents)
    }

    pub fn balance_sheet(&self) -> Result<BalanceSheet, String> {
        let mut assets = vec![];
        let mut debts = vec![];
        let labels = self.db.account_institutions().map_err(|e| e.to_string())?;
        for mut a in self.accounts()? {
            // Hand-entered holdings keep the label they were added with, such as "Vehicle".
            if let Some(l) = labels
                .get(&a.id)
                .filter(|l| HOLDING_KINDS.contains(&l.as_str()))
            {
                a.kind = l.clone();
            }
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
