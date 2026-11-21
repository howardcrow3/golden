//! Local AI for Golden: the model catalog, a hardware check and resumable, verified model
//! downloads. The catalog, hardware check and downloader come from CeeVee's `ceevee-ai`
//! (the plan in docs/04-architecture.md is to share them as `turtlebox-ai` once that is
//! extracted); they are kept line-for-line close so they can be swapped for the shared crate.
pub mod backend;
pub mod catalog;
pub mod download;
pub mod engine;
pub mod grammar;
pub mod hardware;
#[cfg(feature = "llama")]
pub mod llama;
pub mod manager;
pub mod task;
pub mod validate;
pub mod worker;

pub use catalog::{Catalog, ModelEntry, Role};
pub use engine::Engine;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("cancelled")]
    Cancelled,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
