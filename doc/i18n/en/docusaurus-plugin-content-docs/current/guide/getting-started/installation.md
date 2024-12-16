---
sidebar_position: 1
---

# Installation
## Browser Usage

The easiest way to start using Althread is to use the online editor available at [althread.github.io/editor](https://althread.github.io/editor). This will allow you to test the language without having to install anything on your machine.
## Local Installation

To use Althread on your machine, you must install the Althread compiler.
* Clone the github project: `git clone https://github.com/althread/althread.git`
* Run the program (this will install dependencies and run the program): `cargo run --release`
* You can also compile the program with `cargo build --release` and run the program with `./target/release/althread-cli`
The available commands are as follows:


### Compile

```
./target/release/althread-cli compile file.alt
```
compiles the `file.alt` program and displays any potential errors. On success, displays the abstract syntax tree and the generated code.

### Run
```
./target/release/althread-cli run file.alt
```

compiles and runs the `file.alt` program. On success, displays the execution result. Use the `--debug` option to see lines executed by processes. Use the `--verbose` option to see the evolution of each process's state. Use the `--seed <seed>` option to set the random number generator seed.


### Random search

```
./target/release/althread-cli random-search file.alt
```

compiles and runs the `file.alt` program multiple times using different random values. In case of an invariant violation, indicates the seed that caused the error.

### Check

```
./target/release/althread-cli check file.alt
```

compiles the `file.alt` program, generates the graph of accessible system states, and checks that invariants are respected in each state.