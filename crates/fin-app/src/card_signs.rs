//! Some card issuers (American Express, Discover) export CSVs with charges as positive amounts
//! and payments as negative, the reverse of every other file Golden reads. Read as-is, every
//! purchase would count as income. Such a file is turned the right way round on import, but
//! only when the signal is clear, and never in a checking or savings account. The person can
//! settle it for an account either way, which overrides the detection.
use crate::App;
use fin_db::ledger::AccountId;
use fin_ingest::ParsedTxn;

impl App {
    /// Should this CSV's amounts be flipped before they are stored in `account`?
    pub(crate) fn card_csv_reversed(
        &self,
        bytes: &[u8],
        account: AccountId,
        parsed: &[ParsedTxn],
    ) -> Result<bool, String> {
        let Some(layout) = fin_ingest::csv_sign_layout(bytes) else {
            return Ok(false);
        };
        if let Some(set) = self
            .db
            .account_amounts_reversed(account)
            .map_err(|e| e.to_string())?
        {
            return Ok(set);
        }
        if !layout.single_amount {
            return Ok(false);
        }
        let kind = self
            .db
            .accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|(id, ..)| *id == account)
            .map(|(_, _, kind, _)| kind)
            .unwrap_or_default();
        let card = kind == "credit";
        let bank = kind == "checking" || kind == "savings";
        Ok(!bank
            && (card || layout.reversed_issuer)
            && fin_ingest::csv_reader::card_signs_reversed(parsed))
    }

    /// Say whether an account's CSV files print charges as positive amounts: `Some(true)` turns
    /// every future CSV import round, `Some(false)` never does, `None` goes back to detecting
    /// it from each file. Transactions already imported are not changed.
    pub fn set_amounts_reversed(
        &self,
        account_id: i64,
        reversed: Option<bool>,
    ) -> Result<(), String> {
        self.require_edit()?;
        self.db
            .set_amounts_reversed(AccountId(account_id), reversed)
            .map_err(|_| "No such account.".to_string())
    }
}
