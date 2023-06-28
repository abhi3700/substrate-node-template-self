# Hello Pallet

## Overview

Hello pallet with 2 dispatchables:

- `say_hello()`
- `say_any()`

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
$ cargo test -p pallet-hello
```

---

To run the individual test:

```sh
# example
$ cargo test -p pallet-hello --lib -- tests::fails_for_wish_start_w_hello
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

## Pallet

### Dispatchables

#### `say_hello`

In Polkadot JS Apps,

Authorize transaction for the selected dispatchable function via signed transaction:
![](../../img/hello-pallet-sayhello1.png)

added in block:
![](../../img/hello-pallet-sayhello2.png)

---

In Substrate FE template repo GUI,

![](../../img/hello-pallet-sayhello3.png)

---

In CLI,

![](../../img/hello-pallet-sayhello4.png)

#### `say_any`

In Polkadot JS Apps,

Authorize transaction for the selected dispatchable function via signed transaction:
![](../../img/hello-pallet-sayany1.png)

added in block:
![](../../img/hello-pallet-sayany2.png)

---

In Substrate FE template repo GUI,

![](../../img/hello-pallet-sayany3.png)

---

In CLI,

![](../../img/hello-pallet-sayany4.png)

---

❌ Throwing error as the `wish` started with 'hello' (as added in dispatchable function):
![](../../img/hello-pallet-sayany5.png)
