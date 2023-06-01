# ismp-rs ![Unit Tests](https://github.com/polytope-labs/ismp-rs/actions/workflows/ci.yml/badge.svg)

Rust implementation of the Interoperable State Machine Protocol. This project is [funded by the web3 foundation](https://github.com/w3f/Grants-Program/blob/master/applications/ismp.md).

## Overview

This repo provides an implementation of the neccessary components laid out in the [ISMP spec](https://github.com/polytope-labs/ismp).

## Testing and Testing Guide
Please see [CI](.github/workflows/ci.yml) for test coverage.

## Run Test in Docker
```bash
docker run --memory="8g" --rm --user root -v "$PWD":/app -w /app rust:latest cargo test --release --manifest-path=./ismp-testsuite/Cargo.toml
```

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2023 Polytope Labs.