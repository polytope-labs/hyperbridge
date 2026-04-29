//! BEEFY consensus-update helpers — pure functions over
//! `Sp1BeefyProof` + `ConsensusState`.

use anchor_lang::prelude::*;
use parity_scale_codec::{Compact, Decode};
use sha3::{Digest, Keccak256};

use sp1_beefy_verifier::{ConsensusState, Sp1BeefyProof};

use crate::error::HandlerError;

/// Hand-rolled `PublicInputs.abi_encode()`. Layout:
///
/// ```text
/// [0..32]    outer offset = 0x20
/// [32..64]   authorities_root
/// [64..96]   authorities_len   (u256 BE)
/// [96..128]  leaf_hash
/// [128..160] headers offset = 0x80
/// [160..192] headers length    (u256 BE)
/// [192..]    32 B id (u256 BE) || 32 B hash, per header
/// ```
pub fn build_public_inputs_abi(
    authorities_root: &[u8; 32],
    authorities_len: u32,
    leaf_hash: &[u8; 32],
    headers: &[(u32, [u8; 32])],
) -> Vec<u8> {
    let n = headers.len();
    let mut out = Vec::with_capacity(192 + 64 * n);

    out.extend_from_slice(&u256_be(32));
    out.extend_from_slice(authorities_root);
    out.extend_from_slice(&u256_be(authorities_len as u64));
    out.extend_from_slice(leaf_hash);
    out.extend_from_slice(&u256_be(128));
    out.extend_from_slice(&u256_be(n as u64));
    for (para_id, hash) in headers {
        out.extend_from_slice(&u256_be(*para_id as u64));
        out.extend_from_slice(hash);
    }
    out
}

#[inline]
fn u256_be(value: u64) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[24..].copy_from_slice(&value.to_be_bytes());
    buf
}

#[inline]
pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    Keccak256::digest(bytes).into()
}

/// Reads `(number, state_root)` from a SCALE-encoded Substrate header
/// (parent_hash 32B || Compact<u32> number || state_root 32B || …).
pub fn extract_header_prefix(header: &[u8]) -> Result<(u32, [u8; 32])> {
    require!(header.len() >= 32 + 1 + 32, HandlerError::HeaderTooShort);

    let mut input: &[u8] = &header[32..];
    let number = Compact::<u32>::decode(&mut input)
        .map_err(|_| error!(HandlerError::HeaderDecodeFailed))?
        .0;
    require!(input.len() >= 32, HandlerError::HeaderTooShort);

    let mut state_root = [0u8; 32];
    state_root.copy_from_slice(&input[..32]);
    Ok((number, state_root))
}

pub struct ConsensusUpdate {
    pub new_height: u32,
    /// `(state_machine, block_number, state_root)` per parachain header.
    pub commitments: Vec<(u32, u32, [u8; 32])>,
    pub authority_set_id: u64,
}

/// Verifies the BEEFY proof against `state`. Pure — caller persists.
pub fn verify_and_extract_update(
    state: &ConsensusState,
    proof: &Sp1BeefyProof,
    sp1_vkey_hash: &[u8; 32],
) -> Result<ConsensusUpdate> {
    require!(
        proof.block_number > state.latest_beefy_height,
        HandlerError::NonMonotonicHeight
    );

    let authority = if proof.validator_set_id == state.next_authorities.id {
        &state.next_authorities
    } else if proof.validator_set_id == state.current_authorities.id {
        &state.current_authorities
    } else {
        return err!(HandlerError::UnknownAuthoritySet);
    };

    let leaf_hash = {
        use parity_scale_codec::Encode;
        keccak256(&proof.mmr_leaf.encode())
    };
    let headers: Vec<(u32, [u8; 32])> = proof
        .headers
        .iter()
        .map(|h| (h.para_id, keccak256(&h.header)))
        .collect();
    let public_inputs = build_public_inputs_abi(
        &authority.keyset_commitment,
        authority.len,
        &leaf_hash,
        &headers,
    );

    sp1_beefy_verifier::verify_sp1_v6(
        &proof.proof,
        &public_inputs,
        sp1_vkey_hash,
        &sp1_beefy_verifier::VK_ROOT_V6_1_0_BYTES,
        &[0u8; 32],
    )
    .map_err(|_| error!(HandlerError::Sp1VerificationFailed))?;

    let mut commitments = Vec::with_capacity(proof.headers.len());
    for h in &proof.headers {
        let (number, state_root) = extract_header_prefix(&h.header)?;
        commitments.push((h.para_id, number, state_root));
    }

    Ok(ConsensusUpdate {
        new_height: proof.block_number,
        commitments,
        authority_set_id: proof.validator_set_id,
    })
}

/// On a `next_authorities`-signed proof, rotate `current = next; next = mmr_leaf.beefy_next_authority_set`.
pub fn maybe_rotate_authorities(state: &mut ConsensusState, proof: &Sp1BeefyProof) {
    if proof.validator_set_id == state.next_authorities.id {
        state.current_authorities = state.next_authorities.clone();
        state.next_authorities = proof.mmr_leaf.beefy_next_authority_set.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parity_scale_codec::Encode;

    #[test]
    fn header_prefix_decodes_compact_block_number_and_state_root() {
        let parent_hash = [0xaau8; 32];
        let block_number: u32 = 30_701_354;
        let state_root = [0xbbu8; 32];
        let extrinsics_root = [0xccu8; 32];

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&parent_hash);
        bytes.extend_from_slice(&Compact(block_number).encode());
        bytes.extend_from_slice(&state_root);
        bytes.extend_from_slice(&extrinsics_root);
        bytes.extend_from_slice(&[0u8]); // empty digest

        let (n, root) = extract_header_prefix(&bytes).unwrap();
        assert_eq!(n, block_number);
        assert_eq!(root, state_root);
    }

    #[test]
    fn header_prefix_rejects_short_input() {
        // 32 + 1 + 31 — short of state_root.
        let too_short = vec![0u8; 32 + 1 + 31];
        assert!(extract_header_prefix(&too_short).is_err());
    }

    #[test]
    fn build_public_inputs_layout_matches_documented_layout() {
        let auth_root = [0x11u8; 32];
        let leaf = [0x22u8; 32];
        let headers = [(2042u32, [0x33u8; 32]), (2007u32, [0x44u8; 32])];
        let buf = build_public_inputs_abi(&auth_root, 5, &leaf, &headers);

        assert_eq!(buf.len(), 192 + 64 * headers.len());
        assert_eq!(&buf[32..64], &auth_root);
        assert_eq!(&buf[96..128], &leaf);
        // headers length lives at [160..192] as u256 BE.
        assert_eq!(buf[160..192][31], 2);
        // First header id (2042) at [216..224] (last 8 bytes of the u256 BE slot).
        assert_eq!(u64::from_be_bytes(buf[216..224].try_into().unwrap()), 2042);
        assert_eq!(&buf[224..256], &[0x33u8; 32]);
    }
}
