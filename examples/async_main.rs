use std::env;
use std::error::Error;
use std::process;

use rusty_reckoning::run_async;

#[tokio::main]
async fn main() {
    if let Err(err) = run_app().await {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

async fn run_app() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err("Usage: cargo run -- transactions.csv".into());
    }

    run_async(args[1].clone(), std::io::stdout()).await
}
