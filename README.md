# Rusty Reckoning ⚖️🏴‍☠️
[![Tests](https://github.com/KasparasMasiukas/rusty-reckoning/actions/workflows/tests.yml/badge.svg)](https://github.com/KasparasMasiukas/rusty-reckoning/actions/workflows/tests.yml)
[![Coverage Status](https://coveralls.io/repos/github/KasparasMasiukas/rusty-reckoning/badge.svg?branch=master)](https://coveralls.io/github/KasparasMasiukas/rusty-reckoning?branch=master)
[![Security Audit](https://github.com/KasparasMasiukas/rusty-reckoning/actions/workflows/security.yml/badge.svg)](https://github.com/KasparasMasiukas/rusty-reckoning/actions/workflows/security.yml)

Settling disputes and ensuring fair trade on the high seas. 🌊

### Running
Run the engine with one of the example data files provided, e.g.:
```
cargo run -- data/10_clients.csv
```
The output rows are sorted by client id for easier predictable testing.

### Testing
The crate includes a comprehensive test suite. To run it:
```
cargo test
```
Or, for 10x faster execution, run:
```
cargo test --release
```

Additionally, the `examples/generator.rs` can be used to generate a CSV file with transactions for a given number of clients:
```
cargo run --example generator 100 > data/100_clients.csv
```

The generator script is deterministic, and allows for easy verification of the engine's output correctness (see **Maths** in `examples/generator.rs` for details). A couple of generated files have been included in the repo for testing:
* `data/10_clients.csv` - 10 clients, 1000 total transactions
* `data/10K_clients.csv` - 10,000 clients, 1M total transactions

### Assumptions
Note: Each assumption is covered by a test.
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

### Benchmarks
The crate includes a benchmark for the engine's throughput, measured with `criterion`, using the 1M transactions input file.
The benchmark measures the time including file streaming, CSV parsing, transaction processing, and CSV serialization for the output.
It does not include the time writing to stdout (a `NoopWriter` is used for the benchmark).

To run the benchmark:
```
cargo bench
```

Results:
```
Insert results here
```

#### System Information
* CPU Model: Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz
* Architecture: x86_64
* Total RAM: 31Gi
* L3 Cache: 12 MiB
