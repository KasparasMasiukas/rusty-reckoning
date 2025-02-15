//! Core transaction processing engine for the payment system.
//!
//! The engine is responsible for:
//! - Processing deposits, withdrawals, and dispute-related transactions
//! - Maintaining account balances and states
//! - Enforcing business rules like insufficient funds checks and account locks
//!
//! The [`Engine`] struct serves as the main entry point for transaction processing,
//! coordinating between the accounts and transactions stores while ensuring
//! data consistency and transaction validity.

use rust_decimal::Decimal;

use crate::{Account, AccountsStore, Error, Transaction, TransactionType, TransactionsStore};

#[derive(Default)]
pub struct Engine {
    accounts: AccountsStore,
    transactions: TransactionsStore,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            accounts: AccountsStore::new(),
            transactions: TransactionsStore::new(),
        }
    }

    pub fn process_transaction(&mut self, transaction: Transaction) -> Result<(), Error> {
        self.accounts.check_account_lock(transaction.client)?;

        match transaction.tx_type {
            TransactionType::Deposit => self.process_deposit(
                transaction.client,
                transaction.tx,
                transaction.amount.ok_or(Error::InvalidTransaction)?,
            ),
            TransactionType::Withdrawal => self.process_withdrawal(
                transaction.client,
                transaction.tx,
                transaction.amount.ok_or(Error::InvalidTransaction)?,
            ),
            TransactionType::Dispute => self.process_dispute(transaction.client, transaction.tx),
            TransactionType::Resolve => self.process_resolve(transaction.client, transaction.tx),
            TransactionType::Chargeback => {
                self.process_chargeback(transaction.client, transaction.tx)
            }
        }
    }

    fn process_deposit(&mut self, client: u16, tx: u32, amount: Decimal) -> Result<(), Error> {
        if amount <= Decimal::ZERO {
            return Err(Error::AmountMustBePositive);
        }
        if self.transactions.is_processed(tx) {
            return Err(Error::DuplicateTransaction);
        }

        self.transactions.store_deposit(tx, client, amount);
        let account = self.accounts.get_or_create_mut(client);
        account.available += amount;
        self.transactions.mark_processed(tx);
        Ok(())
    }

    fn process_withdrawal(&mut self, client: u16, tx: u32, amount: Decimal) -> Result<(), Error> {
        if amount <= Decimal::ZERO {
            return Err(Error::AmountMustBePositive);
        }
        if self.transactions.is_processed(tx) {
            return Err(Error::DuplicateTransaction);
        }

        let account = self.accounts.get_mut(client)?;
        if account.available < amount {
            return Err(Error::InsufficientFunds);
        }
        account.available -= amount;
        self.transactions.mark_processed(tx);
        Ok(())
    }

    fn process_dispute(&mut self, client: u16, tx: u32) -> Result<(), Error> {
        let deposit = self.transactions.get_mut_deposit(client, tx)?;
        if deposit.disputed {
            return Err(Error::TransactionAlreadyDisputed);
        }
        deposit.disputed = true;

        let amount = deposit.amount;
        let account = self.accounts.get_or_create_mut(client);
        account.held += amount;
        account.available -= amount;
        Ok(())
    }

    fn process_resolve(&mut self, client: u16, tx: u32) -> Result<(), Error> {
        let deposit = self.transactions.get_mut_deposit(client, tx)?;
        if !deposit.disputed {
            return Err(Error::TransactionNotDisputed);
        }
        deposit.disputed = false;

        let amount = deposit.amount;
        let account = self.accounts.get_or_create_mut(client);
        account.held -= amount;
        account.available += amount;
        Ok(())
    }

    fn process_chargeback(&mut self, client: u16, tx: u32) -> Result<(), Error> {
        let deposit = self.transactions.get_mut_deposit(client, tx)?;
        if !deposit.disputed {
            return Err(Error::TransactionNotDisputed);
        }
        deposit.disputed = false;

        let amount = deposit.amount;
        let account = self.accounts.get_or_create_mut(client);
        account.held -= amount;
        account.locked = true;
        Ok(())
    }

    pub fn accounts(&self) -> impl Iterator<Item = &Account> {
        self.accounts.iter()
    }
}
