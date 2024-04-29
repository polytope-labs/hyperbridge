#!/usr/bin/env bash

wasm-pack build --target=bundler --release --no-default-features --features=wasm
cd pkg
npx --yes change-package-name @polytope-labs/hyperclient
