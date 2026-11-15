//! Bookkeeping logic on top of parsed transactions: payee normalization, categorization
//! rules, and transfer detection. Pure functions, no I/O.
pub mod normalize;
pub mod rules;
pub mod transfers;
