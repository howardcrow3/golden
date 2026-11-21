//! Tax engine: bracket math, federal income tax, long-term gains and FICA.
//! Rules live in versioned JSON data files so they can be updated yearly without code changes.

pub mod brackets;
pub mod federal;

pub use brackets::{Bracket, BracketTable};
pub use federal::{
    taxable_social_security, FederalInput, FederalResult, FederalTaxYear, FilingStatus,
};
pub mod state;
pub use state::{StateRules, StateTaxYear};
