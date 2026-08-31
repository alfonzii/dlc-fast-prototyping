# DLC Fast Prototyping

**DLC Fast Prototyping** is a lightweight framework for fast experimentation, benchmarking, and development of [Discreet Log Contracts (DLCs)](https://adiabat.github.io/dlc.pdf) on Bitcoin.  
It allows testing various cryptographic configurations—such as ECDSA vs. Schnorr adaptor signatures—and running detailed benchmarks for performance analysis.


## Prerequisites

- Rust (recommended version: `cargo 1.85.0`)
- A Unix-like system is recommended (Linux/macOS)
    - or WSL 2 on Windows


## Running the Program
### Run with Baseline Configuration
To run the program with the baseline settings (ECDSA + simple method):
```
cargo run --release --features baseline
```
- `--release` enables compiler optimizations for realistic performance.

This will execute the DLC setup using ECDSA adaptor signatures and simple method for anticipation point computation, all running single-threadly.

### Run with Custom Features
To test specific configurations (e.g., using Schnorr adaptor signatures, basis method and enabling parallelism), specify the desired ones:
```
cargo run --release --features "enable-benchmarks, schnorr, parallel-cpt, basis-method"
```

Already implemented custom features (visible in `Cargo.toml`) to be tried out are:
- `ecdsa` - ECDSA adaptor signature scheme
- `schnorr` - Schnorr adaptor signature scheme (ECDSA or Schnorr must be used, not both nor neither)
- `simple-method` - simple atp point computation method (more straightforward implementation)
- `basis-method` - basis atp point computation method (faster for most cases)
- `parallel-cpt` - enable parallel computation of anticipation points and adaptor signatures
- `parallel-parser` - enable parallel creation of `ParsedContract` structure
- `enable-benchmarks` - enable full end-to-end benchmark thorough whole run of program, showing run time of individual DLC setup steps

> ⚠️ Warning for developers: To get rid of errors in your IDE for not using features, uncomment `default` feature in `Cargo.toml`. If you want to use `schnorr` or `basis-method` for developing purposes, then use `--no-default-feature` flag for successful compilation, to get rid of `ecdsa` and `simple-method` from default, or you can just comment `default` back.

## Additional Configuration
Parameters that are not controlled via feature flags are located in:
```
src/config.rs
```
What can be changed in `config.rs` file is following:
- Constants
    + `NB_DIGITS` - number of digits that outcome is using
    + `CONTRACT_INPUT_PATH` - path to contract from which we will be setting up DLC
- Type aliases
    + `MyOracle` - oracle type
    + `MyParser` - parser type


## Benchmarks
The framework supports both:
- **End-to-end DLC benchmarks** (via `enable-benchmarks` feature)
- **Function-specific math microbenchmarks** (via `math-bench.rs`)

### Run High-Level Benchmarks
To measure overall runtime of DLC creation and execution:
```
cargo run --release --features "enable-benchmarks, [CUSTOM FEATURES]"
```
Two key runtime bottlenecks typically emerge:

- `Init storage`

- `Verify adaptors`

These are analyzed further using fine-grained function-level benchmarking.

### Run Microbenchmarks (Function-level)

To benchmark individual functions in isolation:
```
cargo bench --bench math-bench --features "[CUSTOM FEATURES]"
```
This allows testing core cryptographic primitives.

> Only adaptor signature scheme and anticipation point computation features are relevant for function-level benchmarks.

### Run Comparative Decision Benchmarks

During development, several alternative designs were benchmarked to guide decisions. These comparisons are available in:
```
benches/benchmark.rs
```
You can run them similarly:
```
cargo bench --bench benchmark --features "[CUSTOM FEATURES]"
```

## About

This framework was developed as part of the thesis project _“Practical Oracle-Based Bitcoin Payments”_.
The goal was to create a flexible, modular environment for evaluating and optimizing DLC protocol components—particularly focusing on cryptographic performance and protocol design.

For more details on the design rationale and performance analysis, please refer to the [thesis](http://hdl.handle.net/20.500.11956/209708).

## Acknowledgments

Special thanks to:

- [@Tibo-lg](https://github.com/Tibo-lg) for his responsiveness, and work on [`rust-dlc`](https://github.com/p2pderivatives/rust-dlc), which laid the foundation for many of the ideas explored here.
- [@siv2r](https://github.com/siv2r) for implementing Schnorr adaptor signatures in C for `secp256k1-zkp`, and for his help to hasten up building Rust wrappers around them. His support made it possible to focus on the benchmarking framework without being blocked on low-level cryptographic integration.
- [@Kixunil](https://github.com/Kixunil) for his invaluable experience with the Rust language and for the time he dedicated to code review so that the project can better and more qualitatively reflect the standards used in Rust language.

This project would not have been possible without their help and contributions to the open-source DLC ecosystem.


## License

This project is licensed under the [MIT License](LICENSE).

Parts of the code are adapted from MIT-licensed projects, including:
- [`rust-dlc`](https://github.com/p2pderivatives/rust-dlc)

See [NOTICE](NOTICE) for full attribution details.
