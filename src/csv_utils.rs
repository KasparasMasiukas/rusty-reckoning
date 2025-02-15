//! CSV serialization and deserialization utilities.
//!
//! Provides generic functions for reading and writing CSV data.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::Write;
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

/// Writes an iterator of records to a CSV writer.
/// Each record must implement Serialize.
pub fn write_csv<T, W>(writer: W, records: impl Iterator<Item = T>) -> csv::Result<()>
where
    T: Serialize,
    W: Write,
{
    let mut wtr = csv::Writer::from_writer(writer);
    for record in records {
        wtr.serialize(record)?;
    }
    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dto::AccountRow, dto::Transaction, TransactionType};
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

    #[test]
    fn test_write_csv() -> csv::Result<()> {
        let accounts = vec![
            AccountRow {
                client: 1,
                available: dec!(1.5),
                held: dec!(0.0),
                total: dec!(1.5),
                locked: false,
            },
            AccountRow {
                client: 2,
                available: dec!(2.0),
                held: dec!(3.1234),
                total: dec!(5.1234),
                locked: true,
            },
            AccountRow {
                client: 3,
                available: dec!(0.0),
                held: dec!(0.0),
                total: dec!(0.0),
                locked: false,
            },
            // Test rounding behavior
            AccountRow {
                client: 4,
                available: dec!(1.23456),
                held: dec!(2.34567),
                total: dec!(3.58009),
                locked: false,
            },
        ];

        let mut output = vec![];
        write_csv(&mut output, accounts.into_iter())?;

        let csv_string = String::from_utf8(output).unwrap();
        let expected = "\
client,available,held,total,locked
1,1.5,0.0,1.5,false
2,2.0,3.1234,5.1234,true
3,0.0,0.0,0.0,false
4,1.2345,2.3456,3.5800,false
";

        assert_eq!(csv_string, expected);
        Ok(())
    }
}
