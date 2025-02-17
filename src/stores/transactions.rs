//! Transaction history storage for dispute resolution and duplicate prevention.
//!
//! Maintains a record of deposits for:
//! - Preventing duplicate transactions
//! - Supporting dispute/resolve/chargeback operations
//! - Validating transaction ownership

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

    /// Stores a new deposit transaction to track its dispute status.
    /// Returns an error if the deposit with the same transaction ID already exists.
    pub fn store_new_deposit(
        &mut self,
        tx: u32,
        client: u16,
        amount: Decimal,
    ) -> Result<(), Error> {
        if self.deposits.contains_key(&tx) {
            return Err(Error::DuplicateTransaction);
        }
        self.deposits.insert(
            tx,
            StoredDeposit {
                client,
                amount,
                disputed: false,
            },
        );
        Ok(())
    }

    /// Gets a stored deposit entry if it exists, and validates that it belongs to the client.
    /// Returns a mutable reference to the deposit, or an error if the deposit does not exist or
    /// belongs to a different client.
    pub fn get_deposit_mut(&mut self, client: u16, tx: u32) -> Result<&mut StoredDeposit, Error> {
        let deposit = self
            .deposits
            .get_mut(&tx)
            .ok_or(Error::TransactionNotFound)?;
        if deposit.client != client {
            return Err(Error::TransactionClientMismatch);
        }
        Ok(deposit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_new_store_is_empty() {
        let mut store = TransactionsStore::new();
        assert!(!store.is_processed(1));
        assert!(store.get_deposit_mut(1, 1).is_err());
    }

    #[test]
    fn test_mark_and_check_processed() {
        let mut store = TransactionsStore::new();

        // Initially not processed
        assert!(!store.is_processed(1));

        // Mark as processed
        store.mark_processed(1);
        assert!(store.is_processed(1));

        // Other transactions still not processed
        assert!(!store.is_processed(2));
    }

    #[test]
    fn test_store_and_get_deposit() {
        let mut store = TransactionsStore::new();
        let tx = 1;
        let client = 1;
        let amount = dec!(100.50);

        // Store deposit
        store.store_new_deposit(tx, client, amount).unwrap();

        // Retrieve and verify
        let deposit = store.get_deposit_mut(client, tx).unwrap();
        assert_eq!(deposit.client, client);
        assert_eq!(deposit.amount, amount);
        assert!(!deposit.disputed);
    }

    #[test]
    fn test_get_nonexistent_deposit() {
        let mut store = TransactionsStore::new();
        assert!(matches!(
            store.get_deposit_mut(1, 1),
            Err(Error::TransactionNotFound)
        ));
    }

    #[test]
    fn test_get_deposit_wrong_client() {
        let mut store = TransactionsStore::new();
        let tx = 1;
        let client = 1;
        let amount = dec!(100);

        // Store deposit for client 1
        store.store_new_deposit(tx, client, amount).unwrap();

        // Try to access with client 2
        assert!(matches!(
            store.get_deposit_mut(2, tx),
            Err(Error::TransactionClientMismatch)
        ));
    }

    #[test]
    fn test_duplicate_tx_different_clients() {
        let mut store = TransactionsStore::new();
        let tx = 1;

        // Create deposit for first client
        store.store_new_deposit(tx, 1, dec!(100)).unwrap();

        // Attempt to create deposit with same tx for different client
        let result = store.store_new_deposit(tx, 2, dec!(200));
        assert!(matches!(result, Err(Error::DuplicateTransaction)));

        // Verify original deposit remains unchanged
        let deposit = store.get_deposit_mut(1, tx).unwrap();
        assert_eq!(deposit.client, 1);
        assert_eq!(deposit.amount, dec!(100));
    }

    #[test]
    fn test_deposit_dispute_status() {
        let mut store = TransactionsStore::new();
        let tx = 1;
        let client = 1;

        store.store_new_deposit(tx, client, dec!(100)).unwrap();

        // Modify dispute status
        {
            let deposit = store.get_deposit_mut(client, tx).unwrap();
            deposit.disputed = true;
        }

        // Verify status persists
        let deposit = store.get_deposit_mut(client, tx).unwrap();
        assert!(deposit.disputed);
    }

    #[test]
    fn test_multiple_deposits_same_client() {
        let mut store = TransactionsStore::new();
        let client = 1;

        // Store multiple deposits
        store.store_new_deposit(1, client, dec!(100)).unwrap();
        store.store_new_deposit(2, client, dec!(200)).unwrap();

        // Verify each deposit independently to avoid multiple mutable borrows
        {
            let deposit1 = store.get_deposit_mut(client, 1).unwrap();
            assert_eq!(deposit1.amount, dec!(100));
        }
        {
            let deposit2 = store.get_deposit_mut(client, 2).unwrap();
            assert_eq!(deposit2.amount, dec!(200));
        }
    }

    #[test]
    fn test_overwrite_deposit_should_fail() {
        let mut store = TransactionsStore::new();
        let tx = 1;
        let client = 1;

        // Store initial deposit
        store.store_new_deposit(tx, client, dec!(100)).unwrap();

        // Attempt to overwrite with new amount
        let result = store.store_new_deposit(tx, client, dec!(200));
        assert!(matches!(result, Err(Error::DuplicateTransaction)));

        // Verify original amount remains
        let deposit = store.get_deposit_mut(client, tx).unwrap();
        assert_eq!(deposit.amount, dec!(100));
    }

    #[test]
    fn test_processed_and_stored_independence() {
        let mut store = TransactionsStore::new();
        let tx = 1;
        let client = 1;

        // Mark as processed without storing
        store.mark_processed(tx);
        assert!(store.is_processed(tx));
        assert!(store.get_deposit_mut(client, tx).is_err());

        // Store without marking as processed
        let tx2 = 2;
        store.store_new_deposit(tx2, client, dec!(100)).unwrap();
        assert!(!store.is_processed(tx2));
        assert!(store.get_deposit_mut(client, tx2).is_ok());
    }
}
