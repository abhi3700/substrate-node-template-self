# Bank Pallet

## Overview

Anyone can open FD (Fixed Deposit) by reserving some amount of currency.

During the FD period, the reserved amount cannot be used that's why need to be freed from the `free_balance`.
In order to receive interest, FD can only be closed after the `MinFDPeriod` is elapsed, else the reserved amount is returned
to the user without any interest as per the premature withdrawal facility. The penalty (0.5-1%) is stored & set by the root origin.

But, if the FD is closed after `MinFDPeriod`, then the reserved amount is returned to the user with
some interest. The interest is stored & set by the root origin.

TODO:

- [ ] We can also add the functionality of auto_maturity of FDs using hooks.
- [ ] After every few blocks, some balance is transferred to the TREASURY account.
  - L0 chain's inflation is transferred to the TREASURY account.

The interest comes from a treasury 💎 account which is funded by the root origin.

NOTE: The runtime must include the `Balances` pallet to handle the accounts and balances for your chain.

## Build

Check if the dependencies are working properly:

```sh
$ cargo check -p node-template-runtime
```

Build the runtime's WASM binary with the following command:

```sh
$ cargo build -r
```

## Test

To run all the tests in a pallet:

```sh
$ cargo test -p pallet-bank
```

---

To run the individual test:

```sh
# example
$ cargo test -p pallet-bank --lib -- tests::it_works_for_default_value
```

Although there is a button shown above to run individual test in VSCode.

## Benchmark

<!-- TODO: -->

## Run

Run a relaychain node (w/o debug mode):

```sh
$ ./target/release/node-template --dev
```

In debug mode, run a relaychain node:

```sh
$ RUST_LOG=runtime=debug ./target/release/node-template --dev
```
