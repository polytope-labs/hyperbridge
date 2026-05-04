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

/// Header digests extracted from a parachain header. Mirrors EVM
/// `Header.sol::stateCommitment`: `mmr_root` becomes `overlay_root`,
/// `child_trie_root` becomes `state_root` on the resulting `StateCommitment`.
pub struct HeaderDigests {
    pub block_number: u32,
    pub mmr_root: [u8; 32],
    pub child_trie_root: [u8; 32],
    pub timestamp_secs: u64,
}

/// Walks a SCALE-encoded Substrate parachain header and pulls out the
/// pallet-ismp `ConsensusDigest` (engine_id `b"ISMP"`) and `TimestampDigest`
/// (engine_id `b"ISTM"`).
///
/// Header layout (matches `sp_runtime::generic::Header<u32, BlakeTwo256>`):
/// ```text
/// parent_hash       32B
/// Compact<u32>      block number
/// state_root        32B
/// extrinsics_root   32B
/// digest = Compact<u32> length || [DigestItem]*
/// ```
///
/// `DigestItem` SCALE variant tags (see `sp_runtime::generic::DigestItem`):
/// `0` Other (Vec<u8>) — `4` Consensus ([u8;4] id || Vec<u8>) —
/// `5` Seal — `6` PreRuntime — `7` RuntimeEnvironmentUpdated.
pub fn extract_header_digests(header: &[u8]) -> Result<HeaderDigests> {
    // 32 (parent_hash) + ≥1 (compact number) + 32 (state_root) + 32 (extrinsics_root)
    // + ≥1 (compact digest length).
    require!(header.len() >= 32 + 1 + 32 + 32 + 1, HandlerError::HeaderTooShort);

    let mut input: &[u8] = &header[32..];
    let block_number = Compact::<u32>::decode(&mut input)
        .map_err(|_| error!(HandlerError::HeaderDecodeFailed))?
        .0;
    require!(input.len() >= 64, HandlerError::HeaderTooShort);
    // Skip parachain-side state_root and extrinsics_root — we replace both
    // with the values from the ISMP digest below to match EVM Header.sol.
    input = &input[64..];

    let n_digests = Compact::<u32>::decode(&mut input)
        .map_err(|_| error!(HandlerError::HeaderDecodeFailed))?
        .0;

    let mut mmr_root: Option<[u8; 32]> = None;
    let mut child_trie_root: Option<[u8; 32]> = None;
    let mut timestamp_secs: Option<u64> = None;

    for _ in 0..n_digests {
        require!(!input.is_empty(), HandlerError::HeaderDecodeFailed);
        let variant = input[0];
        input = &input[1..];

        match variant {
            // Consensus(engine_id, data)
            4 => {
                require!(input.len() >= 4, HandlerError::HeaderDecodeFailed);
                let mut engine_id = [0u8; 4];
                engine_id.copy_from_slice(&input[..4]);
                input = &input[4..];
                let data = Vec::<u8>::decode(&mut input)
                    .map_err(|_| error!(HandlerError::HeaderDecodeFailed))?;

                if engine_id == *b"ISMP" {
                    require!(data.len() >= 64, HandlerError::HeaderDecodeFailed);
                    let mut m = [0u8; 32];
                    let mut c = [0u8; 32];
                    m.copy_from_slice(&data[..32]);
                    c.copy_from_slice(&data[32..64]);
                    mmr_root = Some(m);
                    child_trie_root = Some(c);
                } else if engine_id == *b"ISTM" {
                    let mut bytes = data.as_slice();
                    let ts = u64::decode(&mut bytes)
                        .map_err(|_| error!(HandlerError::HeaderDecodeFailed))?;
                    timestamp_secs = Some(ts);
                }
            },
            // Other(Vec<u8>)
            0 => {
                let _ = Vec::<u8>::decode(&mut input)
                    .map_err(|_| error!(HandlerError::HeaderDecodeFailed))?;
            },
            // Seal([u8;4], Vec<u8>) | PreRuntime([u8;4], Vec<u8>)
            5 | 6 => {
                require!(input.len() >= 4, HandlerError::HeaderDecodeFailed);
                input = &input[4..];
                let _ = Vec::<u8>::decode(&mut input)
                    .map_err(|_| error!(HandlerError::HeaderDecodeFailed))?;
            },
            // RuntimeEnvironmentUpdated — no payload
            7 => {},
            _ => return err!(HandlerError::HeaderDecodeFailed),
        }
    }

    Ok(HeaderDigests {
        block_number,
        mmr_root: mmr_root.ok_or(error!(HandlerError::IsmpDigestMissing))?,
        child_trie_root: child_trie_root.ok_or(error!(HandlerError::IsmpDigestMissing))?,
        timestamp_secs: timestamp_secs.ok_or(error!(HandlerError::TimestampDigestMissing))?,
    })
}

pub struct ConsensusUpdate {
    pub new_height: u32,
    /// One entry per parachain header: `(para_id, block_number, mmr_root,
    /// child_trie_root, timestamp_secs)`. The persisted `StateCommitment`
    /// puts `mmr_root` in `overlay_root` and `child_trie_root` in
    /// `state_root`, mirroring EVM `Header.sol::stateCommitment`.
    pub commitments: Vec<(u32, u32, [u8; 32], [u8; 32], u64)>,
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
        let d = extract_header_digests(&h.header)?;
        commitments.push((h.para_id, d.block_number, d.mmr_root, d.child_trie_root, d.timestamp_secs));
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

    fn build_test_header(
        block_number: u32,
        ismp_payload: Option<(&[u8; 32], &[u8; 32])>,
        timestamp: Option<u64>,
        extra_digests: &[(u8, &[u8])],
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0xaau8; 32]); // parent_hash
        bytes.extend_from_slice(&Compact(block_number).encode());
        bytes.extend_from_slice(&[0xbbu8; 32]); // (parachain) state_root — discarded by extractor
        bytes.extend_from_slice(&[0xccu8; 32]); // extrinsics_root

        let mut digest_count: u32 = extra_digests.len() as u32;
        if ismp_payload.is_some() { digest_count += 1; }
        if timestamp.is_some() { digest_count += 1; }
        bytes.extend_from_slice(&Compact(digest_count).encode());

        if let Some((mmr, child)) = ismp_payload {
            bytes.push(4); // Consensus
            bytes.extend_from_slice(b"ISMP");
            let mut data = Vec::with_capacity(64);
            data.extend_from_slice(mmr);
            data.extend_from_slice(child);
            bytes.extend_from_slice(&data.encode());
        }
        if let Some(ts) = timestamp {
            bytes.push(4);
            bytes.extend_from_slice(b"ISTM");
            bytes.extend_from_slice(&ts.encode().encode());
        }
        for (variant, payload) in extra_digests {
            bytes.push(*variant);
            bytes.extend_from_slice(payload);
        }
        bytes
    }

    #[test]
    fn extract_header_digests_pulls_ismp_and_istm_payloads() {
        let mmr = [0x11u8; 32];
        let child = [0x22u8; 32];
        let ts: u64 = 1_700_000_000;
        let header = build_test_header(30_701_354, Some((&mmr, &child)), Some(ts), &[]);

        let d = extract_header_digests(&header).unwrap();
        assert_eq!(d.block_number, 30_701_354);
        assert_eq!(d.mmr_root, mmr);
        assert_eq!(d.child_trie_root, child);
        assert_eq!(d.timestamp_secs, ts);
    }

    #[test]
    fn extract_header_digests_skips_unrelated_consensus_digests() {
        let mmr = [0x11u8; 32];
        let child = [0x22u8; 32];
        let ts: u64 = 42;
        // A foreign Consensus digest with engine_id "aura" + a Seal digest
        // before the ISMP one — the walker must skip both cleanly.
        let mut aura_consensus = Vec::new();
        aura_consensus.extend_from_slice(b"aura");
        aura_consensus.extend_from_slice(&Vec::<u8>::from([9u8; 16].as_slice()).encode());
        let mut seal = Vec::new();
        seal.extend_from_slice(b"BABE");
        seal.extend_from_slice(&Vec::<u8>::from([7u8; 8].as_slice()).encode());

        let header = build_test_header(
            1,
            Some((&mmr, &child)),
            Some(ts),
            &[(4, &aura_consensus), (5, &seal)],
        );

        let d = extract_header_digests(&header).unwrap();
        assert_eq!(d.mmr_root, mmr);
        assert_eq!(d.timestamp_secs, ts);
    }

    #[test]
    fn extract_header_digests_rejects_when_ismp_missing() {
        let header = build_test_header(1, None, Some(0), &[]);
        assert!(extract_header_digests(&header).is_err());
    }

    #[test]
    fn extract_header_digests_rejects_when_timestamp_missing() {
        let header = build_test_header(1, Some((&[0u8; 32], &[0u8; 32])), None, &[]);
        assert!(extract_header_digests(&header).is_err());
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
