# Pallets

Pallets are the building blocks of the runtime. They are the modules that implement the business logic of the blockchain. They are the equivalent of smart contracts in other blockchains (parachains).

## List of Pallets

> The order is as per the level of complexity of the pallets.

- [Template](./template) - A template pallet to create new pallets.
- [Hello](./hello) - A simple pallet to say hello to the world.
- [Flipper](./flipper) - A simple pallet to flip a boolean value.
- [Counter](./counter) - A simple pallet to count the number of times it is called.
- [🧑🏻‍💻] [Voting](./voting/) - A pallet to vote for a candidate. [Reference](https://docs.soliditylang.org/en/v0.8.17/solidity-by-example.html#voting)

## Add NEW pallet

> including pallet code, test, benchmark

To add a new pallet, you need to follow these:

#### 1. copy the [template](./template/) module & rename it to the name of your pallet at [pallets](./) directory.

#### 2. Then you need to add it to the `Cargo.toml` file of the runtime.

#### 3. Finally, you need to add it to the `construct_runtime!` macro in the `runtime/src/lib.rs` file.

#### 4. Check if the dependencies are working properly for the node runtime:

```sh
$ cargo check -p node-template-runtime
```

> In order to check a pallet individually (at the root of the project):

```sh
$ cargo check -p pallet-hello
```

#### 5. Build the runtime's WASM binary:

```sh
$ cargo build --release
```

#### 6. Write the test cases for your pallet in the `runtime/src/tests.rs` file. Prior to this, add `mock.rs` file for creating a runtime.

```sh
# run all the tests in the runtime
$ cargo test

# run the pallet (package) tests
$ cargo test --package pallet-hello

# run the pallet individual tests
$ cargo test --package pallet-hello --lib -- tests::fails_for_wish_start_w_hello
```

> Although there is a button shown above test function to run individual test in VSCode.

#### 7. Write the benchmarking code for your pallet in the `runtime/src/benchmarks.rs` file. <!-- TODO: -->

## Documentation

For any sort of documentation, it is recommended to follow rust inner doc formats i.e `//!` for entire pallet; `///` for config, storage, events, errors, dispatchables.

And for overview, add a section with `## Overview` title & mention with dispatchables in bullets. That's it!

```markdown
## Overview

Hello pallet has 2 dispatchables:

- `say_hello`
- `say_any`
```
