//! Local AI for Golden: the model catalog, a hardware check and resumable, verified model
//! downloads. The catalog, hardware check and downloader come from CeeVee's `ceevee-ai`
//! (the plan in docs/04-architecture.md is to share them as `turtlebox-ai` once that is
//! extracted); they are kept line-for-line close so they can be swapped for the shared crate.
pub mod catalog;
pub mod download;
pub mod hardware;
pub mod manager;

pub use catalog::{Catalog, ModelEntry, Role};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("cancelled")]
    Cancelled,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
