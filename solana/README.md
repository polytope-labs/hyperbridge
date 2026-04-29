# Hyperbridge on Solana

On-chain Solana code for receiving Hyperbridge messages.

This directory is a standalone Cargo workspace, kept separate from the Hyperbridge runtime workspace at the repo root because the Solana BPF target and the polkadot-sdk dep tree don't compile together cleanly. Treat it as its own subproject.

## Layout

```
solana/
  programs/
    sp1-beefy-verifier/   on-chain SP1 v6 Groth16 verifier (BEEFY consensus)
  proofs/
    groth16_vk_v6_1_0.bin verifying key for SP1 v6.1.0
  Cargo.toml              workspace
  SECURITY.md             trust roots and threat model
```

## `sp1-beefy-verifier`

Verifies SP1 v6 Groth16 proofs of Hyperbridge BEEFY consensus, calling Light Protocol's [`groth16-solana`](https://github.com/Lightprotocol/groth16-solana) directly to run the BN254 pairing check via Solana's `alt_bn128_*` syscalls. Bypasses [`succinctlabs/sp1-solana`](https://github.com/succinctlabs/sp1-solana), which currently supports SP1 v2-v5 only.

Two ways to consume it:

- **As a library** (default). Host programs link the crate and call `verify_sp1_v6` in-process. No CPI, no separate program account.
- **As a stand-alone deployable program.** Build with `--features entrypoint`. Used for end-to-end CU and tx-size measurement (see the `onchain_tx` example).

### Measured behavior

| Metric | Value | Solana cap | Headroom |
|---|---|---|---|
| Compute units per verification | **278,286** | 1,400,000 | 5.0× |
| Single-header tx size | **859 B** | 1,232 | 1.4× |
| Headers per single tx (max) | **6** | — | — |
| Program binary (`.so`) | ~140 KB | — | — |

Each additional parachain header adds exactly 64 B to the tx (one Solidity-ABI `(uint256 id, bytes32 hash)` tuple in `public_inputs`). Beyond 6 headers per single tx, a buffer-account upload pattern is required.

### Build and test

Requires: Rust (stable), Solana CLI 3.1+.

```sh
# Native unit + integration tests (host_verify, tx_size_sweep)
cargo test --release --manifest-path programs/sp1-beefy-verifier/Cargo.toml

# Solana BPF build (produces target/deploy/sp1_beefy_verifier.so)
cargo build-sbf --features entrypoint \
  --manifest-path programs/sp1-beefy-verifier/Cargo.toml
```

### End-to-end on a local validator

```sh
# Terminal 1
solana-test-validator --reset

# Terminal 2
solana config set --url http://127.0.0.1:8899
solana airdrop 10
cargo build-sbf --features entrypoint \
  --manifest-path programs/sp1-beefy-verifier/Cargo.toml
solana program deploy target/deploy/sp1_beefy_verifier.so
# copy the printed Program Id

PROGRAM_ID=<program-id> cargo run --release --example onchain_tx \
  --manifest-path programs/sp1-beefy-verifier/Cargo.toml
```

Successful output ends with:

```text
Program log: sp1 v6 beefy groth16 verification ok
CONSUMED COMPUTE UNITS: 278286
```

### How the adapter works

SP1 v6's `.bytes()` output packs 356 bytes into this layout:

```text
[0..4]     sha256(groth16_vk)[..4]          vk fingerprint selector
[4..36]    exit_code                         32 B  (NEW in v6)
[36..68]   vk_root                           32 B  (NEW in v6)
[68..100]  proof_nonce                       32 B  (NEW in v6)
[100..356] piA || piB || piC (G1||G2||G1)   256 B uncompressed
```

The Groth16 public-input vector grew from 2 elements (v5) to 5 elements (v6):

```text
[sp1_vkey_hash, hash(sp1_public_inputs), exit_code, vk_root, proof_nonce]
```

`verifier.rs` parses the v6 layout, builds the 5-element vector, and hands the triple off to `groth16-solana` for the BN254 pairing check.

### Instruction-data layout (when used as a stand-alone program)

```text
[0..32]     sp1_vkey_hash         32 B
[32..36]    proof_len (u32 BE)     4 B
[36..36+p]  proof                  p B (356 for v6)
[36+p..]    sp1_public_inputs      variable
```

### Credits

- `programs/sp1-beefy-verifier/src/utils.rs` vendors proof + verifying-key parsing from [`succinctlabs/sp1-solana`](https://github.com/succinctlabs/sp1-solana) (MIT). Vendored rather than imported because the upstream hasn't released v6 support yet — see [succinctlabs/sp1-solana#23](https://github.com/succinctlabs/sp1-solana/issues/23).
- BN254 pairing arithmetic by Light Protocol's [`groth16-solana`](https://github.com/Lightprotocol/groth16-solana) (via Solana's `alt_bn128_*` syscalls).
