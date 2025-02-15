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

        self.transactions.store_new_deposit(tx, client, amount)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn create_transaction(
        tx_type: TransactionType,
        client: u16,
        tx: u32,
        amount: Option<Decimal>,
    ) -> Transaction {
        Transaction {
            tx_type,
            client,
            tx,
            amount,
        }
    }

    #[test]
    fn test_valid_deposit() {
        let mut engine = Engine::new();
        let tx = create_transaction(TransactionType::Deposit, 1, 1, Some(dec!(100.0)));

        engine.process_transaction(tx).unwrap();

        let account = engine.accounts().next().unwrap();
        assert_eq!(account.available, dec!(100.0));
        assert_eq!(account.total(), dec!(100.0));
        assert_eq!(account.held, dec!(0.0));
        assert!(!account.locked);
    }

    #[test]
    fn test_duplicate_deposit() {
        let mut engine = Engine::new();
        let tx = create_transaction(TransactionType::Deposit, 1, 1, Some(dec!(100.0)));

        engine.process_transaction(tx.clone()).unwrap();
        assert!(matches!(
            engine.process_transaction(tx),
            Err(Error::DuplicateTransaction)
        ));

        let account = engine.accounts().next().unwrap();
        assert_eq!(account.available, dec!(100.0));
    }

    #[test]
    fn test_valid_withdrawal() {
        let mut engine = Engine::new();

        // First deposit
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // Then withdraw
        engine
            .process_transaction(create_transaction(
                TransactionType::Withdrawal,
                1,
                2,
                Some(dec!(50.0)),
            ))
            .unwrap();

        let account = engine.accounts().next().unwrap();
        assert_eq!(account.available, dec!(50.0));
    }

    #[test]
    fn test_withdrawal_insufficient_funds() {
        let mut engine = Engine::new();

        // Deposit 100
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // Try to withdraw 150
        let result = engine.process_transaction(create_transaction(
            TransactionType::Withdrawal,
            1,
            2,
            Some(dec!(150.0)),
        ));

        assert!(matches!(result, Err(Error::InsufficientFunds)));

        let account = engine.accounts().next().unwrap();
        assert_eq!(account.available, dec!(100.0));
    }

    #[test]
    fn test_withdrawal_from_nonexistent_account() {
        let mut engine = Engine::new();
        let result = engine.process_transaction(create_transaction(
            TransactionType::Withdrawal,
            1,
            1,
            Some(dec!(50.0)),
        ));

        assert!(matches!(result, Err(Error::AccountNotFound)));
    }

    #[test]
    fn test_deposit_non_positive_amount() {
        let mut engine = Engine::new();

        // Test zero amount
        let result = engine.process_transaction(create_transaction(
            TransactionType::Deposit,
            1,
            1,
            Some(dec!(0.0)),
        ));
        assert!(matches!(result, Err(Error::AmountMustBePositive)));

        // Test negative amount
        let result = engine.process_transaction(create_transaction(
            TransactionType::Deposit,
            1,
            2,
            Some(dec!(-10.0)),
        ));
        assert!(matches!(result, Err(Error::AmountMustBePositive)));
    }

    #[test]
    fn test_withdrawal_non_positive_amount() {
        let mut engine = Engine::new();

        // First make a deposit
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // Test zero amount
        let result = engine.process_transaction(create_transaction(
            TransactionType::Withdrawal,
            1,
            2,
            Some(dec!(0.0)),
        ));
        assert!(matches!(result, Err(Error::AmountMustBePositive)));

        // Test negative amount
        let result = engine.process_transaction(create_transaction(
            TransactionType::Withdrawal,
            1,
            3,
            Some(dec!(-10.0)),
        ));
        assert!(matches!(result, Err(Error::AmountMustBePositive)));
    }

    #[test]
    fn test_duplicate_withdrawal() {
        let mut engine = Engine::new();

        // First make a deposit to have funds
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // First withdrawal succeeds
        let withdrawal = create_transaction(TransactionType::Withdrawal, 1, 2, Some(dec!(50.0)));
        engine.process_transaction(withdrawal.clone()).unwrap();

        // Second withdrawal with same tx ID fails
        let result = engine.process_transaction(withdrawal);
        assert!(matches!(result, Err(Error::DuplicateTransaction)));

        // Verify account state hasn't changed after failed withdrawal
        let account = engine.accounts().next().unwrap();
        assert_eq!(account.available, dec!(50.0));
        assert_eq!(account.total(), dec!(50.0));
    }

    #[test]
    fn test_valid_dispute() {
        let mut engine = Engine::new();

        // Make a deposit
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // Dispute it
        engine
            .process_transaction(create_transaction(TransactionType::Dispute, 1, 1, None))
            .unwrap();

        let account = engine.accounts().next().unwrap();
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(100.0));
        assert_eq!(account.total(), dec!(100.0));
    }

    #[test]
    fn test_duplicate_dispute() {
        let mut engine = Engine::new();

        // Make a deposit
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // First dispute
        let tx = create_transaction(TransactionType::Dispute, 1, 1, None);
        engine.process_transaction(tx.clone()).unwrap();

        // Second dispute
        let result = engine.process_transaction(tx);

        assert!(matches!(result, Err(Error::TransactionAlreadyDisputed)));
    }

    #[test]
    fn test_dispute_nonexistent_transaction() {
        let mut engine = Engine::new();
        let result =
            engine.process_transaction(create_transaction(TransactionType::Dispute, 1, 999, None));

        assert!(matches!(result, Err(Error::TransactionNotFound)));
    }

    #[test]
    fn test_dispute_wrong_client() {
        let mut engine = Engine::new();

        // Make a deposit for client 1
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // Try to dispute it as client 2
        let result =
            engine.process_transaction(create_transaction(TransactionType::Dispute, 2, 1, None));

        assert!(matches!(result, Err(Error::TransactionClientMismatch)));
    }

    #[test]
    fn test_valid_resolve() {
        let mut engine = Engine::new();

        // Make a deposit
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // Dispute it
        engine
            .process_transaction(create_transaction(TransactionType::Dispute, 1, 1, None))
            .unwrap();

        // Resolve it
        engine
            .process_transaction(create_transaction(TransactionType::Resolve, 1, 1, None))
            .unwrap();

        let account = engine.accounts().next().unwrap();
        assert_eq!(account.available, dec!(100.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total(), dec!(100.0));
    }

    #[test]
    fn test_resolve_without_dispute() {
        let mut engine = Engine::new();

        // Make a deposit
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // Try to resolve without dispute
        let result =
            engine.process_transaction(create_transaction(TransactionType::Resolve, 1, 1, None));

        assert!(matches!(result, Err(Error::TransactionNotDisputed)));
    }

    #[test]
    fn test_valid_chargeback() {
        let mut engine = Engine::new();

        // Make a deposit
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // Dispute it
        engine
            .process_transaction(create_transaction(TransactionType::Dispute, 1, 1, None))
            .unwrap();

        // Chargeback
        engine
            .process_transaction(create_transaction(TransactionType::Chargeback, 1, 1, None))
            .unwrap();

        let account = engine.accounts().next().unwrap();
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total(), dec!(0.0));
        assert!(account.locked);
    }

    #[test]
    fn test_chargeback_without_dispute() {
        let mut engine = Engine::new();

        // Make a deposit
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // Try chargeback without dispute
        let result =
            engine.process_transaction(create_transaction(TransactionType::Chargeback, 1, 1, None));

        assert!(matches!(result, Err(Error::TransactionNotDisputed)));
        let account = engine.accounts().next().unwrap();
        assert_eq!(account.available, dec!(100.0));
        assert!(!account.locked);
    }

    #[test]
    fn test_locked_account_rejects_transactions() {
        let mut engine = Engine::new();

        // Make a deposit
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // Dispute and chargeback to lock the account
        engine
            .process_transaction(create_transaction(TransactionType::Dispute, 1, 1, None))
            .unwrap();
        engine
            .process_transaction(create_transaction(TransactionType::Chargeback, 1, 1, None))
            .unwrap();

        // Try new deposit
        let result = engine.process_transaction(create_transaction(
            TransactionType::Deposit,
            1,
            2,
            Some(dec!(50.0)),
        ));
        assert!(matches!(result, Err(Error::AccountLocked)));

        // Try withdrawal
        let result = engine.process_transaction(create_transaction(
            TransactionType::Withdrawal,
            1,
            3,
            Some(dec!(50.0)),
        ));
        assert!(matches!(result, Err(Error::AccountLocked)));
    }

    #[test]
    fn test_redispute_after_resolve() {
        let mut engine = Engine::new();

        // Make a deposit
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // First dispute cycle
        engine
            .process_transaction(create_transaction(TransactionType::Dispute, 1, 1, None))
            .unwrap();
        engine
            .process_transaction(create_transaction(TransactionType::Resolve, 1, 1, None))
            .unwrap();

        // Second dispute should work
        engine
            .process_transaction(create_transaction(TransactionType::Dispute, 1, 1, None))
            .unwrap();

        let account = engine.accounts().next().unwrap();
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(100.0));
        assert_eq!(account.total(), dec!(100.0));
    }

    #[test]
    fn test_chargeback_results_in_negative_balance() {
        let mut engine = Engine::new();

        // Make a deposit of 100
        engine
            .process_transaction(create_transaction(
                TransactionType::Deposit,
                1,
                1,
                Some(dec!(100.0)),
            ))
            .unwrap();

        // Verify initial state after deposit
        {
            let account = engine.accounts().next().unwrap();
            assert_eq!(account.available, dec!(100.0));
            assert_eq!(account.held, dec!(0.0));
            assert_eq!(account.total(), dec!(100.0));
            assert!(!account.locked);
        }

        // Withdraw 75
        engine
            .process_transaction(create_transaction(
                TransactionType::Withdrawal,
                1,
                2,
                Some(dec!(75.0)),
            ))
            .unwrap();

        // Verify state after withdrawal: available = 25, held = 0, total = 25
        {
            let account = engine.accounts().next().unwrap();
            assert_eq!(account.available, dec!(25.0));
            assert_eq!(account.held, dec!(0.0));
            assert_eq!(account.total(), dec!(25.0));
            assert!(!account.locked);
        }

        // Dispute the original deposit
        engine
            .process_transaction(create_transaction(TransactionType::Dispute, 1, 1, None))
            .unwrap();

        // Verify state after dispute: available = -75, held = 100, total = 25
        {
            let account = engine.accounts().next().unwrap();
            assert_eq!(account.available, dec!(-75.0));
            assert_eq!(account.held, dec!(100.0));
            assert_eq!(account.total(), dec!(25.0));
            assert!(!account.locked);
        }

        // Chargeback the deposit
        engine
            .process_transaction(create_transaction(TransactionType::Chargeback, 1, 1, None))
            .unwrap();

        // Verify final state: available = -75, held = 0, total = -75, locked = true
        let account = engine.accounts().next().unwrap();
        assert_eq!(account.available, dec!(-75.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total(), dec!(-75.0));
        assert!(account.locked);
    }
}
