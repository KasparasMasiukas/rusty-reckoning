//! Data Transfer Objects (DTOs) for the payment processing system.
//!
//! This module contains the structs and enums used for:
//! - Parsing input transactions from CSV ([`Transaction`], [`TransactionType`])
//! - Serializing account state to CSV output ([`AccountRow`])
//!
//! It also includes serialization/deserialization helpers for handling decimal numbers
//! with 4 decimal places precision.

use crate::stores::Account;
use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy;
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    #[serde(deserialize_with = "deserialize_decimal_4dp")]
    pub amount: Option<Decimal>,
}

#[derive(Debug, Serialize)]
pub struct AccountRow {
    pub client: u16,
    #[serde(serialize_with = "serialize_decimal_4dp")]
    pub available: Decimal,
    #[serde(serialize_with = "serialize_decimal_4dp")]
    pub held: Decimal,
    #[serde(serialize_with = "serialize_decimal_4dp")]
    pub total: Decimal,
    pub locked: bool,
}

impl From<&Account> for AccountRow {
    fn from(account: &Account) -> Self {
        AccountRow {
            client: account.id,
            available: account.available,
            held: account.held,
            total: account.total(),
            locked: account.locked,
        }
    }
}

pub fn deserialize_decimal_4dp<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<Decimal>::deserialize(deserializer)
        .map(|opt_dec| opt_dec.map(|dec| dec.round_dp_with_strategy(4, RoundingStrategy::ToZero)))
}

pub fn serialize_decimal_4dp<S>(decimal: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded = decimal.round_dp_with_strategy(4, RoundingStrategy::ToZero);
    serializer.serialize_str(&rounded.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn parse_csv_row(row: &str) -> Result<Transaction, csv::Error> {
        let data_with_header = format!("type,client,tx,amount\n{}", row);
        let mut reader = csv::Reader::from_reader(data_with_header.as_bytes());
        reader.deserialize().next().unwrap()
    }

    #[test]
    fn test_parse_deposit() {
        assert_eq!(
            parse_csv_row("deposit,1,1,0.1234").unwrap(),
            Transaction {
                tx_type: TransactionType::Deposit,
                client: 1,
                tx: 1,
                amount: Some(dec!(0.1234)),
            }
        );
    }

    #[test]
    fn test_parse_withdrawal() {
        assert_eq!(
            parse_csv_row("withdrawal,2,2,1.5").unwrap(),
            Transaction {
                tx_type: TransactionType::Withdrawal,
                client: 2,
                tx: 2,
                amount: Some(dec!(1.5)),
            }
        );
    }

    #[test]
    fn test_parse_dispute() {
        assert_eq!(
            parse_csv_row("dispute,1,1,").unwrap(),
            Transaction {
                tx_type: TransactionType::Dispute,
                client: 1,
                tx: 1,
                amount: None,
            }
        );
    }

    #[test]
    fn test_parse_resolve() {
        assert_eq!(
            parse_csv_row("resolve,1,1,").unwrap(),
            Transaction {
                tx_type: TransactionType::Resolve,
                client: 1,
                tx: 1,
                amount: None,
            }
        );
    }

    #[test]
    fn test_parse_chargeback() {
        assert_eq!(
            parse_csv_row("chargeback,1,1,").unwrap(),
            Transaction {
                tx_type: TransactionType::Chargeback,
                client: 1,
                tx: 1,
                amount: None,
            }
        );
    }

    #[test]
    fn test_parse_invalid_amount_format() {
        let result = parse_csv_row("deposit,1,1,abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_transaction_type() {
        let result = parse_csv_row("invalid,1,1,1.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_client_id_overflow() {
        let result = parse_csv_row("deposit,65536,1,1.0"); // u16::MAX + 1
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_id_overflow() {
        let result = parse_csv_row("deposit,1,4294967296,1.0"); // u32::MAX + 1
        assert!(result.is_err());
    }

    #[test]
    fn test_max_valid_ids() {
        assert_eq!(
            parse_csv_row(&format!("deposit,{},{},1.0", u16::MAX, u32::MAX)).unwrap(),
            Transaction {
                tx_type: TransactionType::Deposit,
                client: u16::MAX,
                tx: u32::MAX,
                amount: Some(dec!(1.0)),
            }
        );
    }

    #[test]
    fn test_rounds_to_4_decimal_places() {
        assert_eq!(
            parse_csv_row("deposit,1,1,0.12345").unwrap(),
            Transaction {
                tx_type: TransactionType::Deposit,
                client: 1,
                tx: 1,
                amount: Some(dec!(0.1234)), // Rounded down from 0.12345
            }
        );

        assert_eq!(
            parse_csv_row("deposit,1,1,0.123499999").unwrap(),
            Transaction {
                tx_type: TransactionType::Deposit,
                client: 1,
                tx: 1,
                amount: Some(dec!(0.1234)), // Rounded down from 0.123499999
            }
        );
    }

    #[test]
    fn test_account_row_serialization() {
        let row = AccountRow {
            client: 1,
            available: dec!(1.23456),
            held: dec!(2.34567),
            total: dec!(3.58003),
            locked: false,
        };

        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.serialize(&row).unwrap();
        let csv_output = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        assert_eq!(
            csv_output,
            "client,available,held,total,locked\n1,1.2345,2.3456,3.5800,false\n"
        );
    }

    #[test]
    fn test_account_to_account_row_conversion() {
        let test_cases = vec![
            // Basic case with available funds only
            (
                Account {
                    id: 1,
                    available: dec!(100.5),
                    held: dec!(0.0),
                    locked: false,
                },
                AccountRow {
                    client: 1,
                    available: dec!(100.5),
                    held: dec!(0.0),
                    total: dec!(100.5),
                    locked: false,
                },
            ),
            // Case with both available and held funds
            (
                Account {
                    id: 2,
                    available: dec!(50.25),
                    held: dec!(25.25),
                    locked: false,
                },
                AccountRow {
                    client: 2,
                    available: dec!(50.25),
                    held: dec!(25.25),
                    total: dec!(75.50),
                    locked: false,
                },
            ),
            // Locked account case
            (
                Account {
                    id: 3,
                    available: dec!(-50.0),
                    held: dec!(0.0),
                    locked: true,
                },
                AccountRow {
                    client: 3,
                    available: dec!(-50.0),
                    held: dec!(0.0),
                    total: dec!(-50.0),
                    locked: true,
                },
            ),
            // Zero balance case
            (
                Account {
                    id: 4,
                    available: dec!(0.0),
                    held: dec!(0.0),
                    locked: false,
                },
                AccountRow {
                    client: 4,
                    available: dec!(0.0),
                    held: dec!(0.0),
                    total: dec!(0.0),
                    locked: false,
                },
            ),
            // High precision case
            (
                Account {
                    id: 5,
                    available: dec!(100.1234),
                    held: dec!(50.5678),
                    locked: true,
                },
                AccountRow {
                    client: 5,
                    available: dec!(100.1234),
                    held: dec!(50.5678),
                    total: dec!(150.6912),
                    locked: true,
                },
            ),
        ];

        for (account, expected_row) in test_cases {
            let row = AccountRow::from(&account);
            assert_eq!(row.client, expected_row.client);
            assert_eq!(row.available, expected_row.available);
            assert_eq!(row.held, expected_row.held);
            assert_eq!(row.total, expected_row.total);
            assert_eq!(row.locked, expected_row.locked);
        }
    }
}
