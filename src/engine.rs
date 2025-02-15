use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::{TransactionDto, TransactionType};

#[derive(Debug)]
pub enum Error {
    AccountLocked,
    InsufficientFunds,
    InvalidTransaction,
    TransactionAlreadyDisputed,
    TransactionNotDisputed,
    TransactionNotFound,
}

#[derive(Debug)]
enum Transaction {
    Deposit {
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    Withdrawal {
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    Dispute {
        client: u16,
        tx: u32,
    },
    Resolve {
        client: u16,
        tx: u32,
    },
    Chargeback {
        client: u16,
        tx: u32,
    },
}

#[derive(Debug)]
struct Account {
    id: u16,
    available: Decimal,
    held: Decimal,
    locked: bool,
}

impl Account {
    pub fn total(&self) -> Decimal {
        self.available + self.held
    }
}

struct StoredDeposit {
    client: u16,
    amount: Decimal,
    disputed: bool,
}

pub struct Engine {
    accounts: HashMap<u16, Account>,
    /// Deposits can be disputed, so this is a map of all successful deposits
    deposits: HashMap<u32, StoredDeposit>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            deposits: HashMap::new(),
        }
    }

    // TODO: this could return a tuple of (&mut account, transaction)
    fn validate_and_parse_transaction(
        &mut self,
        transaction: TransactionDto,
    ) -> Result<Transaction, Error> {
        // Prepare and validate the Account
        let account = self
            .accounts
            .entry(transaction.client)
            .or_insert_with(|| Account {
                id: transaction.client,
                available: Decimal::ZERO,
                held: Decimal::ZERO,
                locked: false,
            });
        if account.locked {
            return Err(Error::AccountLocked);
        }

        // Validate and parse the Transaction
        match transaction.tx_type {
            TransactionType::Deposit => Ok(Transaction::Deposit {
                client: transaction.client,
                tx: transaction.tx,
                amount: transaction.amount.ok_or(Error::InvalidTransaction)?,
            }),
            TransactionType::Withdrawal => Ok(Transaction::Withdrawal {
                client: transaction.client,
                tx: transaction.tx,
                amount: transaction.amount.ok_or(Error::InvalidTransaction)?,
            }),
            TransactionType::Dispute => Ok(Transaction::Dispute {
                client: transaction.client,
                tx: transaction.tx,
            }),
            TransactionType::Resolve => Ok(Transaction::Resolve {
                client: transaction.client,
                tx: transaction.tx,
            }),
            TransactionType::Chargeback => Ok(Transaction::Chargeback {
                client: transaction.client,
                tx: transaction.tx,
            }),
        }
    }

    pub fn process_transaction(&mut self, transaction_dto: TransactionDto) -> Result<(), Error> {
        // Check if account is locked before processing any transaction
        let transaction = self.validate_and_parse_transaction(transaction_dto)?;

        match transaction {
            Transaction::Deposit { client, tx, amount } => self.process_deposit(client, tx, amount),
            Transaction::Withdrawal { client, tx, amount } => {
                self.process_withdrawal(client, tx, amount)
            }
            Transaction::Dispute { client, tx } => self.process_dispute(client, tx),
            Transaction::Resolve { client, tx } => self.process_resolve(client, tx),
            Transaction::Chargeback { client, tx } => self.process_chargeback(client, tx),
        }
    }

    fn process_deposit(&mut self, client: u16, tx: u32, amount: Decimal) -> Result<(), Error> {
        // For now, we assume pre-validated transaction, account has already been created/validated. TODO: revisit
        let account = self.accounts.get_mut(&client).unwrap();
        self.deposits.insert(
            tx,
            StoredDeposit {
                client,
                amount,
                disputed: false,
            },
        );
        account.available += amount;
        Ok(())
    }

    fn process_withdrawal(&mut self, client: u16, tx: u32, amount: Decimal) -> Result<(), Error> {
        let account = self.accounts.get_mut(&client).unwrap();
        if account.available < amount {
            return Err(Error::InsufficientFunds);
        }
        account.available -= amount;
        Ok(())
    }

    fn process_dispute(&mut self, client: u16, tx: u32) -> Result<(), Error> {
        let deposit = self
            .deposits
            .get_mut(&tx)
            .ok_or(Error::TransactionNotFound)?;
        if deposit.client != client {
            return Err(Error::InvalidTransaction);
        }
        if deposit.disputed {
            return Err(Error::TransactionAlreadyDisputed);
        }
        deposit.disputed = true;
        let account = self.accounts.get_mut(&client).unwrap();
        account.held += deposit.amount;
        account.available -= deposit.amount;
        Ok(())
    }

    fn process_resolve(&mut self, client: u16, tx: u32) -> Result<(), Error> {
        let deposit = self
            .deposits
            .get_mut(&tx)
            .ok_or(Error::TransactionNotFound)?;
        if deposit.client != client {
            return Err(Error::InvalidTransaction);
        }
        if !deposit.disputed {
            return Err(Error::TransactionNotDisputed);
        }
        deposit.disputed = false;
        let account = self.accounts.get_mut(&client).unwrap();
        account.held -= deposit.amount;
        account.available += deposit.amount;
        Ok(())
    }

    fn process_chargeback(&mut self, client: u16, tx: u32) -> Result<(), Error> {
        let deposit = self
            .deposits
            .get_mut(&tx)
            .ok_or(Error::TransactionNotFound)?;
        if deposit.client != client {
            return Err(Error::InvalidTransaction);
        }
        if !deposit.disputed {
            return Err(Error::TransactionNotDisputed);
        }
        deposit.disputed = false;
        let account = self.accounts.get_mut(&client).unwrap();
        account.held -= deposit.amount;
        account.locked = true;
        Ok(())
    }
}
