Rust implementation of the Interoperable State Machine Protocol. This project is [funded by the web3 foundation](https://github.com/w3f/Grants-Program/blob/master/applications/ismp.md).

## Overview

This repo provides an implementation of the neccessary components laid out in the [ISMP spec](https://github.com/polytope-labs/ismp).

## Testing and Testing Guide
This guide assumes [Rust](https://www.rust-lang.org/tools/install) and  it's [nightly](https://rust-lang.github.io/rustup/concepts/channels.html#:~:text=it%20just%20run-,rustup%20toolchain%20install%20nightly,-%3A) version is installed.

To run the tests suite associated with this library;
```
cargo +nightly test --all-features --workspace
```

Please see [CI](.github/workflows/ci.yml) for more test coverage.

## Run Test in Docker
```bash
docker run --memory="8g" --rm --user root -v "$PWD":/app -w /app rust:latest cargo test --release --manifest-path=./ismp-testsuite/Cargo.toml
```

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2023 Polytope Labs.
