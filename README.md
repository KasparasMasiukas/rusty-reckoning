# Rusty Reckoning ⚖️🏴‍☠️
[![Tests](https://github.com/KasparasMasiukas/rusty-reckoning/actions/workflows/tests.yml/badge.svg)](https://github.com/KasparasMasiukas/rusty-reckoning/actions/workflows/tests.yml)
[![Coverage Status](https://coveralls.io/repos/github/KasparasMasiukas/rusty-reckoning/badge.svg?branch=master)](https://coveralls.io/github/KasparasMasiukas/rusty-reckoning?branch=master)
[![Security Audit](https://github.com/KasparasMasiukas/rusty-reckoning/actions/workflows/security.yml/badge.svg)](https://github.com/KasparasMasiukas/rusty-reckoning/actions/workflows/security.yml)

Settling disputes and ensuring fair trade on the high seas. 🌊

### Running
Run the engine synchronously with one of the example data files provided, e.g.:
```
cargo run -- data/10_clients.csv
```
The output rows are sorted by client id for easier predictable testing.

An example has been added for running the engine asynchronously using tokio runtime:
```
cargo run --example async_main data/10_clients.csv
```
However, due to the sequential nature of processing, benchmarks show that the synchronous single-threaded implementation is faster.

### Testing
The crate includes a comprehensive test suite. To run it:
```
cargo test
```
Or, for 10x faster execution, run:
```
cargo test --release
```

Additionally, the `examples/generator.rs` can be used to generate a CSV file with transactions for a given number of clients, e.g.:
```
cargo run --example generator 100 > data/100_clients.csv
```

The generator script is deterministic, and allows for easy verification of the engine's output correctness (see **Maths** in `examples/generator.rs` for details). A couple of generated files have been included in the repo for testing:
* `data/10_clients.csv` - 10 clients, 1000 total transactions
* `data/10K_clients.csv` - 10,000 clients, 1M total transactions

Test file composition by transaction type:
* 70% deposits
* 20.5% withdrawals
* 5% disputes
* 4% resolves
* 0.5% chargebacks

### Assumptions
Note: Each assumption is covered by a test. Other more obvious assumptions/edge cases are also tested (see tests in `src/engine.rs`).
* Deposit and withdrawal transaction amounts must be positive (>0), otherwise the transaction is rejected. (`test_deposit_non_positive_amount`, `test_withdrawal_non_positive_amount`)
* When parsing amounts, we round down to 4 decimal places. E.g. input amount 0.123499999 will be parsed as 0.1234. (`test_rounds_to_4_decimal_places`)
* Only deposits can be disputed. (`test_dispute_resolve_chargeback_only_for_deposits`)
    * This matches the spec, saying that "available funds should decrease" and "held funds should increase" - this would not make sense if withdrawals could be disputed.
    * Additionally, it makes sense logically - if a client successfully withdraws funds, disputing it would be meaningless.
* Disputes may cause the account's available funds to go to negative. (`test_chargeback_results_in_negative_balance`)
    * E.g. a client deposits \$100, withdraws \$50 (available = \$50), then disputes the deposit. Result: available = \$-50, held = \$100. Then if chargeback occurs, the client's account will be locked with \$-50 total funds.
* Once an account is locked, no further transactions are processed for that account. (`test_locked_account_rejects_transactions`)
* A new client record can only be created as part of their first deposit transaction. 
    * A withdrawal attempt from a non-existent client will be rejected without creating a record. (`test_withdrawal_from_nonexistent_account`)
    * Same with dispute/resolve/chargeback transactions - they would be rejected with reason `TransactionNotFound` before any client record is created. (`test_dispute_resolve_chargeback_nonexistent_account`)
* If a transaction is disputed and then resolved, the same transaction **can** be disputed again. (`test_redispute_after_resolve`)
    * There is no mention of this not being allowed in the spec, so we keep the logic simple and allow it.

Additionally:
* Transactions are streamed from the input file, and processed immediately as they arrive.
* No unsafe Rust is used; all operations are memory-safe.

### Memory Requirements
While the system is designed to be memory-efficient, it assumes the machine has enough heap space to store the minimum transaction-related data necessary to support all operations.

The data structures used are:
* `HashMap<u32, StoredDeposit>` - to store all successful deposits, used to support dispute/resolve/chargeback transactions.
    * `StoredDeposit` is represented by 20 bytes.
* `HashSet<u32>` - to store all processed transactions, used to prevent duplicates.
* `HashMap<u16, Account>` - to store all account states.
    * Because of the small u16 space, the memory footprint is irrelevant.

Worst case memory requirements are tied to the number of possible unique transactions to fill the u32 space (4.29B). In the worst case, if all 4.29B transactions are deposits, the memory usage would be:
```
4.29B * (4 + 20 + 4) = ~120GB (excluding HashMap/HashSet overhead)
```

This is well within the limits of modern cloud compute. For reference, at the time of writing, AWS (US East) `x2gd.4xlarge` (256 GB RAM) has an on-demand hourly rate of $1.336, translating to $11.7K USD per annum.

### Benchmarks
The crate includes a benchmark for the engine's throughput, measured with `criterion`, using the 1M transactions input file.
The benchmark measures the time including file streaming, CSV parsing, transaction processing, and CSV serialization for the output.
It does not include the time writing to stdout (a `NoopWriter` is used for the benchmark).

To run the benchmark:
```
cargo bench
```

The system achieves a throughput of **1.6M tx / sec** on the testing machine.

Results:
```
throughput/process_10K_clients_1M_transactions
                        time:   [610.27 ms 612.29 ms 614.43 ms]
                        thrpt:  [1.6275 Melem/s 1.6332 Melem/s 1.6386 Melem/s]
```

#### System Information
* CPU Model: Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz
* Architecture: x86_64
* Total RAM: 31Gi
* L3 Cache: 12 MiB
* Disk: Samsung SSD 970 EVO Plus 1TB (Seq. Read Speed: Up to 3,500 MB/s)
