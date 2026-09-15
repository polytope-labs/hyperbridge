// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Primitives for the Tempo consensus verifier.
//!
//! Tempo finalizes blocks with commonware's simplex consensus using the
//! `bls12381_threshold::vrf` signing scheme (MinSig variant: signatures in G1,
//! public keys in G2). A finalization certificate is a single threshold
//! signature pair, verifiable against the network's static group public key
//! (the "network identity"). This module contains:
//!
//! - The Tempo block header (an RLP wrapper around a standard Ethereum header)
//! - Decoders for the commonware wire formats a light client consumes: the `Finalization`
//!   certificate and the `OnchainDkgOutcome` embedded in epoch-boundary block headers.

use alloc::vec::Vec;
use alloy_primitives::B256;
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use codec::{Decode, Encode};
use geth_primitives::{CodecHeader, Header};
use ismp::messaging::Keccak256;
use primitive_types::H256;

use crate::error::Error;

/// Base namespace used by Tempo for consensus signature domain separation.
pub const TEMPO_NAMESPACE: &[u8] = b"TEMPO";
/// Suffix appended to the namespace for finalize votes.
pub const FINALIZE_SUFFIX: &[u8] = b"_FINALIZE";
/// Suffix appended to the namespace for per-view seed (VRF) signatures.
pub const SEED_SUFFIX: &[u8] = b"_SEED";

/// Compressed BLS12-381 G1 point length (signatures under MinSig).
pub const G1_LEN: usize = 48;
/// Compressed BLS12-381 G2 point length (public keys under MinSig).
pub const G2_LEN: usize = 96;
/// A threshold certificate is a (vote, seed) signature pair.
pub const CERTIFICATE_LEN: usize = 2 * G1_LEN;
/// Ed25519 public key length (validator identity keys).
const ED25519_LEN: usize = 32;
/// Length of the blake3 transcript summary in the DKG output.
const SUMMARY_LEN: usize = 32;

/// Tempo mainnet ("Presto") EVM chain id.
pub const TEMPO_MAINNET_CHAIN_ID: u32 = 4217;
/// Tempo testnet ("Moderato") EVM chain id.
pub const TEMPO_TESTNET_CHAIN_ID: u32 = 42431;

/// Scale-encodable consensus state for the Tempo light client.
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct ConsensusState {
	/// The network identity: compressed BLS12-381 G2 group public key. This is
	/// the constant term of the threshold public polynomial and remains fixed
	/// across epoch reshares; it only changes when governance schedules a full
	/// DKG ceremony.
	pub network_identity: [u8; G2_LEN],
	/// First epoch for which `network_identity` verifies certificates.
	pub from_epoch: u64,
	/// Latest finalized block height.
	pub finalized_height: u64,
	/// Latest finalized block hash.
	pub finalized_hash: H256,
	/// Number of blocks per epoch. A block at height `h` is an epoch boundary
	/// iff `(h + 1) % epoch_length == 0`; boundary headers carry the DKG
	/// outcome for the next epoch in `extra_data`.
	pub epoch_length: u64,
	/// EVM chain id of the tracked Tempo network.
	pub chain_id: u32,
	/// Whether the DKG ceremony running during the current epoch is a full one
	/// (which mints a brand-new group key) rather than a reshare.
	///
	/// Read from the `is_next_full_dkg` flag of the most recently verified
	/// boundary block. A boundary is only allowed to change
	/// [`Self::network_identity`] when this is set, so every rotation is
	/// announced on-chain a full epoch in advance and an unannounced one is
	/// rejected outright.
	pub full_dkg_pending: bool,
}

impl Default for ConsensusState {
	fn default() -> Self {
		Self {
			network_identity: [0u8; G2_LEN],
			from_epoch: 0,
			finalized_height: 0,
			finalized_hash: H256::zero(),
			epoch_length: 0,
			chain_id: 0,
			full_dkg_pending: false,
		}
	}
}

/// A consensus update submitted by a relayer: a raw commonware finalization
/// certificate plus the pre-image of the block digest it finalizes.
#[derive(Encode, Decode, Debug, Clone, scale_info::TypeInfo)]
pub struct TempoClientUpdate {
	/// Raw commonware-codec encoded `Finalization<Scheme, Digest>` bytes, as
	/// returned by Tempo's `consensus_getFinalization` RPC.
	pub finalization: Vec<u8>,
	/// The Tempo block header whose hash is the finalized digest.
	pub header: CodecTempoHeader,
}

/// Consensus context metadata appended to Tempo block headers.
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
pub struct ConsensusContext {
	pub epoch: u64,
	pub view: u64,
	pub parent_view: u64,
	pub proposer: B256,
}

/// Tempo block header.
///
/// RLP-encoded as `[general_gas_limit, shared_gas_limit, timestamp_millis_part,
/// inner, consensus_context?]` where `inner` is a standard Ethereum header. The
/// block hash (and thus the digest signed by consensus) is
/// `keccak256(rlp(TempoHeader))`.
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct TempoHeader {
	/// Non-payment gas limit for the block.
	pub general_gas_limit: u64,
	/// Gas limit allocated for the subblocks section of the block.
	pub shared_gas_limit: u64,
	/// Sub-second (milliseconds) portion of the timestamp.
	pub timestamp_millis_part: u64,
	/// Inner Ethereum header.
	pub inner: Header,
	/// Consensus metadata. `None` for pre-fork blocks.
	pub consensus_context: Option<ConsensusContext>,
}

impl TempoHeader {
	pub fn hash<H: Keccak256>(self) -> H256 {
		let encoding = alloy_rlp::encode(self);
		H::keccak256(&encoding)
	}
}

/// Scale-encodable mirror of [`TempoHeader`].
#[derive(Encode, Decode, Debug, Clone, scale_info::TypeInfo)]
pub struct CodecTempoHeader {
	pub general_gas_limit: u64,
	pub shared_gas_limit: u64,
	pub timestamp_millis_part: u64,
	pub inner: CodecHeader,
	pub consensus_context: Option<CodecConsensusContext>,
}

/// Scale-encodable mirror of [`ConsensusContext`].
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct CodecConsensusContext {
	pub epoch: u64,
	pub view: u64,
	pub parent_view: u64,
	pub proposer: H256,
}

impl From<&CodecTempoHeader> for TempoHeader {
	fn from(value: &CodecTempoHeader) -> Self {
		TempoHeader {
			general_gas_limit: value.general_gas_limit,
			shared_gas_limit: value.shared_gas_limit,
			timestamp_millis_part: value.timestamp_millis_part,
			inner: (&value.inner).into(),
			consensus_context: value.consensus_context.as_ref().map(|ctx| ConsensusContext {
				epoch: ctx.epoch,
				view: ctx.view,
				parent_view: ctx.parent_view,
				proposer: ctx.proposer.0.into(),
			}),
		}
	}
}

/// A decoded commonware `Finalization` certificate.
///
/// Wire format (commonware-codec):
/// `varint(epoch) || varint(view) || varint(parent_view) || digest[32] ||
/// vote_signature[48] || seed_signature[48]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finalization {
	/// Epoch of the finalized round.
	pub epoch: u64,
	/// View of the finalized round.
	pub view: u64,
	/// View of the parent proposal.
	pub parent_view: u64,
	/// The finalized payload digest: the Tempo block hash.
	pub digest: H256,
	/// Threshold BLS signature (G1) over the finalize vote.
	pub vote_signature: [u8; G1_LEN],
	/// Threshold BLS signature (G1) over the per-view seed.
	pub seed_signature: [u8; G1_LEN],
	/// The exact proposal bytes covered by the vote signature:
	/// `varint(epoch) || varint(view) || varint(parent_view) || digest`.
	pub proposal_bytes: Vec<u8>,
	/// The exact round bytes covered by the seed signature:
	/// `varint(epoch) || varint(view)`.
	pub round_bytes: Vec<u8>,
}

/// Reads a LEB128 varint from `bytes` at `offset`, advancing the offset.
///
/// Non-minimal encodings (a continuation byte chain whose final byte
/// contributes no bits) are rejected, matching `commonware_codec`'s decoder —
/// otherwise the same logical value would have several encodings, and the
/// signed `proposal_bytes`/`round_bytes` slices this decoder reconstructs
/// would not be canonical.
fn read_varint(bytes: &[u8], offset: &mut usize) -> Result<u64, Error> {
	let mut value = 0u64;
	let mut shift = 0u32;
	loop {
		let byte = *bytes.get(*offset).ok_or(Error::UnexpectedEndOfInput)?;
		*offset += 1;
		if shift >= 64 || (shift == 63 && byte > 1) {
			return Err(Error::VarintOverflow);
		}
		value |= u64::from(byte & 0x7f) << shift;
		if byte & 0x80 == 0 {
			if shift > 0 && byte == 0 {
				return Err(Error::NonMinimalVarint);
			}
			return Ok(value);
		}
		shift += 7;
	}
}

/// Reads a varint used as a length or count, rejecting values that do not fit
/// in `usize`. The runtime targets 32-bit wasm, where a bare `as usize` cast
/// would silently truncate and could turn an over-long declared length into a
/// small one.
fn read_length(bytes: &[u8], offset: &mut usize) -> Result<usize, Error> {
	let value = read_varint(bytes, offset)?;
	usize::try_from(value).map_err(|_| Error::LengthOverflow)
}

/// Encodes a LEB128 varint.
pub fn encode_varint(mut value: u64) -> Vec<u8> {
	let mut out = Vec::with_capacity(10);
	while value >= 0x80 {
		out.push((value as u8 & 0x7f) | 0x80);
		value >>= 7;
	}
	out.push(value as u8);
	out
}

/// `commonware_utils::union_unique`: concatenates a namespace and message,
/// prefixed with a varint encoding of the namespace length. This is the
/// payload that commonware BLS operations sign.
pub fn union_unique(namespace: &[u8], message: &[u8]) -> Vec<u8> {
	let mut out = Vec::with_capacity(1 + namespace.len() + message.len());
	out.extend_from_slice(&encode_varint(namespace.len() as u64));
	out.extend_from_slice(namespace);
	out.extend_from_slice(message);
	out
}

/// Decodes a commonware `Finalization<Scheme, Digest>`; rejects trailing bytes.
pub fn decode_finalization(bytes: &[u8]) -> Result<Finalization, Error> {
	let mut cursor = 0usize;
	let epoch = read_varint(bytes, &mut cursor)?;
	let view = read_varint(bytes, &mut cursor)?;
	let round_end = cursor;
	let parent_view = read_varint(bytes, &mut cursor)?;
	let proposal_end = cursor + 32;
	let digest: [u8; 32] = bytes
		.get(cursor..proposal_end)
		.ok_or(Error::UnexpectedEndOfInput)?
		.try_into()
		.expect("slice length is 32; qed");
	cursor = proposal_end;

	let vote_signature: [u8; G1_LEN] = bytes
		.get(cursor..cursor + G1_LEN)
		.ok_or(Error::UnexpectedEndOfInput)?
		.try_into()
		.expect("slice length is 48; qed");
	cursor += G1_LEN;
	let seed_signature: [u8; G1_LEN] = bytes
		.get(cursor..cursor + G1_LEN)
		.ok_or(Error::UnexpectedEndOfInput)?
		.try_into()
		.expect("slice length is 48; qed");
	cursor += G1_LEN;

	if cursor != bytes.len() {
		return Err(Error::TrailingBytes);
	}

	Ok(Finalization {
		epoch,
		view,
		parent_view,
		digest: digest.into(),
		vote_signature,
		seed_signature,
		proposal_bytes: bytes[..proposal_end].to_vec(),
		round_bytes: bytes[..round_end].to_vec(),
	})
}

/// The light-client-relevant summary of an `OnchainDkgOutcome` decoded from an
/// epoch-boundary block's `extra_data`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DkgOutcome {
	/// The epoch for which this outcome's key is used.
	pub epoch: u64,
	/// The group public key for `epoch`: the constant term of the public
	/// polynomial (compressed G2).
	pub network_identity: [u8; G2_LEN],
	/// Number of participants in the committee for `epoch`.
	pub participants: u32,
	/// Whether the ceremony running during `epoch` is a full DKG (which
	/// produces a brand-new group key) instead of a reshare.
	pub is_next_full_dkg: bool,
}

/// Skips a commonware `ordered::Set<ed25519::PublicKey>`: varint length
/// followed by that many 32-byte keys.
fn skip_ed25519_set(bytes: &[u8], offset: &mut usize) -> Result<(), Error> {
	let len = read_length(bytes, offset)?;
	let end = offset
		.checked_add(len.checked_mul(ED25519_LEN).ok_or(Error::LengthOverflow)?)
		.ok_or(Error::LengthOverflow)?;
	if end > bytes.len() {
		return Err(Error::UnexpectedEndOfInput);
	}
	*offset = end;
	Ok(())
}

/// Decodes the `OnchainDkgOutcome` written to epoch-boundary block headers.
///
/// Wire format (commonware-codec):
/// ```text
/// varint(epoch)
/// summary[32]                     // blake3 transcript commitment
/// mode[1] || total[u32 BE]        // Sharing header
/// varint(num_coeffs) || num_coeffs * G2[96]  // public polynomial
/// dealers || players || revealed  // ordered::Set<ed25519 key>, varint-prefixed
/// next_players                    // ordered::Set<ed25519 key>
/// is_next_full_dkg[1]
/// ```
///
/// Trailing bytes are tolerated. Tempo's own nodes decode `extra_data` with
/// commonware's prefix-only `Read`, and block validation compares the *decoded
/// struct* against the locally computed outcome — so a boundary block carrying
/// trailing garbage is a valid, finalized chain state. Rejecting it here would
/// let one Byzantine boundary proposer permanently freeze identity rotation.
/// The bytes are committed under a certificate-verified header hash either way.
pub fn decode_dkg_outcome(bytes: &[u8]) -> Result<DkgOutcome, Error> {
	let mut offset = 0usize;
	let epoch = read_varint(bytes, &mut offset)?;

	// transcript summary
	offset = offset.checked_add(SUMMARY_LEN).ok_or(Error::UnexpectedEndOfInput)?;
	// sharing: mode (1 byte) + total (u32 big-endian)
	let total_end = offset.checked_add(1 + 4).ok_or(Error::UnexpectedEndOfInput)?;
	let total_bytes = bytes.get(offset + 1..total_end).ok_or(Error::UnexpectedEndOfInput)?;
	let participants = u32::from_be_bytes(total_bytes.try_into().expect("slice length is 4; qed"));
	offset = total_end;

	// public polynomial
	let num_coeffs = read_length(bytes, &mut offset)?;
	if num_coeffs == 0 {
		return Err(Error::InvalidDkgOutcome);
	}
	let identity_end = offset.checked_add(G2_LEN).ok_or(Error::UnexpectedEndOfInput)?;
	let network_identity: [u8; G2_LEN] = bytes
		.get(offset..identity_end)
		.ok_or(Error::UnexpectedEndOfInput)?
		.try_into()
		.expect("slice length is 96; qed");
	let poly_end = offset
		.checked_add(num_coeffs.checked_mul(G2_LEN).ok_or(Error::LengthOverflow)?)
		.ok_or(Error::LengthOverflow)?;
	if poly_end > bytes.len() {
		return Err(Error::UnexpectedEndOfInput);
	}
	offset = poly_end;

	// dealers, players, revealed, next_players
	skip_ed25519_set(bytes, &mut offset)?;
	skip_ed25519_set(bytes, &mut offset)?;
	skip_ed25519_set(bytes, &mut offset)?;
	skip_ed25519_set(bytes, &mut offset)?;

	let is_next_full_dkg = match bytes.get(offset) {
		Some(0) => false,
		Some(1) => true,
		_ => return Err(Error::InvalidDkgOutcome),
	};

	Ok(DkgOutcome { epoch, network_identity, participants, is_next_full_dkg })
}

/// The finalize-vote signing payload for a decoded finalization:
/// `union_unique(namespace || "_FINALIZE", proposal_bytes)`.
pub fn finalize_payload(finalization: &Finalization) -> Vec<u8> {
	let mut namespace = Vec::with_capacity(TEMPO_NAMESPACE.len() + FINALIZE_SUFFIX.len());
	namespace.extend_from_slice(TEMPO_NAMESPACE);
	namespace.extend_from_slice(FINALIZE_SUFFIX);
	union_unique(&namespace, &finalization.proposal_bytes)
}

/// The seed signing payload for a decoded finalization:
/// `union_unique(namespace || "_SEED", round_bytes)`.
pub fn seed_payload(finalization: &Finalization) -> Vec<u8> {
	let mut namespace = Vec::with_capacity(TEMPO_NAMESPACE.len() + SEED_SUFFIX.len());
	namespace.extend_from_slice(TEMPO_NAMESPACE);
	namespace.extend_from_slice(SEED_SUFFIX);
	union_unique(&namespace, &finalization.round_bytes)
}

/// Computes the epoch a block at `height` belongs to.
pub fn compute_epoch(height: u64, epoch_length: u64) -> u64 {
	height / epoch_length
}

/// Whether a block at `height` is an epoch boundary block (the last block of
/// its epoch, which must carry the DKG outcome for the next epoch).
pub fn is_epoch_boundary(height: u64, epoch_length: u64) -> bool {
	height.saturating_add(1).is_multiple_of(epoch_length)
}

/// One half of a fraud proof: a finalization certificate together with the
/// header it finalizes.
///
/// The header is required so that equivocation can be proven by *height* as
/// well as by round — a fork where two conflicting blocks at the same height
/// are finalized in different views is a genuine simplex safety violation, but
/// the certificates alone carry only rounds and digests, never heights.
#[derive(Encode, Decode, Debug, Clone, scale_info::TypeInfo)]
pub struct FraudProof {
	/// Raw commonware-codec encoded finalization.
	pub finalization: Vec<u8>,
	/// The header whose hash must equal the certificate's digest.
	pub header: CodecTempoHeader,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn varint_roundtrip() {
		for value in [0u64, 1, 127, 128, 1510, 21600, 12595, u32::MAX as u64, u64::MAX] {
			let encoded = encode_varint(value);
			let mut offset = 0;
			assert_eq!(read_varint(&encoded, &mut offset).unwrap(), value);
			assert_eq!(offset, encoded.len());
		}
	}

	#[test]
	fn varint_overflow_rejected() {
		// 11 bytes of continuation overflows u64
		let bytes = vec![0xff; 11];
		let mut offset = 0;
		assert!(matches!(read_varint(&bytes, &mut offset), Err(Error::VarintOverflow)));
	}

	#[test]
	fn non_minimal_varints_rejected() {
		// Each of these decodes to a value that has a shorter encoding.
		for bytes in [
			vec![0x80, 0x00],             // 0 with one redundant continuation
			vec![0xff, 0x00],             // 127 padded
			vec![0x80, 0x80, 0x00],       // 0 with two
			vec![0x81, 0x80, 0x80, 0x00], // 1 padded
		] {
			let mut offset = 0;
			assert!(
				matches!(read_varint(&bytes, &mut offset), Err(Error::NonMinimalVarint)),
				"{bytes:?} should be rejected"
			);
		}
	}

	#[test]
	fn varint_truncation_rejected() {
		// A continuation bit with no following byte.
		let mut offset = 0;
		assert!(matches!(read_varint(&[0x80], &mut offset), Err(Error::UnexpectedEndOfInput)));
		let mut offset = 0;
		assert!(matches!(read_varint(&[], &mut offset), Err(Error::UnexpectedEndOfInput)));
	}

	#[test]
	fn oversized_length_rejected_on_32_bit_targets() {
		// u64::MAX as a length must never truncate into a small usize.
		let encoded = encode_varint(u64::MAX);
		let mut offset = 0;
		let result = read_length(&encoded, &mut offset);
		#[cfg(target_pointer_width = "64")]
		assert_eq!(result.unwrap(), usize::MAX);
		#[cfg(not(target_pointer_width = "64"))]
		assert!(matches!(result, Err(Error::LengthOverflow)));
	}

	#[test]
	fn union_unique_matches_commonware_framing() {
		// varint(len(ns)) || ns || msg
		assert_eq!(union_unique(b"AB", b"xyz"), b"\x02ABxyz".to_vec());
		assert_eq!(union_unique(b"", b"m"), b"\x00m".to_vec());
	}
}
