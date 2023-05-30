# Flipper Pallet

## Overview

The Flipper pallet has these dispatchables:

- `set_value`
- `flip_value`

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
$ cargo test --package pallet-hello
```

---

To run the individual test:

```sh
# example
$ cargo test --package pallet-hello --lib -- tests::fails_for_wish_start_w_hello
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

## References

- [rust-crewmates flipper tutorial](https://github.com/rusty-crewmates/substrate-tutorials/tree/main/exercises/ex00-testing)
