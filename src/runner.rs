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

        let expected = "client,available,held,total,locked
1,1.5,0,1.5,false
2,2,0,2,false
";
        assert_eq!(String::from_utf8(output)?, expected);
        Ok(())
    }

    #[test]
    fn test_10_clients() -> Result<(), Box<dyn Error>> {
        let mut output = Vec::new();
        run("data/10_clients.csv", &mut output)?;

        let expected = "client,available,held,total,locked
1,270,10,280,false
2,580,0,580,true
3,810,30,840,false
4,1160,0,1160,true
5,1350,50,1400,false
6,1740,0,1740,true
7,1890,70,1960,false
8,2320,0,2320,true
9,2430,90,2520,false
10,2900,0,2900,true
";
        assert_eq!(String::from_utf8(output)?, expected);
        Ok(())
    }

    #[test]
    fn test_10000_clients() -> Result<(), Box<dyn Error>> {
        let mut output = Vec::new();
        run("data/10K_clients.csv", &mut output)?;

        // Build expected CSV output dynamically (see examples/generator.rs for maths).
        let mut expected = String::from("client,available,held,total,locked\n");
        for i in 1..=10000 {
            if i % 2 == 1 {
                // Odd client: available = 270*i, held = 10*i, total = 280*i, locked false.
                expected.push_str(&format!("{},{},{},{},false\n", i, 270 * i, 10 * i, 280 * i));
            } else {
                // Even client: available = 290*i, held = 0, total = 290*i, locked true.
                expected.push_str(&format!("{},{},{},{},true\n", i, 290 * i, 0, 290 * i));
            }
        }

        assert_eq!(String::from_utf8(output)?, expected);
        Ok(())
    }
}
