//! Domain primitives shared by every Golden crate: money in integer cents,
//! calendar grains (week/month/quarter/year) and people with birthdates.

pub mod grain;
pub mod money;
pub mod person;

pub use chrono::NaiveDate;
pub use grain::{Grain, PeriodKey};
pub use money::{Money, MoneyError};
pub use person::{age_on, Person};
