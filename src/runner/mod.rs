//! The runner is responsible for setting up a file stream for reading from CSV,
//! processing the transactions, and writing the output to a writer.
//!
//! This module provides both a synchronous and an asynchronous runner implementations.
//!
mod async_runner;
mod sync_runner;

pub use async_runner::run as run_async;
pub use sync_runner::run;
