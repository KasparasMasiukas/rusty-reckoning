use std::env;
use std::error::Error;
use std::process;

use rusty_reckoning::{dto, read_csv, Engine};

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
    let transactions_iter = read_csv::<dto::Transaction, _>(&args[1])?;
    // TODO: Process transactions
    for transaction in transactions_iter.flatten() {
        let _ = engine.process_transaction(transaction); // TODO: Handle errors
    }
    Ok(())
}
