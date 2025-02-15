//! Storage layer for the payment processing system. Provides storage for:
//! - Account balances and states ([`AccountsStore`])
//! - Transaction history for dispute handling ([`TransactionsStore`])
//!
//! Current implementation is optimized for synchronous, direct memory
//! access.

mod accounts;
mod transactions;

pub use accounts::{Account, AccountsStore};
pub use transactions::{StoredDeposit, TransactionsStore};
