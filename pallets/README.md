# Pallets

Pallets are the building blocks of the runtime. They are the modules that implement the business logic of the blockchain. They are the equivalent of smart contracts in other blockchains (parachains).

## List of Pallets

> The order is as per the level of complexity of the pallets.

- [Template](./template) - A template pallet to create new pallets.
- [Hello](./hello) - A simple pallet to say hello to the world.
- [Flipper](./flipper) - A simple pallet to flip a boolean value.
- [Counter](./counter) - A simple pallet to count the number of times it is called.
- [Bank](./bank) - A simple pallet to get balance of an account from inside the pallet.
- [🧑🏻‍💻] [Voting](./voting/) - A pallet to vote for a candidate. [Reference](https://docs.soliditylang.org/en/v0.8.17/solidity-by-example.html#voting)
  - [ ] The default weight as `1` for all the users can be updated to any value based on their locked currency.
- [ ] [Substrate Kitties]()

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
$ cargo build -r
```

#### 6. Run the node:

Run a relaychain node (w/o debug mode):

```sh
$ ./target/release/node-template --dev
```

In debug mode, run a relaychain node:

```sh
$ RUST_LOG=runtime=debug ./target/release/node-template --dev
```

#### 7. Write the test cases for your pallet in the `runtime/src/tests.rs` file. Prior to this, add `mock.rs` file for creating a runtime.

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

## Runtime Upgrade

Select the file: ./target/release/wbuild/node-template-runtime/node-template-runtime.compressed.wasm

Change in `runtime/src/lib.rs` file:

```rust
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("node-template"),
	impl_name: create_runtime_str!("node-template"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 100,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};
```

Change the spec_version to 101, 102, 103, etc. and build the runtime. And the binary will be generated at `./target/release/wbuild/node-template-runtime/node-template-runtime.compressed.wasm` & then upload it to the chain under "Extrinsics >> sudo >> sudoUncheckedWeight" & then "system >> setCode" with file_upload option. And then "Submit Transaction" as Alice (has DOT balance).
