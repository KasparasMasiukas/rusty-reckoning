use std::env;
use std::error::Error;
use std::process;

use rusty_reckoning::csv_utils::read_csv;
use rusty_reckoning::dto;

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
    let transactions_iter = read_csv::<dto::TransactionDto, _>(&args[1])?;
    // TODO: Process transactions
    for transaction in transactions_iter {
        println!("{:?}", transaction);
    }
    Ok(())
}
