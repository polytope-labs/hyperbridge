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

//! # Tempo consensus verifier
//!
//! Tempo (<https://github.com/tempoxyz/tempo>) is a Reth-based EVM chain
//! finalized by commonware's simplex consensus with BLS12-381 threshold
//! certificates (`bls12381_threshold::vrf`, MinSig variant). Every finalized
//! block carries a ~134-byte certificate: a threshold signature pair over the
//! finalize vote and the per-view VRF seed, verifiable against a single static
//! group public key — the "network identity".
//!
//! The identity is the constant term of the threshold public polynomial. Epoch
//! reshares redistribute shares to new validator sets while provably preserving
//! this constant term, so validator churn does not rotate the key. Only a
//! governance-scheduled full DKG produces a new identity, which is committed
//! on-chain: the last block of each epoch embeds the next epoch's
//! `OnchainDkgOutcome` (public polynomial + player set) in its header
//! `extra_data`, and that boundary block is finalized under the previous key.
//! The light client therefore rotates identities by verifying boundary blocks —
//! the same chain-of-trust handoff its full nodes use. To keep that handoff
//! unbreakable, an update may never carry the client across an epoch boundary
//! it has not verified, and a boundary may only change the key if the previous
//! boundary announced a full DKG.
//!
//! # Trust assumptions
//!
//! - **The group secret is the root of trust, across all time.** Because a
//!   reshare preserves the constant term, a quorum of *any* past epoch's shares
//!   reconstructs the same secret and can therefore sign certificates this
//!   client accepts. Security rests on no such quorum ever colluding, not on
//!   the current validator set alone. Retiring a compromised committee requires
//!   a full DKG, and this client only learns of it by verifying the boundary
//!   block that commits it.
//! - **Boundary `extra_data` is trusted to the extent Tempo's block execution
//!   enforces it.** The client checks that the outcome's epoch is the expected
//!   one, that a key change was announced an epoch in advance, and that the new
//!   key is a valid G2 point; beyond that it accepts what a finalized boundary
//!   block commits.
//! - **Networks are separated by their group keys, not by chain id.**
//!   Certificates carry no chain identifier, so a consensus state's `chain_id`
//!   is only meaningful because mainnet and testnet were produced by different
//!   DKG ceremonies and so have different identities.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod bls;
pub mod error;
pub mod primitives;

use ismp::messaging::Keccak256;
use primitive_types::H256;

pub use error::Error;
use primitives::{
	compute_epoch, decode_dkg_outcome, decode_finalization, finalize_payload, is_epoch_boundary,
	seed_payload, ConsensusState, DkgOutcome, Finalization, FraudProof, TempoClientUpdate,
	TempoHeader,
};

/// An identity rotation discovered in a verified epoch-boundary block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityRotation {
	/// The new network identity (compressed G2 group public key).
	pub network_identity: [u8; primitives::G2_LEN],
	/// The epoch from which the new identity verifies certificates.
	pub from_epoch: u64,
}

/// What an epoch-boundary block commits for the epoch that follows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryUpdate {
	/// Set when the boundary's DKG outcome carries a different group key,
	/// i.e. a completed full-DKG rotation.
	pub rotation: Option<IdentityRotation>,
	/// Whether the ceremony running during the next epoch is a full DKG. This
	/// is what authorises the *following* boundary to rotate the key.
	pub full_dkg_pending: bool,
}

/// The outcome of successfully verifying a [`TempoClientUpdate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationResult {
	/// Height of the finalized block.
	pub height: u64,
	/// Hash of the finalized block (the certificate digest).
	pub hash: H256,
	/// The finalized block's state root.
	pub state_root: H256,
	/// The finalized block's timestamp in seconds.
	pub timestamp: u64,
	/// Set when the update is an epoch-boundary block.
	pub boundary: Option<BoundaryUpdate>,
}

/// Verifies a finalization certificate against the given network identity.
///
/// Checks both threshold signatures in the certificate: the finalize vote
/// (over the proposal) and the VRF seed (over the round).
pub fn verify_finalization(
	network_identity: &[u8; primitives::G2_LEN],
	finalization: &Finalization,
) -> Result<(), Error> {
	let public_key = bls::decode_public_key(network_identity)?;
	bls::verify_signature(
		&public_key,
		&finalization.vote_signature,
		&finalize_payload(finalization),
	)?;
	bls::verify_signature(&public_key, &finalization.seed_signature, &seed_payload(finalization))?;
	Ok(())
}

/// Verifies a Tempo consensus update against the trusted consensus state.
///
/// On success returns the newly finalized state commitment, plus an identity
/// rotation if the update is a boundary block that changes the group key.
/// Height monotonicity is the caller's responsibility.
pub fn verify_tempo_update<H: Keccak256>(
	consensus_state: &ConsensusState,
	update: &TempoClientUpdate,
) -> Result<VerificationResult, Error> {
	if consensus_state.epoch_length == 0 {
		return Err(Error::InvalidConsensusState);
	}

	let finalization = decode_finalization(&update.finalization)?;

	// Certificates predating the current identity's activation cannot be
	// soundly verified: a full DKG may have replaced the key they were
	// signed under.
	if finalization.epoch < consensus_state.from_epoch {
		return Err(Error::EpochBeforeIdentity {
			certificate_epoch: finalization.epoch,
			from_epoch: consensus_state.from_epoch,
		});
	}

	verify_finalization(&consensus_state.network_identity, &finalization)?;

	// Bind the supplied header to the finalized digest.
	let header = TempoHeader::from(&update.header);
	let hash = header.hash::<H>();
	if hash != finalization.digest {
		return Err(Error::DigestMismatch);
	}

	let number = update.header.inner.number.low_u64();

	// An update may not carry the client across an epoch boundary it has not
	// verified. The boundary block is the only place the next epoch's DKG
	// outcome is committed, so without this rule a relayer could hop over a
	// full-DKG rotation and leave the client trusting a key the chain has
	// already retired — forever, since every later certificate would still be
	// checked against it. Tempo's own nodes enforce the same handoff before
	// following a new epoch.
	let current_epoch =
		compute_epoch(consensus_state.finalized_height, consensus_state.epoch_length);
	let update_epoch = compute_epoch(number, consensus_state.epoch_length);
	if update_epoch != current_epoch {
		let at_boundary =
			is_epoch_boundary(consensus_state.finalized_height, consensus_state.epoch_length);
		if !at_boundary || update_epoch != current_epoch + 1 {
			return Err(Error::UnprocessedEpochBoundary {
				finalized_height: consensus_state.finalized_height,
				update_height: number,
			});
		}
	}

	let boundary = if is_epoch_boundary(number, consensus_state.epoch_length) {
		let outcome =
			boundary_dkg_outcome(consensus_state, &update.header.inner.extra_data, number)?;
		let rotation = if outcome.network_identity != consensus_state.network_identity {
			// A reshare provably preserves the group key, so a change means a
			// full DKG completed — and that ceremony must have been announced
			// by the previous boundary. Without this check a single block with
			// adversarial `extra_data` would hand the client to an attacker
			// key permanently and unrecoverably.
			if !consensus_state.full_dkg_pending {
				return Err(Error::UnannouncedIdentityRotation { epoch: outcome.epoch });
			}
			// Refuse to adopt a key that cannot verify anything: storing it
			// would brick the client just as effectively as a hostile one.
			let _ = bls::decode_public_key(&outcome.network_identity)
				.map_err(|_| Error::InvalidIdentity)?;
			Some(IdentityRotation {
				network_identity: outcome.network_identity,
				from_epoch: outcome.epoch,
			})
		} else {
			None
		};
		Some(BoundaryUpdate { rotation, full_dkg_pending: outcome.is_next_full_dkg })
	} else {
		None
	};

	Ok(VerificationResult {
		height: number,
		hash,
		state_root: update.header.inner.state_root,
		timestamp: update.header.inner.timestamp,
		boundary,
	})
}

/// Decodes and sanity-checks the DKG outcome carried by an epoch-boundary
/// block at `height`.
fn boundary_dkg_outcome(
	consensus_state: &ConsensusState,
	extra_data: &[u8],
	height: u64,
) -> Result<DkgOutcome, Error> {
	if extra_data.is_empty() {
		return Err(Error::MissingDkgOutcome);
	}
	let outcome = decode_dkg_outcome(extra_data)?;
	// The boundary block of epoch E carries the outcome used for epoch E + 1.
	let expected = compute_epoch(height, consensus_state.epoch_length) + 1;
	if outcome.epoch != expected {
		return Err(Error::DkgOutcomeEpochMismatch { expected, got: outcome.epoch });
	}
	Ok(outcome)
}

/// Verifies a fraud proof: two finalization certificates that both verify under
/// the trusted identity yet provably conflict.
///
/// A conflict is either of:
///
/// - **same round, different digests** — a classic double-finalize; or
/// - **same height, different digests** — two conflicting blocks finalized at
///   different views, which is what a simplex safety violation actually looks
///   like when a Byzantine quorum nullifies its way to a second notarization.
///
/// The second form is why each half carries its header: certificates encode
/// rounds and digests but never heights, so height equivocation cannot be
/// expressed by certificates alone. Each header is bound to its certificate by
/// hash before its height is trusted.
pub fn verify_fraud_proof<H: Keccak256>(
	consensus_state: &ConsensusState,
	proof_1: &FraudProof,
	proof_2: &FraudProof,
) -> Result<(), Error> {
	let finalization_1 = decode_finalization(&proof_1.finalization)?;
	let finalization_2 = decode_finalization(&proof_2.finalization)?;

	let mut heights = [0u64; 2];
	for (index, (finalization, proof)) in
		[(&finalization_1, proof_1), (&finalization_2, proof_2)].into_iter().enumerate()
	{
		if finalization.epoch < consensus_state.from_epoch {
			return Err(Error::EpochBeforeIdentity {
				certificate_epoch: finalization.epoch,
				from_epoch: consensus_state.from_epoch,
			});
		}
		verify_finalization(&consensus_state.network_identity, finalization)?;
		if TempoHeader::from(&proof.header).hash::<H>() != finalization.digest {
			return Err(Error::DigestMismatch);
		}
		heights[index] = proof.header.inner.number.low_u64();
	}

	let same_round =
		finalization_1.epoch == finalization_2.epoch && finalization_1.view == finalization_2.view;
	let conflicting =
		finalization_1.digest != finalization_2.digest && (same_round || heights[0] == heights[1]);
	if !conflicting {
		return Err(Error::InvalidFraudProof);
	}
	Ok(())
}

/// Applies a [`VerificationResult`] to the consensus state.
pub fn apply_update(
	consensus_state: &mut ConsensusState,
	result: &VerificationResult,
) -> Result<(), Error> {
	if result.height <= consensus_state.finalized_height {
		return Err(Error::ExpiredUpdate {
			current: consensus_state.finalized_height,
			update: result.height,
		});
	}
	consensus_state.finalized_height = result.height;
	consensus_state.finalized_hash = result.hash;
	if let Some(boundary) = &result.boundary {
		if let Some(rotation) = &boundary.rotation {
			consensus_state.network_identity = rotation.network_identity;
			consensus_state.from_epoch = rotation.from_epoch;
		}
		consensus_state.full_dkg_pending = boundary.full_dkg_pending;
	}
	Ok(())
}

/// Convenience wrapper: verify an update and apply it to the consensus state,
/// enforcing height monotonicity.
pub fn verify_and_apply<H: Keccak256>(
	consensus_state: &mut ConsensusState,
	update: &TempoClientUpdate,
) -> Result<VerificationResult, Error> {
	let number = update.header.inner.number.low_u64();
	if number <= consensus_state.finalized_height {
		return Err(Error::ExpiredUpdate {
			current: consensus_state.finalized_height,
			update: number,
		});
	}
	let result = verify_tempo_update::<H>(consensus_state, update)?;
	apply_update(consensus_state, &result)?;
	Ok(result)
}

/// Extracts every field of a [`VerificationResult`] the ISMP client needs
/// without exposing internal types.
pub fn decode_update(update: &[u8]) -> Result<TempoClientUpdate, Error> {
	use codec::Decode;
	TempoClientUpdate::decode(&mut &update[..]).map_err(|_| Error::DecodeClientUpdate)
}

/// Decodes a SCALE-encoded [`ConsensusState`].
pub fn decode_consensus_state(state: &[u8]) -> Result<ConsensusState, Error> {
	use codec::Decode;
	ConsensusState::decode(&mut &state[..]).map_err(|_| Error::DecodeConsensusState)
}

#[cfg(test)]
mod tests {
	use super::*;
	use ark_ec::{AffineRepr, CurveGroup};
	use geth_primitives::CodecHeader;
	use primitives::{
		encode_varint, CodecConsensusContext, CodecTempoHeader, FraudProof, G1_LEN, G2_LEN,
	};

	struct TestHost;
	impl Keccak256 for TestHost {
		fn keccak256(bytes: &[u8]) -> H256 {
			alloy_primitives::keccak256(bytes).0.into()
		}
	}

	const FIXTURES: &str = include_str!("../test-data/tempo_fixtures.json");
	const BOUNDARY_HEIGHT: u64 = 32_723_999;
	const REGULAR_HEIGHT: u64 = 32_724_390;

	fn hex_bytes(value: &json::Value) -> Vec<u8> {
		let s = value.as_str().unwrap();
		hex::decode(s.strip_prefix("0x").unwrap_or(s)).unwrap()
	}

	fn hex_h256(value: &json::Value) -> H256 {
		H256::from_slice(&hex_bytes(value))
	}

	fn qty(value: &json::Value) -> u64 {
		let s = value.as_str().unwrap();
		u64::from_str_radix(s.strip_prefix("0x").unwrap(), 16).unwrap()
	}

	fn header_from_fixture(block: &json::Value) -> CodecTempoHeader {
		let ctx = &block["consensusContext"];
		CodecTempoHeader {
			general_gas_limit: qty(&block["mainBlockGeneralGasLimit"]),
			shared_gas_limit: qty(&block["sharedGasLimit"]),
			timestamp_millis_part: qty(&block["timestampMillisPart"]),
			inner: CodecHeader {
				parent_hash: hex_h256(&block["parentHash"]),
				uncle_hash: hex_h256(&block["sha3Uncles"]),
				coinbase: primitive_types::H160::from_slice(&hex_bytes(&block["miner"])),
				state_root: hex_h256(&block["stateRoot"]),
				transactions_root: hex_h256(&block["transactionsRoot"]),
				receipts_root: hex_h256(&block["receiptsRoot"]),
				logs_bloom: ethabi::ethereum_types::Bloom::from_slice(&hex_bytes(
					&block["logsBloom"],
				)),
				difficulty: qty(&block["difficulty"]).into(),
				number: qty(&block["number"]).into(),
				gas_limit: qty(&block["gasLimit"]),
				gas_used: qty(&block["gasUsed"]),
				timestamp: qty(&block["timestamp"]),
				extra_data: hex_bytes(&block["extraData"]),
				mix_hash: hex_h256(&block["mixHash"]),
				nonce: ethabi::ethereum_types::H64::from_slice(&hex_bytes(&block["nonce"])),
				base_fee_per_gas: Some(qty(&block["baseFeePerGas"]).into()),
				withdrawals_hash: Some(hex_h256(&block["withdrawalsRoot"])),
				blob_gas_used: Some(qty(&block["blobGasUsed"])),
				excess_blob_gas_used: Some(qty(&block["excessBlobGas"])),
				parent_beacon_root: Some(hex_h256(&block["parentBeaconBlockRoot"])),
				requests_hash: Some(hex_h256(&block["requestsHash"])),
			},
			consensus_context: Some(CodecConsensusContext {
				epoch: ctx["epoch"].as_u64().unwrap(),
				view: ctx["view"].as_u64().unwrap(),
				parent_view: ctx["parentView"].as_u64().unwrap(),
				proposer: hex_h256(&ctx["proposer"]),
			}),
		}
	}

	fn fixtures() -> json::Value {
		json::from_str(FIXTURES).unwrap()
	}

	fn mainnet_identity(fixtures: &json::Value) -> [u8; G2_LEN] {
		hex::decode(fixtures["mainnet_identity"].as_str().unwrap())
			.unwrap()
			.try_into()
			.unwrap()
	}

	/// A consensus state trusting the mainnet identity, positioned in the same
	/// epoch as `finalized_height` so updates within that epoch are admissible.
	fn mainnet_state(fixtures: &json::Value, finalized_height: u64) -> ConsensusState {
		ConsensusState {
			network_identity: mainnet_identity(fixtures),
			from_epoch: 0,
			finalized_height,
			finalized_hash: Default::default(),
			epoch_length: fixtures["epoch_length"].as_u64().unwrap(),
			chain_id: fixtures["chain_id"].as_u64().unwrap() as u32,
			full_dkg_pending: false,
		}
	}

	fn update_from(fixtures: &json::Value, name: &str) -> TempoClientUpdate {
		TempoClientUpdate {
			finalization: hex_bytes(&fixtures[name]["finalization"]),
			header: header_from_fixture(&fixtures[name]["block"]),
		}
	}

	// ---------------------------------------------------------------------
	// Synthetic threshold key.
	//
	// A recovered threshold signature is indistinguishable from a plain BLS
	// signature under the group key, so a 1-of-1 "group secret" is sufficient
	// to build certificates for paths mainnet has never exercised (identity
	// rotations, equivocations).
	// ---------------------------------------------------------------------

	struct GroupKey {
		secret: ark_bls12_381::Fr,
		identity: [u8; G2_LEN],
	}

	impl GroupKey {
		fn new(seed: u64) -> Self {
			let secret = ark_bls12_381::Fr::from(seed);
			let public = (ark_bls12_381::G2Affine::generator() * secret).into_affine();
			let identity = ::bls::point_to_signature(public).try_into().unwrap();
			Self { secret, identity }
		}

		fn sign(&self, payload: &[u8]) -> [u8; G1_LEN] {
			let point = (bls::hash_to_g1(payload).unwrap() * self.secret).into_affine();
			::bls::point_to_pubkey(point).try_into().unwrap()
		}

		/// Builds a finalization certificate over `digest` for the given round.
		fn certify(&self, epoch: u64, view: u64, parent_view: u64, digest: H256) -> Vec<u8> {
			let mut round = encode_varint(epoch);
			round.extend_from_slice(&encode_varint(view));
			let mut proposal = round.clone();
			proposal.extend_from_slice(&encode_varint(parent_view));
			proposal.extend_from_slice(digest.as_bytes());

			let mut finalize_ns = primitives::TEMPO_NAMESPACE.to_vec();
			finalize_ns.extend_from_slice(primitives::FINALIZE_SUFFIX);
			let mut seed_ns = primitives::TEMPO_NAMESPACE.to_vec();
			seed_ns.extend_from_slice(primitives::SEED_SUFFIX);

			let mut out = proposal.clone();
			out.extend_from_slice(&self.sign(&primitives::union_unique(&finalize_ns, &proposal)));
			out.extend_from_slice(&self.sign(&primitives::union_unique(&seed_ns, &round)));
			out
		}
	}

	/// Rewrites a fixture boundary header's DKG outcome so it commits
	/// `identity` for `epoch`, announcing `is_next_full_dkg` for the one after.
	fn boundary_header_with(
		fixtures: &json::Value,
		epoch: u64,
		identity: &[u8; G2_LEN],
		is_next_full_dkg: bool,
	) -> CodecTempoHeader {
		let mut header = header_from_fixture(&fixtures["boundary"]["block"]);
		let original = header.inner.extra_data.clone();
		// The identity is the polynomial's constant term; locate it by matching
		// the outcome the fixture already carries, then swap in the new epoch.
		let old_identity = decode_dkg_outcome(&original).unwrap().network_identity;
		let offset = original
			.windows(G2_LEN)
			.position(|window| window == old_identity)
			.expect("fixture contains its identity");

		let mut rewritten = Vec::new();
		rewritten.extend_from_slice(&encode_varint(epoch));
		// skip the original epoch varint, keep everything up to the identity
		let mut cursor = 0usize;
		while original[cursor] & 0x80 != 0 {
			cursor += 1;
		}
		cursor += 1;
		rewritten.extend_from_slice(&original[cursor..offset]);
		rewritten.extend_from_slice(identity);
		rewritten.extend_from_slice(&original[offset + G2_LEN..original.len() - 1]);
		rewritten.push(u8::from(is_next_full_dkg));

		header.inner.extra_data = rewritten;
		header
	}

	/// Seals a header under `key` at the given round, returning a ready update.
	fn signed_update(
		key: &GroupKey,
		header: CodecTempoHeader,
		epoch: u64,
		view: u64,
	) -> TempoClientUpdate {
		let digest = TempoHeader::from(&header).hash::<TestHost>();
		TempoClientUpdate { finalization: key.certify(epoch, view, view - 1, digest), header }
	}

	// ---------------------------------------------------------------------
	// Mainnet vectors
	// ---------------------------------------------------------------------

	#[test]
	fn verifies_mainnet_finalization() {
		let fixtures = fixtures();
		let mut state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);

		let update = update_from(&fixtures, "regular");
		let result = verify_and_apply::<TestHost>(&mut state, &update).unwrap();
		assert_eq!(result.height, REGULAR_HEIGHT);
		assert_eq!(result.boundary, None);
		assert_eq!(state.finalized_height, REGULAR_HEIGHT);
		assert_eq!(state.finalized_hash, result.hash);
	}

	#[test]
	fn verifies_mainnet_boundary_block_and_dkg_outcome() {
		let fixtures = fixtures();
		let mut state = mainnet_state(&fixtures, BOUNDARY_HEIGHT - 1);

		let update = update_from(&fixtures, "boundary");
		let result = verify_and_apply::<TestHost>(&mut state, &update).unwrap();
		assert_eq!(result.height, BOUNDARY_HEIGHT);
		// A reshare preserves the identity, so no rotation is signalled, but the
		// boundary is still recorded (and its full-DKG announcement stored).
		let boundary = result.boundary.expect("boundary recognised");
		assert_eq!(boundary.rotation, None);
		assert!(!boundary.full_dkg_pending);
		assert!(!state.full_dkg_pending);

		let outcome = decode_dkg_outcome(&update.header.inner.extra_data).unwrap();
		assert_eq!(outcome.epoch, 1515);
		assert_eq!(outcome.network_identity, state.network_identity);
		assert!(!outcome.is_next_full_dkg);
		assert_eq!(outcome.participants, 15);
	}

	#[test]
	fn header_hash_matches_mainnet_block_hash() {
		let fixtures = fixtures();
		for name in ["regular", "boundary"] {
			let header = header_from_fixture(&fixtures[name]["block"]);
			let hash = TempoHeader::from(&header).hash::<TestHost>();
			assert_eq!(hash, hex_h256(&fixtures[name]["block"]["hash"]), "{name}");
		}
	}

	// ---------------------------------------------------------------------
	// Rejections
	// ---------------------------------------------------------------------

	#[test]
	fn rejects_tampered_header() {
		let fixtures = fixtures();
		let state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);

		let mut update = update_from(&fixtures, "regular");
		update.header.inner.state_root = H256::repeat_byte(0xde);
		let err = verify_tempo_update::<TestHost>(&state, &update).unwrap_err();
		assert_eq!(err, Error::DigestMismatch);
	}

	#[test]
	fn rejects_tampered_signature() {
		let fixtures = fixtures();
		let state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);

		let mut update = update_from(&fixtures, "regular");
		let len = update.finalization.len();
		update.finalization[len - 96] ^= 0x01;
		let err = verify_tempo_update::<TestHost>(&state, &update).unwrap_err();
		assert!(matches!(err, Error::SignatureVerificationFailed | Error::InvalidPoint));
	}

	#[test]
	fn rejects_tampered_seed_signature() {
		let fixtures = fixtures();
		let state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);

		let mut update = update_from(&fixtures, "regular");
		let len = update.finalization.len();
		update.finalization[len - 1] ^= 0x01;
		let err = verify_tempo_update::<TestHost>(&state, &update).unwrap_err();
		assert!(matches!(err, Error::SignatureVerificationFailed | Error::InvalidPoint));
	}

	#[test]
	fn rejects_wrong_identity() {
		let fixtures = fixtures();
		let mut state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);
		state.network_identity = GroupKey::new(7).identity;

		let update = update_from(&fixtures, "regular");
		let err = verify_tempo_update::<TestHost>(&state, &update).unwrap_err();
		assert_eq!(err, Error::SignatureVerificationFailed);
	}

	#[test]
	fn rejects_identity_at_infinity() {
		let fixtures = fixtures();
		let mut state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);
		// compressed G2 point at infinity: the infinity bit set, rest zero
		let mut infinity = [0u8; G2_LEN];
		infinity[0] = 0xc0;
		state.network_identity = infinity;

		let update = update_from(&fixtures, "regular");
		let err = verify_tempo_update::<TestHost>(&state, &update).unwrap_err();
		assert_eq!(err, Error::InvalidPoint);
	}

	#[test]
	fn rejects_stale_update() {
		let fixtures = fixtures();
		let mut state = mainnet_state(&fixtures, 40_000_000);

		let update = update_from(&fixtures, "regular");
		let err = verify_and_apply::<TestHost>(&mut state, &update).unwrap_err();
		assert!(matches!(err, Error::ExpiredUpdate { .. }));
	}

	#[test]
	fn rejects_epoch_before_identity_activation() {
		let fixtures = fixtures();
		let mut state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);
		state.from_epoch = 2000;

		let update = update_from(&fixtures, "regular");
		let err = verify_tempo_update::<TestHost>(&state, &update).unwrap_err();
		assert!(matches!(err, Error::EpochBeforeIdentity { .. }));
	}

	#[test]
	fn rejects_zero_epoch_length() {
		let fixtures = fixtures();
		let mut state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);
		state.epoch_length = 0;

		let update = update_from(&fixtures, "regular");
		let err = verify_tempo_update::<TestHost>(&state, &update).unwrap_err();
		assert_eq!(err, Error::InvalidConsensusState);
	}

	// ---------------------------------------------------------------------
	// Epoch boundary discipline
	// ---------------------------------------------------------------------

	#[test]
	fn rejects_update_that_skips_an_epoch_boundary() {
		let fixtures = fixtures();
		// Client sits mid-epoch 1514; the regular fixture is in epoch 1515, so
		// accepting it would hop over the boundary that commits epoch 1515's key.
		let state = mainnet_state(&fixtures, BOUNDARY_HEIGHT - 10);

		let update = update_from(&fixtures, "regular");
		let err = verify_tempo_update::<TestHost>(&state, &update).unwrap_err();
		assert!(matches!(err, Error::UnprocessedEpochBoundary { .. }));
	}

	#[test]
	fn accepts_next_epoch_once_the_boundary_is_processed() {
		let fixtures = fixtures();
		let mut state = mainnet_state(&fixtures, BOUNDARY_HEIGHT - 1);

		// Boundary first, then the next epoch's block is admissible.
		verify_and_apply::<TestHost>(&mut state, &update_from(&fixtures, "boundary")).unwrap();
		assert_eq!(state.finalized_height, BOUNDARY_HEIGHT);
		let result =
			verify_and_apply::<TestHost>(&mut state, &update_from(&fixtures, "regular")).unwrap();
		assert_eq!(result.height, REGULAR_HEIGHT);
	}

	#[test]
	fn rejects_update_two_epochs_ahead_even_from_a_boundary() {
		let fixtures = fixtures();
		// Positioned on the boundary of epoch 1512 — two epochs behind.
		let mut state = mainnet_state(&fixtures, 1513 * 21600 - 1);
		state.epoch_length = 21600;

		let update = update_from(&fixtures, "regular");
		let err = verify_tempo_update::<TestHost>(&state, &update).unwrap_err();
		assert!(matches!(err, Error::UnprocessedEpochBoundary { .. }));
	}

	// ---------------------------------------------------------------------
	// Identity rotation
	// ---------------------------------------------------------------------

	#[test]
	fn rotates_identity_on_announced_full_dkg() {
		let fixtures = fixtures();
		let old_key = GroupKey::new(11);
		let new_key = GroupKey::new(22);

		let mut state = mainnet_state(&fixtures, BOUNDARY_HEIGHT - 1);
		state.network_identity = old_key.identity;
		// The previous boundary announced the full DKG running this epoch.
		state.full_dkg_pending = true;

		let header = boundary_header_with(&fixtures, 1515, &new_key.identity, false);
		let update = signed_update(&old_key, header, 1514, 21600);

		let result = verify_and_apply::<TestHost>(&mut state, &update).unwrap();
		let rotation = result.boundary.unwrap().rotation.expect("rotation signalled");
		assert_eq!(rotation.network_identity, new_key.identity);
		assert_eq!(rotation.from_epoch, 1515);
		// The state now trusts the new key, and the announcement is cleared.
		assert_eq!(state.network_identity, new_key.identity);
		assert_eq!(state.from_epoch, 1515);
		assert!(!state.full_dkg_pending);

		// A certificate under the retired key is no longer accepted.
		let mut header = header_from_fixture(&fixtures["regular"]["block"]);
		header.inner.state_root = H256::repeat_byte(0x11);
		let stale = signed_update(&old_key, header.clone(), 1515, 400);
		assert_eq!(
			verify_tempo_update::<TestHost>(&state, &stale).unwrap_err(),
			Error::SignatureVerificationFailed
		);
		// ...while one under the new key is.
		let fresh = signed_update(&new_key, header, 1515, 400);
		assert_eq!(verify_tempo_update::<TestHost>(&state, &fresh).unwrap().height, REGULAR_HEIGHT);
	}

	#[test]
	fn rejects_unannounced_identity_rotation() {
		let fixtures = fixtures();
		let old_key = GroupKey::new(11);
		let attacker = GroupKey::new(99);

		let mut state = mainnet_state(&fixtures, BOUNDARY_HEIGHT - 1);
		state.network_identity = old_key.identity;
		// No full DKG was announced by the previous boundary.
		state.full_dkg_pending = false;

		let header = boundary_header_with(&fixtures, 1515, &attacker.identity, false);
		let update = signed_update(&old_key, header, 1514, 21600);

		let err = verify_and_apply::<TestHost>(&mut state, &update).unwrap_err();
		assert!(matches!(err, Error::UnannouncedIdentityRotation { epoch: 1515 }));
		// The state is untouched.
		assert_eq!(state.network_identity, old_key.identity);
		assert_eq!(state.finalized_height, BOUNDARY_HEIGHT - 1);
	}

	#[test]
	fn rejects_rotation_to_an_invalid_identity() {
		let fixtures = fixtures();
		let old_key = GroupKey::new(11);

		let mut state = mainnet_state(&fixtures, BOUNDARY_HEIGHT - 1);
		state.network_identity = old_key.identity;
		state.full_dkg_pending = true;

		// Not a valid compressed G2 encoding.
		let garbage = [0xaau8; G2_LEN];
		let header = boundary_header_with(&fixtures, 1515, &garbage, false);
		let update = signed_update(&old_key, header, 1514, 21600);

		let err = verify_and_apply::<TestHost>(&mut state, &update).unwrap_err();
		assert_eq!(err, Error::InvalidIdentity);
		assert_eq!(state.network_identity, old_key.identity);
	}

	#[test]
	fn boundary_records_full_dkg_announcement() {
		let fixtures = fixtures();
		let key = GroupKey::new(11);
		let mut state = mainnet_state(&fixtures, BOUNDARY_HEIGHT - 1);
		state.network_identity = key.identity;

		// Same identity (a reshare) but announcing a full DKG for the next epoch.
		let header = boundary_header_with(&fixtures, 1515, &key.identity, true);
		let update = signed_update(&key, header, 1514, 21600);

		let result = verify_and_apply::<TestHost>(&mut state, &update).unwrap();
		assert_eq!(result.boundary.as_ref().unwrap().rotation, None);
		assert!(state.full_dkg_pending, "announcement recorded for the next boundary");
	}

	#[test]
	fn rejects_boundary_with_wrong_outcome_epoch() {
		let fixtures = fixtures();
		let key = GroupKey::new(11);
		let mut state = mainnet_state(&fixtures, BOUNDARY_HEIGHT - 1);
		state.network_identity = key.identity;

		// The boundary of epoch 1514 must commit the outcome for epoch 1515.
		let header = boundary_header_with(&fixtures, 1600, &key.identity, false);
		let update = signed_update(&key, header, 1514, 21600);

		let err = verify_and_apply::<TestHost>(&mut state, &update).unwrap_err();
		assert!(matches!(err, Error::DkgOutcomeEpochMismatch { expected: 1515, got: 1600 }));
	}

	#[test]
	fn rejects_boundary_without_dkg_outcome() {
		let fixtures = fixtures();
		let key = GroupKey::new(11);
		let mut state = mainnet_state(&fixtures, BOUNDARY_HEIGHT - 1);
		state.network_identity = key.identity;

		let mut header = header_from_fixture(&fixtures["boundary"]["block"]);
		header.inner.extra_data = Vec::new();
		let update = signed_update(&key, header, 1514, 21600);

		let err = verify_and_apply::<TestHost>(&mut state, &update).unwrap_err();
		assert_eq!(err, Error::MissingDkgOutcome);
	}

	#[test]
	fn boundary_outcome_tolerates_trailing_bytes() {
		// Tempo nodes decode extra_data prefix-only, so a finalized boundary
		// block may carry trailing bytes; rejecting it would freeze rotation.
		let fixtures = fixtures();
		let mut extra = hex_bytes(&fixtures["boundary"]["block"]["extraData"]);
		let expected = decode_dkg_outcome(&extra).unwrap();
		extra.extend_from_slice(&[0xff, 0x00, 0x42]);
		assert_eq!(decode_dkg_outcome(&extra).unwrap(), expected);
	}

	// ---------------------------------------------------------------------
	// Fraud proofs
	// ---------------------------------------------------------------------

	fn fraud_proof(key: &GroupKey, header: CodecTempoHeader, epoch: u64, view: u64) -> FraudProof {
		let update = signed_update(key, header, epoch, view);
		FraudProof { finalization: update.finalization, header: update.header }
	}

	#[test]
	fn fraud_proof_accepts_same_round_different_digests() {
		let fixtures = fixtures();
		let key = GroupKey::new(11);
		let mut state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);
		state.network_identity = key.identity;

		let mut header_a = header_from_fixture(&fixtures["regular"]["block"]);
		header_a.inner.state_root = H256::repeat_byte(0xaa);
		let mut header_b = header_from_fixture(&fixtures["regular"]["block"]);
		header_b.inner.state_root = H256::repeat_byte(0xbb);

		let proof_1 = fraud_proof(&key, header_a, 1515, 391);
		let proof_2 = fraud_proof(&key, header_b, 1515, 391);
		verify_fraud_proof::<TestHost>(&state, &proof_1, &proof_2).unwrap();
	}

	#[test]
	fn fraud_proof_accepts_same_height_different_views() {
		// A simplex safety violation need not be a same-round double vote: a
		// Byzantine quorum can nullify its way to a second notarization of a
		// conflicting block at the same height in a later view.
		let fixtures = fixtures();
		let key = GroupKey::new(11);
		let mut state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);
		state.network_identity = key.identity;

		let mut header_a = header_from_fixture(&fixtures["regular"]["block"]);
		header_a.inner.state_root = H256::repeat_byte(0xaa);
		let mut header_b = header_from_fixture(&fixtures["regular"]["block"]);
		header_b.inner.state_root = H256::repeat_byte(0xbb);

		let proof_1 = fraud_proof(&key, header_a, 1515, 100);
		let proof_2 = fraud_proof(&key, header_b, 1515, 137);
		verify_fraud_proof::<TestHost>(&state, &proof_1, &proof_2).unwrap();
	}

	#[test]
	fn fraud_proof_rejects_identical_certificates() {
		let fixtures = fixtures();
		let state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);
		let update = update_from(&fixtures, "regular");
		let proof = FraudProof { finalization: update.finalization, header: update.header };
		let err = verify_fraud_proof::<TestHost>(&state, &proof, &proof).unwrap_err();
		assert_eq!(err, Error::InvalidFraudProof);
	}

	#[test]
	fn fraud_proof_rejects_distinct_blocks_at_different_heights() {
		// Two honestly finalized blocks at different heights and rounds are not
		// evidence of equivocation.
		let fixtures = fixtures();
		let state = mainnet_state(&fixtures, BOUNDARY_HEIGHT - 1);
		let boundary = update_from(&fixtures, "boundary");
		let regular = update_from(&fixtures, "regular");
		let proof_1 = FraudProof { finalization: boundary.finalization, header: boundary.header };
		let proof_2 = FraudProof { finalization: regular.finalization, header: regular.header };
		let err = verify_fraud_proof::<TestHost>(&state, &proof_1, &proof_2).unwrap_err();
		assert_eq!(err, Error::InvalidFraudProof);
	}

	#[test]
	fn fraud_proof_rejects_header_not_matching_certificate() {
		let fixtures = fixtures();
		let key = GroupKey::new(11);
		let mut state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);
		state.network_identity = key.identity;

		let mut header_a = header_from_fixture(&fixtures["regular"]["block"]);
		header_a.inner.state_root = H256::repeat_byte(0xaa);
		let mut proof_1 = fraud_proof(&key, header_a.clone(), 1515, 391);
		// swap in a header the certificate does not commit to
		proof_1.header.inner.state_root = H256::repeat_byte(0xcc);

		let mut header_b = header_from_fixture(&fixtures["regular"]["block"]);
		header_b.inner.state_root = H256::repeat_byte(0xbb);
		let proof_2 = fraud_proof(&key, header_b, 1515, 391);

		let err = verify_fraud_proof::<TestHost>(&state, &proof_1, &proof_2).unwrap_err();
		assert_eq!(err, Error::DigestMismatch);
	}

	#[test]
	fn fraud_proof_rejects_unverifiable_certificate() {
		let fixtures = fixtures();
		let attacker = GroupKey::new(99);
		let key = GroupKey::new(11);
		let mut state = mainnet_state(&fixtures, REGULAR_HEIGHT - 1);
		state.network_identity = key.identity;

		let mut header_a = header_from_fixture(&fixtures["regular"]["block"]);
		header_a.inner.state_root = H256::repeat_byte(0xaa);
		let mut header_b = header_from_fixture(&fixtures["regular"]["block"]);
		header_b.inner.state_root = H256::repeat_byte(0xbb);

		let proof_1 = fraud_proof(&attacker, header_a, 1515, 391);
		let proof_2 = fraud_proof(&key, header_b, 1515, 391);
		let err = verify_fraud_proof::<TestHost>(&state, &proof_1, &proof_2).unwrap_err();
		assert_eq!(err, Error::SignatureVerificationFailed);
	}

	// ---------------------------------------------------------------------
	// Decoders
	// ---------------------------------------------------------------------

	#[test]
	fn rejects_truncated_finalization() {
		let fixtures = fixtures();
		let bytes = hex_bytes(&fixtures["regular"]["finalization"]);
		for cut in [1usize, 10, 40, bytes.len() - 1] {
			assert!(matches!(decode_finalization(&bytes[..cut]), Err(Error::UnexpectedEndOfInput)));
		}
	}

	#[test]
	fn rejects_finalization_with_trailing_bytes() {
		let fixtures = fixtures();
		let mut bytes = hex_bytes(&fixtures["regular"]["finalization"]);
		bytes.push(0x00);
		assert_eq!(decode_finalization(&bytes).unwrap_err(), Error::TrailingBytes);
	}

	#[test]
	fn rejects_non_minimal_varint_in_finalization() {
		let fixtures = fixtures();
		let bytes = hex_bytes(&fixtures["regular"]["finalization"]);
		// re-encode the leading epoch varint non-minimally (0x80 0x00 == 0)
		let mut tampered = vec![0x80, 0x00];
		let mut cursor = 0usize;
		while bytes[cursor] & 0x80 != 0 {
			cursor += 1;
		}
		tampered.extend_from_slice(&bytes[cursor + 1..]);
		assert_eq!(decode_finalization(&tampered).unwrap_err(), Error::NonMinimalVarint);
	}

	#[test]
	fn rejects_malformed_dkg_outcomes() {
		let fixtures = fixtures();
		let extra = hex_bytes(&fixtures["boundary"]["block"]["extraData"]);
		// truncations at various structural points
		for cut in [1usize, 33, 38, 40, 200, extra.len() - 1] {
			assert!(
				decode_dkg_outcome(&extra[..cut]).is_err(),
				"truncation at {cut} should not decode"
			);
		}
		// an absurd polynomial length must not overflow or allocate
		let mut cursor = 0usize;
		while extra[cursor] & 0x80 != 0 {
			cursor += 1;
		}
		cursor += 1 + 32 + 5;
		let mut bloated = extra[..cursor].to_vec();
		bloated.extend_from_slice(&encode_varint(u64::MAX));
		bloated.extend_from_slice(&extra[cursor + 1..]);
		assert!(matches!(
			decode_dkg_outcome(&bloated),
			Err(Error::LengthOverflow | Error::UnexpectedEndOfInput)
		));
	}

	#[test]
	fn scale_roundtrip() {
		use codec::{Decode, Encode};
		let fixtures = fixtures();
		let update = update_from(&fixtures, "regular");
		let encoded = update.encode();
		let decoded = TempoClientUpdate::decode(&mut &encoded[..]).unwrap();
		assert_eq!(encoded, decoded.encode());

		let state = mainnet_state(&fixtures, REGULAR_HEIGHT);
		assert_eq!(decode_consensus_state(&state.encode()).unwrap(), state);
	}
}
