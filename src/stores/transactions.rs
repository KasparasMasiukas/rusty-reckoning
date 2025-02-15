use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

use crate::Error;

#[derive(Debug)]
pub struct StoredDeposit {
    pub client: u16,
    pub amount: Decimal,
    pub disputed: bool,
}

#[derive(Default)]
pub struct TransactionsStore {
    /// Deposits can be disputed, so this is a map of all successful deposits
    deposits: HashMap<u32, StoredDeposit>,
    /// Set of all successfully processed deposit/withdrawal transaction IDs to prevent duplicates
    processed_transactions: HashSet<u32>,
}

impl TransactionsStore {
    pub fn new() -> Self {
        Self {
            deposits: HashMap::new(),
            processed_transactions: HashSet::new(),
        }
    }

    /// Checks if a transaction has been processed already.
    /// Processed transactions cannot be repeated.
    pub fn is_processed(&self, tx: u32) -> bool {
        self.processed_transactions.contains(&tx)
    }

    /// Marks a transaction as processed.
    /// Processed transactions cannot be repeated.
    pub fn mark_processed(&mut self, tx: u32) {
        self.processed_transactions.insert(tx);
    }

    /// Stores a deposit transaction to track its dispute status.
    pub fn store_deposit(&mut self, tx: u32, client: u16, amount: Decimal) {
        self.deposits.insert(
            tx,
            StoredDeposit {
                client,
                amount,
                disputed: false,
            },
        );
    }

    /// Gets a stored deposit entry if it exists, and validates that it belongs to the client.
    /// Returns a mutable reference to the deposit, or an error if the deposit does not exist or
    /// belongs to a different client.
    pub fn get_mut_deposit(&mut self, client: u16, tx: u32) -> Result<&mut StoredDeposit, Error> {
        let deposit = self
            .deposits
            .get_mut(&tx)
            .ok_or(Error::TransactionNotFound)?;
        if deposit.client != client {
            return Err(Error::InvalidTransaction);
        }
        Ok(deposit)
    }
}
