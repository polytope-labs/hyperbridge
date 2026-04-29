//! SP1 v6 Groth16 proof verifier — calls Light Protocol's `groth16-solana`
//! directly to run the BN254 pairing check via Solana's `alt_bn128_*` syscalls.
//!
//! Proof layout (SP1 v6.1.0, 356 bytes after SCALE-decoding `Sp1BeefyProof.proof`):
//!
//! ```text
//! [0..4]     sha256(groth16_vk)[..4]   vkey-hash prefix
//! [4..36]    exit_code                 NEW in v6
//! [36..68]   vk_root                   NEW in v6 - matches recursion VK merkle root
//! [68..100]  proof_nonce               NEW in v6
//! [100..356] piA || piB || piC         uncompressed (64 + 128 + 64)
//! ```
//!
//! Groth16 public inputs (5 elements for v6):
//! `[sp1_vkey_hash, hash_public_inputs(sp1_public_inputs), exit_code, vk_root, proof_nonce]`

use groth16_solana::groth16::{Groth16Verifier, Groth16Verifyingkey};
use sha2::{Digest, Sha256};

use crate::utils::{
    self, Error, hash_public_inputs, load_groth16_verifying_key_from_bytes, load_proof_from_bytes,
};
use crate::vk::GROTH16_VK_V6_1_0_BYTES;

const VK_HASH_PREFIX_LEN: usize = 4;
const META_LEN: usize = 32 * 3; // exit_code || vk_root || proof_nonce
const EXPECTED_PROOF_LEN: usize = VK_HASH_PREFIX_LEN + META_LEN + 256;

/// Verify an SP1 v6 Groth16 proof against supplied SP1 public-inputs bytes and
/// the SP1 circuit's vkey hash.
///
/// Returns the verified `(exit_code, vk_root, proof_nonce)` tuple on success
/// (useful for logging / binding), or an `Error` on any failure.
pub fn verify_sp1_v6(
    proof: &[u8],
    sp1_public_inputs: &[u8],
    sp1_vkey_hash: &[u8; 32],
    expected_vk_root: &[u8; 32],
    expected_exit_code: &[u8; 32],
) -> Result<([u8; 32], [u8; 32], [u8; 32]), Error> {
    if proof.len() != EXPECTED_PROOF_LEN {
        return Err(Error::InvalidProof);
    }

    // 1. sha256(groth16_vk)[..4] match — ensures proof is for this VK version.
    let vk_hash: [u8; 4] = Sha256::digest(GROTH16_VK_V6_1_0_BYTES)[..VK_HASH_PREFIX_LEN]
        .try_into()
        .map_err(|_| Error::InvalidProof)?;
    if vk_hash != proof[..VK_HASH_PREFIX_LEN] {
        return Err(Error::Groth16VkeyHashMismatch);
    }

    // 2. Parse the three v6 metadata fields.
    let exit_code: [u8; 32] = proof[4..36].try_into().map_err(|_| Error::InvalidProof)?;
    let vk_root: [u8; 32] = proof[36..68].try_into().map_err(|_| Error::InvalidProof)?;
    let proof_nonce: [u8; 32] = proof[68..100].try_into().map_err(|_| Error::InvalidProof)?;

    if &vk_root != expected_vk_root {
        return Err(Error::VkRootMismatch);
    }
    if &exit_code != expected_exit_code {
        return Err(Error::ExitCodeMismatch);
    }

    // 3. Parse the Groth16 proof triple (proof[100..356] → πA/πB/πC).
    let parsed = load_proof_from_bytes(&proof[100..EXPECTED_PROOF_LEN])?;

    // 4. Decompress the VK.
    let vk = load_groth16_verifying_key_from_bytes(GROTH16_VK_V6_1_0_BYTES)?;

    // 5. Build the 5-element public-input vector. Pattern follows sp1-solana's
    //    v5 convention: sp1_vkey_hash is 32 bytes but has byte 0 zeroed to
    //    guarantee < BN254 Fr modulus; hash_public_inputs zeros the top 3 bits
    //    for the same reason. `exit_code`, `vk_root`, `proof_nonce` are passed
    //    as-is — upstream expects them to be valid Fr elements already.
    let mut sp1_vkey_hash_padded = [0u8; 32];
    sp1_vkey_hash_padded[1..].copy_from_slice(&sp1_vkey_hash[1..]);

    let committed = hash_public_inputs(sp1_public_inputs);

    let public_inputs: [[u8; 32]; 5] =
        [sp1_vkey_hash_padded, committed, exit_code, vk_root, proof_nonce];

    // 6. Hand off to groth16-solana's pairing check.
    let vk_for_verifier = Groth16Verifyingkey {
        nr_pubinputs: vk.nr_pubinputs as usize,
        vk_alpha_g1: vk.vk_alpha_g1,
        vk_beta_g2: vk.vk_beta_g2,
        vk_gamme_g2: vk.vk_gamme_g2,
        vk_delta_g2: vk.vk_delta_g2,
        vk_ic: vk.vk_ic.as_slice(),
    };

    let mut verifier = Groth16Verifier::<5>::new(
        &parsed.pi_a,
        &parsed.pi_b,
        &parsed.pi_c,
        &public_inputs,
        &vk_for_verifier,
    )
    .map_err(|_| Error::VerificationError)?;

    verifier.verify().map_err(|_| Error::VerificationError)?;

    Ok((exit_code, vk_root, proof_nonce))
}

/// Extract the `vk_root` from a v6 proof byte buffer without running verification.
/// Useful once to capture the constant, then hardcode.
pub fn extract_vk_root(proof: &[u8]) -> Result<[u8; 32], Error> {
    if proof.len() < 68 {
        return Err(Error::InvalidProof);
    }
    proof[36..68].try_into().map_err(|_| Error::InvalidProof)
}

// Keep the Error type reachable from the crate root.
pub use utils::Error as UtilsError;
