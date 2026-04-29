//! Verifying-key artefacts for SP1 v6.1.0.
//!
//! - `GROTH16_VK_V6_1_0_BYTES`: byte-identical to upstream
//!   `succinctlabs/sp1@v6.1.0/crates/verifier/vk-artifacts/groth16_vk.bin`
//!   (sha256 4388a21c687fdd5f218d7e3d13190cac4c5355818d3605fd5fb811df468ee696).
//!
//! - `VK_ROOT_V6_1_0_BYTES`: the recursion-VK merkle root commitment. Upstream
//!   computes this dynamically via `VerifierRecursionVks::default().root()`;
//!   pulling that pipeline into a Solana program is impractical, so we capture
//!   the value once from a known-valid v6.1.0 proof (it's a per-version
//!   constant — not per-proof) and hardcode it here.
//!
//!   Extracted via [`crate::extract_vk_root`] against Hyperbridge's BEEFY
//!   SP1 v6 fixture (block #30,701,354, `FIXTURE_VKEY =
//!   0x0059fd0bff44da77…` — see
//!   `modules/pallets/beefy-consensus-proofs/src/benchmarking.rs:37,39,41`).
//!   The integration test `tests/host_verify.rs` re-extracts this on every
//!   run and asserts byte-equality with the constant below; any drift
//!   between SP1 versions will fail loudly there.

pub const GROTH16_VK_V6_1_0_BYTES: &[u8] =
    include_bytes!("../../../proofs/groth16_vk_v6_1_0.bin");

pub const VK_ROOT_V6_1_0_BYTES: [u8; 32] = [
    0x00, 0x2f, 0x85, 0x0e, 0xe9, 0x98, 0x97, 0x4d, 0x6c, 0xc0, 0x0e, 0x50, 0xcd, 0x08, 0x14,
    0xb0, 0x98, 0xc0, 0x5b, 0xfa, 0xde, 0x46, 0x6d, 0x28, 0x57, 0x32, 0x40, 0xd0, 0x57, 0xf2,
    0x53, 0x52,
];
