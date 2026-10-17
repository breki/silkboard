//! In-repository deployment engine, evidence policy and installed adapters.
//! Backends are trusted operator code; identity types are not authorization.

pub mod cli;
pub mod engine;
pub mod evidence;
pub mod linux;
pub mod outcome;
pub mod profile;
pub mod storage;

pub use engine::{Backend, Release, State, Transaction, Version, poll, retry};
