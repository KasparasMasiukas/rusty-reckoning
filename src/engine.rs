use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::{TransactionDto, TransactionType};

#[derive(Debug)]
pub enum Error {
    AccountLocked,
    DuplicateTransaction,
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

    /// Checks that an account is not locked. This should be called early to prevent
    /// processing transactions for locked accounts.
    fn check_account_lock(&self, client: u16) -> Result<(), Error> {
        if let Some(account) = self.accounts.get(&client) {
            if account.locked {
                return Err(Error::AccountLocked);
            }
        }
        Ok(())
    }

    /// Gets a mutable account entry, or creates one if it doesn't exist.
    fn get_mut_account(&mut self, client: u16) -> &mut Account {
        self.accounts.entry(client).or_insert_with(|| Account {
            id: client,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        })
    }

    /// Gets a stored desposit entry if it exists, and validates that it belongs to the client.
    /// Returns a mutable reference to the deposit, or an error if the deposit does not exist or
    /// belongs to a different client.
    fn get_mut_deposit(&mut self, client: u16, tx: u32) -> Result<&mut StoredDeposit, Error> {
        let deposit = self
            .deposits
            .get_mut(&tx)
            .ok_or(Error::TransactionNotFound)?;
        if deposit.client != client {
            return Err(Error::InvalidTransaction);
        }
        Ok(deposit)
    }

    /// Parses a transaction from a TransactionDto to a domain object.
    /// Returns an error if the transaction is invalid.
    fn parse_transaction(&mut self, transaction: TransactionDto) -> Result<Transaction, Error> {
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
        self.check_account_lock(transaction_dto.client)?;
        let transaction = self.parse_transaction(transaction_dto)?;

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

        let account = self.get_mut_account(client);
        account.available += amount;
        Ok(())
    }

    fn process_withdrawal(&mut self, client: u16, tx: u32, amount: Decimal) -> Result<(), Error> {
        let account = self.get_mut_account(client);
        if account.available < amount {
            return Err(Error::InsufficientFunds);
        }
        account.available -= amount;
        Ok(())
    }

    fn process_dispute(&mut self, client: u16, tx: u32) -> Result<(), Error> {
        let deposit = self.get_mut_deposit(client, tx)?;
        if deposit.disputed {
            return Err(Error::TransactionAlreadyDisputed);
        }
        deposit.disputed = true;

        let amount = deposit.amount;
        let account = self.get_mut_account(client);
        account.held += amount;
        account.available -= amount;
        Ok(())
    }

    fn process_resolve(&mut self, client: u16, tx: u32) -> Result<(), Error> {
        let deposit = self.get_mut_deposit(client, tx)?;
        if !deposit.disputed {
            return Err(Error::TransactionNotDisputed);
        }
        deposit.disputed = false;

        let amount = deposit.amount;
        let account = self.get_mut_account(client);
        account.held -= amount;
        account.available += amount;
        Ok(())
    }

    fn process_chargeback(&mut self, client: u16, tx: u32) -> Result<(), Error> {
        let deposit = self.get_mut_deposit(client, tx)?;
        if !deposit.disputed {
            return Err(Error::TransactionNotDisputed);
        }
        deposit.disputed = false;

        let amount = deposit.amount;
        let account = self.get_mut_account(client);
        account.held -= amount;
        account.locked = true;
        Ok(())
    }
}
