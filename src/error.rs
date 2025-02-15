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
