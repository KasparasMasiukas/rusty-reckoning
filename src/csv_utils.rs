use serde::de::DeserializeOwned;
use std::path::Path;

/// Creates an iterator that reads CSV records from a file.
/// Each record is deserialized into type T.
pub fn read_csv<T, P>(path: P) -> csv::Result<impl Iterator<Item = csv::Result<T>>>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    Ok(csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)?
        .into_deserialize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dto::Transaction, TransactionType};
    use rust_decimal_macros::dec;

    #[test]
    fn test_read_csv() -> csv::Result<()> {
        let transactions: Vec<Transaction> =
            read_csv("data/example_input.csv")?.collect::<Result<_, _>>()?;

        let expected_transactions = vec![
            Transaction {
                tx_type: TransactionType::Deposit,
                client: 1,
                tx: 1,
                amount: Some(dec!(1.0)),
            },
            Transaction {
                tx_type: TransactionType::Deposit,
                client: 2,
                tx: 2,
                amount: Some(dec!(2.0)),
            },
            Transaction {
                tx_type: TransactionType::Deposit,
                client: 1,
                tx: 3,
                amount: Some(dec!(2.0)),
            },
            Transaction {
                tx_type: TransactionType::Withdrawal,
                client: 1,
                tx: 4,
                amount: Some(dec!(1.5)),
            },
            Transaction {
                tx_type: TransactionType::Withdrawal,
                client: 2,
                tx: 5,
                amount: Some(dec!(3.0)),
            },
        ];
        assert_eq!(transactions, expected_transactions);

        Ok(())
    }
}
