//! This example generates a CSV file with a number of transactions (configurable through the constants)
//! for a number of clients supplied as a command-line argument.
//!
//! The CSV file can then be used to test the `rusty-reckoning` crate.
//!
//! Example (100 clients):
//! ```bash
//! cargo run --example generator 100 > data/100_clients.csv
//! ```
//! ### Maths
//! Based on the constants, we can easily derive the ending state of the account for any client.
//!
//! Let:
//! - D = NUM_DEPOSITS, W = NUM_WITHDRAWALS, Q = NUM_DISPUTES, R = NUM_RESOLVES,
//! - A_d = BASE_DEPOSIT_AMOUNT, A_w = BASE_WITHDRAWAL_AMOUNT, and i = client id.
//!
//! **Step 1: Deposits & Withdrawals**  
//! Deposits add D·A_d·i and withdrawals subtract W·A_w·i.  
//! With our constants: 70·10·i – 20·20·i = 700·i – 400·i = 300·i.
//!
//! **Step 2: Disputes & Resolves**  
//! Disputes hold Q·A_d·i and resolves release R·A_d·i, so available decreases by (Q–R)·A_d·i = 10·i.
//! This yields:  
//! • Available_after = 300·i – 10·i = 290·i  
//! • Held_after = 10·i
//!
//! **Step 3: Final Transaction**  
//! - If i is odd (extra withdrawal of A_w·i = 20·i):  
//!   • Available = 290·i – 20·i = 270·i  
//!   • Total = 270·i + 10·i = 280·i  
//!   • Locked = false  
//! - If i is even (chargeback removing A_d·i = 10·i from held):  
//!   • Held = 10·i – 10·i = 0  
//!   • Total = 290·i  
//!   • Locked = true
//!
//! **Final State for client i:**  
//! - **Odd i:** available = 270·i, held = 10·i, total = 280·i, unlocked.  
//! - **Even i:** available = 290·i, held = 0, total = 290·i, locked.
//!
//! If the system is correctly implemented, the ending state for any client in the output CSV should match the maths above.
//!

use csv::Writer;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rusty_reckoning::{Transaction, TransactionType};
use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    // Get command-line arguments
    let args: Vec<String> = env::args().collect();

    // Ensure we have the correct number of arguments
    if args.len() != 2 {
        eprintln!("Usage: cargo run --example generator <num_clients>");
        std::process::exit(1);
    }

    // Parse NUM_CLIENTS from the first argument
    let num_clients: u16 = match args[1].parse() {
        Ok(n) if n > 0 => n,
        _ => {
            eprintln!("Error: <num_clients> must be a positive integer.");
            std::process::exit(1);
        }
    };

    // Configuration constants.
    const NUM_DEPOSITS: usize = 70;
    const NUM_WITHDRAWALS: usize = 20;
    const NUM_DISPUTES: usize = 5;
    const NUM_RESOLVES: usize = 4;
    // Final transaction: 1 extra transaction per client.
    const TOTAL_TX_PER_CLIENT: usize =
        NUM_DEPOSITS + NUM_WITHDRAWALS + NUM_DISPUTES + NUM_RESOLVES + 1;

    // Base amounts; these will be scaled by the client ID.
    const BASE_DEPOSIT_AMOUNT: Decimal = dec!(10.0);
    const BASE_WITHDRAWAL_AMOUNT: Decimal = dec!(20.0);

    // We'll assign new global transaction IDs for deposit and withdrawal transactions.
    // (Dispute, resolve, and chargeback transactions reference deposit tx IDs mathematically.)
    let mut global_tx_counter: u32 = 1;

    let mut wtr = Writer::from_writer(std::io::stdout());

    // Process transactions round by round.
    // In each round, every client produces its next transaction in its internal order.
    for round in 0..TOTAL_TX_PER_CLIENT {
        for client in 1..=num_clients {
            let client_decimal = Decimal::from(client);
            let txn = if round < NUM_DEPOSITS {
                // Deposit rounds: assign a new global transaction ID.
                let tx_id = global_tx_counter;
                global_tx_counter += 1;
                Transaction {
                    tx_type: TransactionType::Deposit,
                    client,
                    tx: tx_id,
                    amount: Some(BASE_DEPOSIT_AMOUNT * client_decimal),
                }
            } else if round < NUM_DEPOSITS + NUM_WITHDRAWALS {
                // Withdrawal rounds: assign a new global transaction ID.
                let tx_id = global_tx_counter;
                global_tx_counter += 1;
                Transaction {
                    tx_type: TransactionType::Withdrawal,
                    client,
                    tx: tx_id,
                    amount: Some(BASE_WITHDRAWAL_AMOUNT * client_decimal),
                }
            } else if round < NUM_DEPOSITS + NUM_WITHDRAWALS + NUM_DISPUTES {
                // Dispute rounds: reference the deposit corresponding to dispute index.
                // The dispute for this client at index i (where i = round - (NUM_DEPOSITS+NUM_WITHDRAWALS))
                // references the deposit produced in round i. That deposit’s global ID was:
                // deposit_global_id = i * NUM_CLIENTS + client.
                let dispute_index = round - (NUM_DEPOSITS + NUM_WITHDRAWALS);
                let deposit_tx_id = (dispute_index as u32) * (num_clients as u32) + client as u32;
                Transaction {
                    tx_type: TransactionType::Dispute,
                    client,
                    tx: deposit_tx_id,
                    amount: None,
                }
            } else if round < NUM_DEPOSITS + NUM_WITHDRAWALS + NUM_DISPUTES + NUM_RESOLVES {
                // Resolve rounds: similar to disputes, reference deposit at index i.
                let resolve_index = round - (NUM_DEPOSITS + NUM_WITHDRAWALS + NUM_DISPUTES);
                let deposit_tx_id = (resolve_index as u32) * (num_clients as u32) + client as u32;
                Transaction {
                    tx_type: TransactionType::Resolve,
                    client,
                    tx: deposit_tx_id,
                    amount: None,
                }
            } else {
                // Final round: if client is even, issue a chargeback; if odd, an extra withdrawal.
                if client % 2 == 0 {
                    // Chargeback references deposit with index NUM_RESOLVES.
                    let deposit_tx_id =
                        (NUM_RESOLVES as u32) * (num_clients as u32) + client as u32;
                    Transaction {
                        tx_type: TransactionType::Chargeback,
                        client,
                        tx: deposit_tx_id,
                        amount: None,
                    }
                } else {
                    // Extra withdrawal: assign a new global transaction ID.
                    let tx_id = global_tx_counter;
                    global_tx_counter += 1;
                    Transaction {
                        tx_type: TransactionType::Withdrawal,
                        client,
                        tx: tx_id,
                        amount: Some(BASE_WITHDRAWAL_AMOUNT * client_decimal),
                    }
                }
            };
            wtr.serialize(txn)?;
        }
    }
    wtr.flush()?;
    Ok(())
}
