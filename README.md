# ismp-substrate ![Unit Tests](https://github.com/polytope-labs/substrate-ismp/actions/workflows/ci.yml/badge.svg) [![Telegram: YourGroup](https://img.shields.io/badge/-Telegram-blue?style=flat-square&logo=Telegram&logoColor=white&link=https://t.me/YourGroup)](https://t.me/ismp_guide) [![Discord: YourServer](https://img.shields.io/badge/-Discord-7289DA?style=flat-square&logo=Discord&logoColor=white&link=https://discord.gg/YourServer)](https://discord.gg/vKAa3XcCBX)

Implementation of the Interoperable State Machine Protocol for substrate runtimes. This project is [funded by the web3 foundation](https://github.com/w3f/Grants-Program/blob/master/applications/ismp.md).

<img src="assets/web3 foundation_grants_badge_white.png"  style="max-width: 100%; height: auto;">

## Overview

This repo holds the foundational components substrate runtimes need to interoperate together using [ISMP](https://github.com/polytope-labs/ismp)

- [pallet-ismp](./)
- [ismp-runtime-api](./pallet-ismp/runtime-api)
- [ismp-rpc](./pallet-ismp/rpc)

## Documentation

Installation and integration guides can be found in the [book](https://ismp.polytope.technology).

## Testing and Testing Guide

This guide assumes [Rust](https://www.rust-lang.org/tools/install) and it's [nightly](https://rust-lang.github.io/rustup/concepts/channels.html#:~:text=it%20just%20run-,rustup%20toolchain%20install%20nightly,-%3A) version is installed, followed by calling the [init](https://github.com/paritytech/polkadot/blob/master/scripts/init.sh) script from the official Polkadot repo to initailize a WASM build environment.

To run the unit tests associated with this library;

```
cargo +nightly test -p pallet-ismp --all-targets --all-features
```

Please see [CI](.github/workflows/ci.yml) for test coverage.

## Run Test in Docker

```bash
docker run --memory="24g" --rm --user root -v "$PWD":/app -w /app rust:latest /bin/bash -c "apt update && apt install -y protobuf-compiler libclang-dev && cargo test --release --manifest-path=./Cargo.toml"
```

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2023 Polytope Labs.
