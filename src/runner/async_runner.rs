use std::error::Error;
use std::io::Write;
use std::path::Path;

use crate::{
    csv_utils::write_csv,
    dto::{AccountRow, Transaction},
    Engine,
};

use csv_async::{AsyncReaderBuilder, Error as CsvError, Trim};
use tokio::fs::File;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

const BUFFER_SIZE: usize = 1024;

type Result<T, E = Box<dyn Error + Send + Sync>> = std::result::Result<T, E>;

/// Runs the payment engine async on the given input file and writes results to the provided writer.
/// Spawns two tasks:
/// * CSV reader - streams transactions from the input file, deserializes them and sends them to the processor via channel.
/// * Processor - receives transactions from the channel and processes them until the channel is closed.
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
pub async fn run<P, W>(input_path: P, writer: W) -> Result<()>
where
    P: AsRef<Path>,
    W: Write,
{
    // Create channel for passing transactions from reader to processor
    let (tx, rx) = mpsc::channel(BUFFER_SIZE);
    let input_path = input_path.as_ref().to_owned();

    let reader_handle = tokio::spawn(read_transactions(input_path, tx));
    let processor_handle = tokio::spawn(process_transactions(rx));

    // Wait for reader to finish and propagate any errors
    reader_handle.await??;

    // Get final engine state
    let engine = processor_handle.await?;

    // Sort accounts by client ID for deterministic output
    let mut accounts: Vec<_> = engine.accounts().map(AccountRow::from).collect();
    accounts.sort_by_key(|row| row.client);

    // Write account balances to the provided writer
    write_csv(writer, accounts.into_iter())?;
    Ok(())
}

/// Reads and deserializes transactions from a CSV file.
/// Returns them through the provided channel.
async fn read_transactions(
    input_path: impl AsRef<Path> + Send,
    tx: mpsc::Sender<Transaction>,
) -> Result<(), CsvError> {
    let file = File::open(input_path).await?;
    let mut csv_reader = AsyncReaderBuilder::new()
        .has_headers(true)
        .trim(Trim::All)
        .create_deserializer(file);

    let mut records = csv_reader.deserialize::<Transaction>();
    while let Some(result) = records.next().await {
        match result {
            Ok(transaction) => {
                if tx.send(transaction).await.is_err() {
                    // Receiver dropped, exit gracefully
                    break;
                }
            }
            // CSV parsing errors are critical - propagate them
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Processes transactions received through the channel.
/// Returns the final engine state once the channel is closed by the reader.
async fn process_transactions(mut rx: mpsc::Receiver<Transaction>) -> Engine {
    let mut engine = Engine::new();
    while let Some(transaction) = rx.recv().await {
        // Transaction processing errors should be ignored per spec
        let _ = engine.process_transaction(transaction);
    }
    engine
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_example_input() -> Result<()> {
        let mut output = Vec::new();
        run("data/example_input.csv", &mut output).await?;

        let expected = "client,available,held,total,locked
1,1.5,0,1.5,false
2,2,0,2,false
";
        assert_eq!(String::from_utf8(output)?, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_10_clients() -> Result<()> {
        let mut output = Vec::new();
        run("data/10_clients.csv", &mut output).await?;

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

    #[tokio::test]
    async fn test_10000_clients() -> Result<()> {
        let mut output = Vec::new();
        run("data/10K_clients.csv", &mut output).await?;

        // Dynamically build the expected CSV output.
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
