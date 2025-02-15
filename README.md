# Rusty Reckoning ⚖️🏴‍☠️
[![Tests](https://github.com/KasparasMasiukas/rusty-reckoning/actions/workflows/tests.yml/badge.svg)](https://github.com/KasparasMasiukas/rusty-reckoning/actions/workflows/tests.yml)
[![Coverage Status](https://coveralls.io/repos/github/KasparasMasiukas/rusty-reckoning/badge.svg?branch=master)](https://coveralls.io/github/KasparasMasiukas/rusty-reckoning?branch=master)
[![Security Audit](https://github.com/KasparasMasiukas/rusty-reckoning/actions/workflows/security.yml/badge.svg)](https://github.com/KasparasMasiukas/rusty-reckoning/actions/workflows/security.yml)

Settling disputes and ensuring fair trade on the high seas. 

### Running
```
cargo run -- example_input.csv
```

### Assumptions
* When parsing amounts, we round down to 4 decimal places. E.g. input amount 0.123499999 will be parsed as 0.1234.
* Only deposits can be disputed.
    * This matches the spec, saying that "available funds should decrease" and "held funds should increase" - this would not make sense if withdrawals could be disputed.
    * Additionally, it makes sense logically - if a client successfully withdraws funds, disputing it would be meaningless.
    * Therefore, to support disputes in a memory-efficient way, in main memory we only store transactions that are successful deposits.
* Deposit and withdrawal transaction amounts must be positive (>0), otherwise the transaction is rejected.
* Disputes may cause the account's available funds to go to negative.
    * E.g. a client deposits $100, withdraws $50 (available = $50), then disputes the deposit. Result: available = $-50, held = $100. Then if chargeback occurs, the client's account will be locked with $-50 total funds.
* Once an account is locked, no further transactions are processed for that account.
* A new client record would only be created as part of their first deposit transaction.
    * A withdrawal attempt from a non-existent client will be rejected without creating a record.