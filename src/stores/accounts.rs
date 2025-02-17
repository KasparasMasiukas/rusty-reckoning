//! Account state storage and management.
//!
//! Provides functionality for:
//! - Storing and retrieving account balances
//! - Managing available and held funds
//! - Handling account locks
//! - Creating new accounts on demand

use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::Error;

/// Account state including balance and lock status.
#[derive(Debug)]
pub struct Account {
    pub id: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

impl Account {
    pub fn total(&self) -> Decimal {
        self.available + self.held
    }
}

#[derive(Default)]
pub struct AccountsStore {
    accounts: HashMap<u16, Account>,
}

impl AccountsStore {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    /// Checks that an account is not locked.
    /// This check should be performed early in the pipeline to avoid handling
    /// transactions for locked accounts.
    pub fn check_account_lock(&self, client: u16) -> Result<(), Error> {
        if let Some(account) = self.accounts.get(&client) {
            if account.locked {
                return Err(Error::AccountLocked);
            }
        }
        Ok(())
    }

    /// Gets a mutable account entry, or creates one if it doesn't exist.
    pub fn get_or_create_mut(&mut self, client: u16) -> &mut Account {
        self.accounts.entry(client).or_insert_with(|| Account {
            id: client,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        })
    }

    /// Gets an account entry, or returns an error if it doesn't exist.
    pub fn get_mut(&mut self, client: u16) -> Result<&mut Account, Error> {
        self.accounts.get_mut(&client).ok_or(Error::AccountNotFound)
    }

    /// Returns an iterator over all accounts.
    /// Provides no guarantees about the order of the accounts.
    pub fn iter(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_new_store_is_empty() {
        let store = AccountsStore::new();
        assert!(store.iter().next().is_none());
    }

    #[test]
    fn test_get_or_create_new_account() {
        let mut store = AccountsStore::new();
        let account = store.get_or_create_mut(1);

        assert_eq!(account.id, 1);
        assert_eq!(account.available, Decimal::ZERO);
        assert_eq!(account.held, Decimal::ZERO);
        assert!(!account.locked);
    }

    #[test]
    fn test_get_existing_account() {
        let mut store = AccountsStore::new();

        // Create account first
        {
            let account = store.get_or_create_mut(1);
            account.available = dec!(100);
        }

        // Get it again
        let account = store.get_mut(1).unwrap();
        assert_eq!(account.available, dec!(100));
    }

    #[test]
    fn test_get_nonexistent_account() {
        let mut store = AccountsStore::new();
        assert!(matches!(store.get_mut(1), Err(Error::AccountNotFound)));
    }

    #[test]
    fn test_account_lock_check() {
        let mut store = AccountsStore::new();

        // New account should pass lock check
        {
            store.get_or_create_mut(1);
        }
        assert!(store.check_account_lock(1).is_ok());

        // Lock the account
        let account = store.get_or_create_mut(1);
        account.locked = true;

        // Check should fail for locked account
        assert!(matches!(
            store.check_account_lock(1),
            Err(Error::AccountLocked)
        ));

        // Non-existent account should pass lock check
        assert!(store.check_account_lock(2).is_ok());
    }

    #[test]
    fn test_account_total() {
        let mut store = AccountsStore::new();
        let account = store.get_or_create_mut(1);

        account.available = dec!(100.50);
        account.held = dec!(50.25);

        assert_eq!(account.total(), dec!(150.75));
    }

    #[test]
    fn test_store_iterator() {
        let mut store = AccountsStore::new();

        // Create a few accounts
        store.get_or_create_mut(1).available = dec!(100);
        store.get_or_create_mut(2).available = dec!(200);
        store.get_or_create_mut(3).available = dec!(300);

        let total_available: Decimal = store.iter().map(|acc| acc.available).sum();

        assert_eq!(total_available, dec!(600));
    }

    #[test]
    fn test_multiple_get_or_create_same_account() {
        let mut store = AccountsStore::new();

        // First creation
        let account = store.get_or_create_mut(1);
        account.available = dec!(100);

        // Second get_or_create should return the same account
        let account = store.get_or_create_mut(1);
        assert_eq!(account.available, dec!(100));

        // Ensure only one account exists
        assert_eq!(store.iter().count(), 1);
    }
}
