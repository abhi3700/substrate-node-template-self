# Counter Pallet

## Overview

The Counter pallet has these dispatchables:

- `set`
- `increment`
- `decrement`
- `reset`
- `kill_storage`

## Build

Check if the dependencies are working properly:

```sh
$ cargo check -p node-template-runtime
```

Build the runtime's WASM binary with the following command:

```sh
$ cargo build --release
```

## Test

To run all the tests in a pallet:

```sh
$ cargo test --package pallet-counter
```

---

To run the individual test:

```sh
# example
$ cargo test --package pallet-counter --lib -- tests::succeeds_when_value_set_as_non_zero
```

Although there is a button shown above the test function to run in individual test in VSCode.

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
