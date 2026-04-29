# Security model

What this code trusts, where the trust comes from, and what an auditor should examine first.

## Scope

The verifier in `programs/sp1-beefy-verifier/` is the cryptographic root for any Hyperbridge message reaching Solana. If it accepts an invalid proof, every downstream consumer (host program, ISMP messages, escrow releases, asset transfers) is compromised.

## Trust roots

The verifier carries two hardcoded constants. Either being wrong renders verification meaningless.

| Constant | File | Pinned to |
|---|---|---|
| `GROTH16_VK_V6_1_0_BYTES` (492 B) | `programs/sp1-beefy-verifier/proofs/groth16_vk_v6_1_0.bin` (loaded via `include_bytes!` in `vk.rs`) | The SP1 v6.1.0 Groth16 verifying key. Byte-identical to upstream `succinctlabs/sp1@v6.1.0/crates/verifier/vk-artifacts/groth16_vk.bin` (sha256 `4388a21c687fdd5f218d7e3d13190cac4c5355818d3605fd5fb811df468ee696`). |
| `VK_ROOT_V6_1_0_BYTES` (32 B) | `programs/sp1-beefy-verifier/src/vk.rs` | The Merkle root over all SP1 recursion-program verifying keys at SP1 v6.1.0: `0x002f850ee998974d6cc00e50cd0814b098c05bfade466d28573240d057f25352`. The verifier rejects any proof whose `vk_root` field doesn't match this constant — i.e. proofs generated against a different SP1 version or a substituted recursion VK. |

In addition, the **caller** supplies a third trusted constant — the `sp1_vkey_hash` — which identifies the specific SP1 program (e.g. Hyperbridge's BEEFY circuit) the proof was generated for. The verifier itself is generic over this; identifying the right SP1 program is the host program's responsibility.

## What the verifier checks

`verify_sp1_v6` (`src/verifier.rs:33-105`) accepts the proof only if **all** of the following hold:

1. Proof length is exactly 356 bytes.
2. `proof[0..4] == sha256(GROTH16_VK_V6_1_0_BYTES)[..4]` — proof was produced against this VK.
3. `proof[36..68] == VK_ROOT_V6_1_0_BYTES` — recursion VK pinning.
4. `proof[4..36] == expected_exit_code` (caller passes `[0u8; 32]` for "SP1 program terminated successfully").
5. The Groth16 BN254 pairing equation holds for `(piA, piB, piC, public_inputs, VK)`, where `public_inputs` is the 5-element field vector `[sp1_vkey_hash_padded, sha256(sp1_public_inputs), exit_code, vk_root, proof_nonce]`.

Any failure → `Err(...)`. The caller surfaces this to the runtime, which rejects the transaction.

## What the verifier does NOT check

By design — these are upper-layer concerns:

- **What the SP1 program actually proved.** The verifier is a generic SP1 v6 Groth16 checker. A proof for an unrelated SP1 program would still verify if the caller passed the wrong `sp1_vkey_hash`. Hosts must hardcode or otherwise tightly bind the expected `sp1_vkey_hash`.
- **Whether `sp1_public_inputs` are well-formed.** The verifier hashes the bytes and feeds them into the pairing. The host program is responsible for constructing `sp1_public_inputs` correctly (e.g. ABI-encoding BEEFY public inputs).
- **State-machine semantics.** Authority-set rotation, monotonic block-height advancement, replay prevention — all live in the host program, not here.

## Highest-risk areas for review

- `programs/sp1-beefy-verifier/src/utils.rs` — vendored from `succinctlabs/sp1-solana` (MIT). ~160 LOC of proof and VK parsing (curve-point decompression, endianness conversion, `hash_public_inputs`). Historical Groth16 implementations have had CVEs in exactly this surface — input parsing, point validation, scalar reduction.
- `programs/sp1-beefy-verifier/src/verifier.rs` — the v6 envelope checks and the public-input vector construction. Specifically the byte-zeroing on `sp1_vkey_hash[0]` and on the top 3 bits of `sha256(sp1_public_inputs)`, both required for valid BN254 Fr elements.
- `programs/sp1-beefy-verifier/src/vk.rs` — verify the VK bytes against the upstream sha256 published above before any production deployment.

## Reporting issues

Please report security issues privately to `hello@polytope.technology`. Do not open public issues for vulnerabilities.
