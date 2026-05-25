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
