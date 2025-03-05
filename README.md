# Althread

Althread is an open source Promela alternative for modeling and verifying multi-threaded systems.

## Key Features
- Simple modeling of concurrent systems.
- Automatic detection of race conditions and deadlocks.
- Advanced debugging tools.
- Easy-to-learn syntax for beginners.

## Roadmap

- array
- dictionnary
- regex
- visual representation of the global state


## Installation
1. Clone the repository: 
   ```
   git clone https://github.com/althread/althread.git
   ```

2. Navigate to the directory and compile :
    ```
    cd althread
    cargo build
    ```

3. Run an example
    ```
    cargo run run test.alt
    ```

## Quick Start
Here is a minimal example of modeling a multi-thread system :
```
program A() {
    print("Hello world from A process");
}

main {
    run A();
    print("Hello world from main");
}
```

## Full Documentation
- Check out the [full documentation](https://althread.github.io/) for more examples, guides and a reference of symbols.

## Sources
- https://www.rust-lang.org/fr
- https://pest.rs/
- https://docs.rs/clap/latest/clap/
- https://craftinginterpreters.com/contents.html
- https://github.com/tdp2110/crafting-interpreters-rs?tab=readme-ov-file

## License
MIT License. Contributions are welcome !
