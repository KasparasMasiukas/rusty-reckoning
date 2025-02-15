use std::env;
use std::error::Error;
use std::process;

use rusty_reckoning::{read_csv, write_csv, AccountRow, Engine, Transaction};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err("Usage: cargo run -- transactions.csv".into());
    }
    let mut engine = Engine::new();

    let transactions_iter = read_csv::<Transaction, _>(&args[1])?;
    for transaction in transactions_iter.flatten() {
        let _ = engine.process_transaction(transaction); // TODO: Handle errors
    }

    // Write account balances to stdout
    write_csv(
        std::io::stdout(),
        engine.accounts().map(AccountRow::from),
    )?;
    Ok(())
}
