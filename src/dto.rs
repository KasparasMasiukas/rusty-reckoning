use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    #[serde(deserialize_with = "deserialize_decimal_4dp")]
    pub amount: Option<Decimal>,
}

fn deserialize_decimal_4dp<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<Decimal>::deserialize(deserializer)
        .map(|opt_dec| opt_dec.map(|dec| dec.round_dp_with_strategy(4, RoundingStrategy::ToZero)))
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
}
