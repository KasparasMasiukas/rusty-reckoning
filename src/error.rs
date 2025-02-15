//! Domain-specific errors for the payment processing system.
//!
//! Contains error variants for common failure cases like:
//! - Account-related errors (not found, locked)
//! - Transaction validation errors (duplicate, invalid amount)
//! - Dispute-related errors (already disputed, not disputed)
//!
//! These errors represent business logic failures rather than
//! technical errors like I/O or parsing issues.

#[derive(Debug)]
pub enum Error {
    AccountLocked,
    AccountNotFound,
    AmountMustBePositive,
    DuplicateTransaction,
    InsufficientFunds,
    InvalidTransaction,
    TransactionAlreadyDisputed,
    TransactionNotDisputed,
    TransactionNotFound,
}
