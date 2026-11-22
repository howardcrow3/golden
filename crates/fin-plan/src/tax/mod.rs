//! Tax engine: bracket math, federal income tax, long-term gains and FICA.
//! Rules live in versioned JSON data files so they can be updated yearly without code changes.

pub mod brackets;
pub mod federal;

pub use brackets::{Bracket, BracketTable};
pub use federal::{
    early_withdrawal_penalty, salt_deduction, taxable_social_security, FederalInput, FederalResult,
    FederalTaxYear, FilingStatus,
};
pub mod state;
pub use state::{StateRules, StateTaxYear};
