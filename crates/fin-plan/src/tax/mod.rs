//! Tax engine: bracket math, federal income tax, long-term gains and FICA.
//! Rules live in versioned JSON data files so they can be updated yearly without code changes.

pub mod brackets;
pub mod federal;

pub use brackets::{Bracket, BracketTable};
pub use federal::{FederalInput, FederalResult, FederalTaxYear, FilingStatus};
