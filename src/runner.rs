use std::error::Error;
use std::io::Write;
use std::path::Path;

use crate::{
    csv_utils::{read_csv_into_iter, write_csv},
    dto::{AccountRow, Transaction},
    Engine,
};

/// Runs the payment engine on the given input file and writes results to the provided writer.
///
/// # Arguments
/// * `input_path` - Path to the input CSV file containing transactions
/// * `writer` - Where to write the account balances (e.g. stdout)
///
/// # Errors
/// Returns an error if:
/// * The input file cannot be read
/// * The CSV is malformed
/// * Writing to the output fails
pub fn run<P, W>(input_path: P, writer: W) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
    W: Write,
{
    let mut engine = Engine::new();

    let transactions_iter = read_csv_into_iter::<Transaction, _>(input_path)?;
    for transaction in transactions_iter {
        // CSV parsing errors are critical - propagate them
        let transaction = transaction?;
        // Transaction processing errors should be ignored per spec
        let _ = engine.process_transaction(transaction);
    }

    // Sort accounts by client ID for deterministic output
    let mut accounts: Vec<_> = engine.accounts().map(AccountRow::from).collect();
    accounts.sort_by_key(|row| row.client);

    // Write account balances to the provided writer
    write_csv(writer, accounts.into_iter())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_input() -> Result<(), Box<dyn Error>> {
        let mut output = Vec::new();
        run("data/example_input.csv", &mut output)?;

        let expected = br#"client,available,held,total,locked
1,1.5,0,1.5,false
2,2,0,2,false
"#;
        assert_eq!(output, expected);
        Ok(())
    }
}
