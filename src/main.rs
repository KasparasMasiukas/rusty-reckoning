use std::env;
use std::error::Error;
use std::process;

use rusty_reckoning::run;

fn main() {
    if let Err(err) = run_app() {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

fn run_app() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err("Usage: cargo run -- transactions.csv".into());
    }

    run(&args[1], std::io::stdout())
}
