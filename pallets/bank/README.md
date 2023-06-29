# Bank Pallet

## Overview

Bank pallet has dispatchables:

- `set_balance`
- `update_balance`

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
