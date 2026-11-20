# 2026-10-23 — CI runs the wasm crate checks only for runtime-affecting paths

`Check Wasm Crates` now runs only when a changed path can affect the runtime crates.

The `detect-changes` job has a new `wasm` filter for `modules/**`, `evm/rust/**`,
`parachain/runtimes/**`, the root `Cargo.toml` and `Cargo.lock`, `rust-toolchain.toml`
and `.github/workflows/ci.yml`.

`cargo tree -p gargantua-runtime -p nexus-runtime --target wasm32-unknown-unknown`
shows that the runtime crates' only workspace path dependencies are under `modules/`,
`evm/rust/` and `parachain/runtimes/`. A tesseract-only PR no longer pays for the
two-leg wasm matrix.
