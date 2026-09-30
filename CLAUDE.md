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

This rule applies to every PR and package, including indexer, simplex, SDK, and core.

- Document the final feature or fix: what the PR does, the resulting behavior, and any
  interfaces, fields, or constraints a reader needs to understand it.
- Keep the minimum useful Markdown. Prefer one concise note when it covers the change;
  update that note during review instead of adding a file for each iteration.
- Omit work logs, implementation history, rejected cleanup approaches, and narratives
  about routine refactoring, deduplication, optimization, or testing. These are expected
  engineering work. Describe them only when they are the actual purpose of the PR.
- Use concrete feature and field names instead of generic labels such as "enrichment."
- Before finalizing a PR, review every added or modified Markdown file. Delete redundant
  or process-only notes and remove references to deleted files. Keep the PR description
  focused on the final change, with concise validation results there.

Read relevant existing package notes before changing code to preserve documented
assumptions. Use `docs/ai/changelog/YYYY-MM-DD-short-title.md` for a needed feature note.
Add a decision or flow document only when it explains a distinct, lasting constraint or
code path that the feature note does not cover. Update an existing flow when its behavior
changes; never document a flow speculatively.

Use a separate filename for each PR's note to avoid conflicts between concurrent PRs.
Revise or consolidate notes added by the current PR rather than accumulating review
history. Do not append to shared `ChangeLog.md` files or create empty documentation
directories and templates merely because code changed.

Package `CHANGELOG.md` files remain release logs managed by changesets.
