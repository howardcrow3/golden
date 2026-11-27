//! Receipts in the vault, matched to the transactions that paid for them. A receipt never adds
//! an expense of its own: the statement or bank line is the record, and the receipt is attached
//! to it. So a purchase is counted once whether the receipt or the statement arrives first, and
//! a receipt uploaded twice is flagged instead of linked twice.
use crate::App;
use chrono::NaiveDate;
use fin_db::dupes::same_payee;
use fin_ingest::receipt::{parse_receipt, Receipt};
use serde::Serialize;
use std::collections::HashMap;

/// How many days from the receipt's date a charge may post.
const WINDOW_DAYS: i64 = 5;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TxnRef {
    pub id: i64,
    pub day: String,
    pub payee: String,
    pub cents: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptItem {
    pub document_id: i64,
    pub name: String,
    pub merchant: String,
    pub total_cents: Option<i64>,
    pub day: Option<String>,
    /// "linked", "suggested" (one or more possible charges), "duplicate" (the same receipt
    /// is already linked) or "unmatched" (no charge found yet).
    pub status: String,
    pub linked: Option<TxnRef>,
    pub candidates: Vec<TxnRef>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptReport {
    /// Receipts linked automatically by this run.
    pub auto_linked: usize,
    pub items: Vec<ReceiptItem>,
}

fn tref(t: &fin_db::ledger::Txn) -> TxnRef {
    TxnRef {
        id: t.id,
        day: t.day.to_string(),
        payee: t.payee.clone(),
        cents: t.amount.cents(),
    }
}

/// Two receipts that read the same are one purchase photographed twice.
fn same_receipt(a: &Receipt, b: &Receipt) -> bool {
    a.total.is_some()
        && a.total == b.total
        && a.day == b.day
        && same_payee(&a.merchant, &b.merchant)
}

impl App {
    /// Match every unlinked receipt in the vault to the charge it paid for. A receipt is linked
    /// on its own when exactly one charge fits (same amount, a few days of its date, and the
    /// same shop or the same day); anything less certain is left as a suggestion to confirm.
    pub fn match_receipts(&self) -> Result<ReceiptReport, String> {
        let db = &self.db;
        let docs: Vec<_> = db
            .list_documents("")
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|d| d.kind == "Receipt")
            .collect();
        let mut parsed: HashMap<i64, Receipt> = HashMap::new();
        for d in &docs {
            let text = db
                .document_text(d.id)
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            parsed.insert(d.id, parse_receipt(&text));
        }
        let mut links: HashMap<i64, i64> = db
            .receipt_links()
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect();
        let from = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2999, 12, 31).unwrap();
        let txns = db.txns_between(from, to).map_err(|e| e.to_string())?;
        let by_id: HashMap<i64, &fin_db::ledger::Txn> = txns.iter().map(|t| (t.id, t)).collect();

        // Oldest document first, so of two copies the earlier keeps the link.
        let mut order: Vec<&fin_db::documents::DocRow> = docs.iter().collect();
        order.sort_by_key(|d| d.id);
        let mut auto_linked = 0;
        let mut items = Vec::new();
        for d in order {
            let r = &parsed[&d.id];
            let mut item = ReceiptItem {
                document_id: d.id,
                name: d.name.clone(),
                merchant: r.merchant.clone(),
                total_cents: r.total.map(|t| t.cents()),
                day: r.day.map(|x| x.to_string()),
                status: "unmatched".into(),
                linked: None,
                candidates: vec![],
            };
            if let Some(t) = links.get(&d.id).and_then(|id| by_id.get(id)) {
                item.status = "linked".into();
                item.linked = Some(tref(t));
                items.push(item);
                continue;
            }
            let Some(total) = r.total else {
                items.push(item);
                continue;
            };
            let taken: Vec<i64> = links.values().copied().collect();
            // Charges for the receipt's total, near its date when it has one.
            let mut cands: Vec<&fin_db::ledger::Txn> = txns
                .iter()
                .filter(|t| t.amount.cents() == -total.cents() && !taken.contains(&t.id))
                .filter(|t| {
                    r.day
                        .map(|day| (t.day - day).num_days().abs() <= WINDOW_DAYS)
                        .unwrap_or(true)
                })
                .collect();
            cands.sort_by_key(|t| r.day.map(|day| (t.day - day).num_days().abs()).unwrap_or(0));
            cands.truncate(5);
            // Already linked to the same purchase by another copy of this receipt?
            let dup = links.iter().any(|(doc, txn)| {
                parsed.get(doc).is_some_and(|o| same_receipt(o, r))
                    && by_id.get(txn).is_some_and(|t| {
                        t.amount.cents() == -total.cents()
                            && r.day
                                .map(|day| (t.day - day).num_days().abs() <= WINDOW_DAYS)
                                .unwrap_or(true)
                    })
            });
            if dup {
                item.status = "duplicate".into();
                items.push(item);
                continue;
            }
            item.candidates = cands.iter().map(|t| tref(t)).collect();
            if let [only] = cands.as_slice() {
                let certain = r.day == Some(only.day) || same_payee(&r.merchant, &only.payee);
                if certain && r.day.is_some() {
                    db.link_receipt(d.id, only.id).map_err(|e| e.to_string())?;
                    links.insert(d.id, only.id);
                    item.status = "linked".into();
                    item.linked = Some(tref(only));
                    item.candidates.clear();
                    auto_linked += 1;
                    items.push(item);
                    continue;
                }
            }
            if !item.candidates.is_empty() {
                item.status = "suggested".into();
            }
            items.push(item);
        }
        Ok(ReceiptReport { auto_linked, items })
    }

    /// Confirm a suggested match, or move a receipt to a different charge.
    pub fn link_receipt(&self, document_id: i64, txn_id: i64) -> Result<(), String> {
        self.require_edit()?;
        self.db.get_txn(txn_id).map_err(|e| e.to_string())?;
        self.db
            .link_receipt(document_id, txn_id)
            .map_err(|e| e.to_string())
    }

    /// Undo a match (right or wrong); the receipt goes back to being unmatched.
    pub fn unlink_receipt(&self, document_id: i64) -> Result<(), String> {
        self.require_edit()?;
        self.db
            .unlink_receipt(document_id)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
