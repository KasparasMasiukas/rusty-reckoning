use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::Error;

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

    pub fn iter(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }
}
