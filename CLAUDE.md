# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## ABI Encoding: Rust ↔ Solidity Parity

ISMP types (PostRequest, GetRequest, GetResponse) are hashed using `keccak256(abi.encode(struct))` on both Solidity and Rust sides. The encoding must be identical across both languages.

### Rules

- **Solidity**: Use `abi.encode(req)` to encode structs. This produces a tuple-wrapped encoding.
- **Rust (alloy)**: Use `SolValue::abi_encode()` on the generated sol type. This matches Solidity's `abi.encode(struct)`.
- **NEVER use `abi_encode_params()`** for commitment hashing. It encodes struct fields as bare function parameters *without* the outer tuple wrapper, so it does NOT match `abi.encode(struct)` in Solidity.

### Cross-language compatibility

| Direction | Method |
|-----------|--------|
| Rust encode → Solidity decode | `sol_struct.abi_encode()` → `abi.decode(data, (StructType))` |
| Solidity encode → Rust decode | `abi.encode(structVal)` → `SolStruct::abi_decode(&data)` |

### Where the encoding lives

- **Solidity**: `sdk/packages/core/contracts/libraries/Message.sol` — `encode()` and `hash()` functions
- **Rust**: `modules/ismp/core/src/abi.rs` — `encode_post_request()`, `encode_get_request()`, `encode_get_response()`, `encode_request()` (enum dispatch)
- **Rust types**: `evm/rust/` (`ismp-abi` crate) — generated sol types from compiled ABI JSON artifacts
- **Conversions**: `modules/ismp/core/src/abi.rs` — `From<router::PostRequest> for EvmHost::PostRequest`, etc.

### Tests

Cross-language encoding parity is tested in `evm/tests/rust/src/tests/abi_encode.rs`. These tests:
1. Encode a struct in Rust, send to Solidity's `abi.decode`, verify it decodes correctly
2. Encode the same struct in Solidity, bring it back to Rust, verify bytes are identical
3. Compare `keccak256(encode(x))` from both sides

The Solidity test helper contract is `evm/tests/foundry/AbiEncodeTest.sol`.

## AI workflow docs

Each package under `sdk/packages/*` keeps AI workflow docs in `docs/ai/`, as one file per
entry. Every package's `docs/ai/README.md` states the conventions; in short:

- `changelog/YYYY-MM-DD-short-title.md` — one file per AI-assisted code change: what changed and why, then a `Files:` line listing the files touched.
- `decisions/YYYY-MM-DD-short-title.md` — one file per non-obvious choice, with the alternatives considered and why they lost.
- `flows/<flow-name>.md` — one file per code path, no date in the name, describing how it actually executes. Update it when the documented control flow changes, and add flows as they are read and verified. Never document a flow speculatively.

Always write a new file for a changelog or decision entry. Never append to an existing one,
and never gather entries back into a shared `ChangeLog.md`-style file — that is the layout
this replaced. Two concurrent PRs appending to one file collide on the same line, and GitHub
blocks the merge: it ignores the `merge=union` driver in `.gitattributes` that resolves the
collision locally, so the conflict is real as far as the PR is concerned. Separate files
cannot collide, so the question never arises.

Flow files are the exception — they are edited in place, so two PRs revising the same flow
do conflict. That conflict is worth seeing, because it means two changes disagree about how
the code runs.

When changing code in a package that has `docs/ai/`, writing these files is part of the
change, not optional follow-up. When starting substantial work in a package that has none
yet (lz-endpoint), create the three directories and the README as the first step and seed
them from that task's actual work — never with empty templates.

These are not release notes. Package `CHANGELOG.md` files are changesets release logs,
managed separately.
