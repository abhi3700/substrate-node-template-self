# Pallets

Pallets are the building blocks of the runtime. They are the modules that implement the business logic of the blockchain. They are the equivalent of smart contracts in other blockchains (parachains).

## List of Pallets

> The order is as per the level of complexity of the pallets.

### Practice

- [x] [Template](./template) - A template pallet to create new pallets.
- [x] [Hello](./hello) - A simple pallet to say hello to the world.
- [x] [Flipper](./flipper) - A simple pallet to flip a boolean value.
- [x] [Counter](./counter) - A simple pallet to count the number of times it is called.
- [ ] [Vault](./vault) - A simple pallet to get balance of an account from inside the pallet.
- [ ] [Voting](./voting/) - A pallet to vote for a candidate. [Reference](https://docs.soliditylang.org/en/v0.8.17/solidity-by-example.html#voting) -

  The default weight as `1` for all the users can be updated to any value based on their locked currency.

- [x] [Lockable Currency](./lockable-currency) - A pallet to lock currency (lockable) for a specific period of time.
- [ ] [Substrate Kitties]()

### Real

- [x] [Bank](./bank/) - A pallet to to handle multiple investment types.
- [ ] 🧑🏻‍💻 DPoS Substrate node
- [ ] AntelopeAccount
  <details>
  <summary>View details:</summary>
  Q. In Rust substrate, I really think that account generation may have a better way. As in, in substrate, currently account has this stash versus controller model. And I feel that it should be in such a way where an account has two key pairs. So if one of the key pair is compromised, the person can actually use the other key pair to change that key pair. Can we do that?

  A. Substrate already supports a model which is very similar to what you've described, known as the Stash/Controller model.

  In this model, a stash account is like your bank account, where your funds are stored securely, and the controller account is like a debit card, which can make decisions on behalf of the stash account (like voting in governance, validating, nominating validators, etc.), but it doesn't hold any funds directly. If the controller is compromised, the attacker won't be able to directly access the funds in the stash account.

  If you want to provide an additional level of security by having two key pairs for the controller account, where one could replace the other if it's compromised, it's possible but it would involve some modifications to the account model in Substrate. You would need to write a custom pallet to support this behavior.

  The custom pallet would maintain a mapping of accounts to an array of their authorized keys. When one key is compromised, the other key could be used to update the compromised key. Here's a very simplified example of what the core logic might look like:

  ```rust
  decl_storage! {
  	trait Store for Module<T: Config> as MultiSigAccountModule {
  		// Mapping of account to its list of authorized keys
  		AccountKeys get(fn account_keys): map hasher(blake2_128_concat) T::AccountId => Vec<T::AccountId>;
  	}
  }

  decl_module! {
  	pub struct Module<T: Config> for enum Call where origin: T::Origin {
  		// Function to update a compromised key
  		#[weight = 10_000]
  		fn update_key(origin, old_key: T::AccountId, new_key: T::AccountId) -> dispatch::DispatchResult {
  			let sender = ensure_signed(origin)?;

  			// Ensure the sender is one of the authorized keys for this account
  			let keys = Self::account_keys(&sender);
  			ensure!(keys.contains(&sender), "Not authorized");

  			// Find and replace the old key with the new key
  			let old_key_index = keys.iter().position(|k| k == &old_key).ok_or("Old key not found")?;
  			AccountKeys::<T>::mutate(&sender, |keys| keys[old_key_index] = new_key);

  			Ok(())
  		}
  	}
  }
  ```

  Please note, this is a very simplified and untested version of what you're looking to achieve. You would also need to add checks to ensure an account can't be left without any authorized keys, and you might want to add limits on the number of authorized keys an account can have.

  Make sure to thoroughly review and test any changes to account management code, as mistakes can lead to lost funds or other serious issues.

  </details>

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
$ cargo test -p pallet-hello

# run the pallet individual tests
$ cargo test -p pallet-hello --lib -- tests::fails_for_wish_start_w_hello
```

> Although there is a button shown above test function to run individual test in VSCode.

In order to view the values of the variables in the test function, you can use `println!` macro.

#### 8. Write the benchmarking code for your pallet in the `runtime/src/benchmarks.rs` file. <!-- TODO: -->

## Package NEW pallet

1. `$ cargo publish --dry-run`, to check if everything is fine.
2. `$ cargo publish`, to publish the pallet to crates.io.

## Add crate to your pallet

From your CLI at the root of the repo location, run this command:

```sh
# if w default features, (used in testing under [dev-dependencies])
$ cargo add <crate-name> -p <pallet-name>

# if w/o default features (used in production under [dependencies])
$ cargo add <crate-name> -p <pallet-name> --no-default-features
```

## Usage of my pallet in other's runtime [OPTIONAL]

This is a very common scenario where anyone would want to use your pallet's funcitonality in their runtime. So, in order to do that, you need to follow these steps:

1. Make a trait for my pallet's dispatchables in a separate file named `mytrait.rs` in the `pallets/<my-pallet>/src` directory.
2. Then other dev can use my pallet's functionality/trait `mytrait.rs` in their runtime's pallet `pallets/<other-pallet>/src/lib.rs` file.
3. And then they can either use it as an associated type of their `Config` trait or use it for their pallet implementation via `impl<T: Config> MyTrait<T> for Pallet<T> {}`.

## Documentation

For any sort of documentation, it is recommended to follow rust inner doc formats i.e `//!` for entire pallet; `///` for config, storage, events, errors, dispatchables.

[TEMPLATE](https://github.com/paritytech/substrate/blob/master/frame/examples/basic/README.md#documentation-template)

To view the documentation of `pallet-example` done inside `src/lib.rs` using `//!`, run this command at the repository root:

```sh
$ cargo doc -p pallet-example --open
```

It would create a `html` file in this location: `target/doc/pallet-example/index.html`

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
