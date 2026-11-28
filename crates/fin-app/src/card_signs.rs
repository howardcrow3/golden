//! Some card issuers (American Express, Discover) export CSVs with charges as positive amounts
//! and payments as negative, the reverse of every other file Golden reads. Read as-is, every
//! purchase would count as income. Such a file is turned the right way round on import, but
//! only when the signal is clear, and never in a checking or savings account.
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
}
