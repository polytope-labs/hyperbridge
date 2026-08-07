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

//! Publishes a commitment to the relay chain's BEEFY BLS public keys in this chain's headers.
//!
//! An APK proof binds a prover to a validator set through a single Poseidon2 commitment over the
//! validators' BLS12-381 G1 keys. A client verifying BEEFY finality needs that commitment, and the
//! cheapest trustworthy place to get it is a header this chain already publishes: the relay state
//! proof carried in every parachain block is checked against the relay parent's state root by the
//! validators, so the keys can be read out of it without any new trust assumption.
//!
//! The commitment is expensive. A full 1024-slot set is roughly 420ms of wasm, which does not fit
//! in a block, so it is absorbed a chunk at a time across blocks and published once complete. The
//! authority set for the next session is known a session ahead, which is what makes that possible.
//!
//! What still has to be decided before this is more than a skeleton is marked `DECIDE` below.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

use apk_commitment::{PartialCommitment, NUM_VALIDATORS};
use ark_bls12_381::G1Affine;
use ark_serialize::CanonicalDeserialize;
use codec::{Decode, Encode, MaxEncodedLen};
use cumulus_pallet_parachain_system::RelayChainStateProof;
use frame_support::weights::Weight;
use polkadot_sdk::*;
use scale_info::TypeInfo;

pub use pallet::*;

/// Wire size of a paired (ECDSA, BLS12-381) BEEFY key: `ecdsa(33) || G1(48) || G2(96)`.
const PAIRED_LEN: usize = 177;
/// Offset of the BLS G1 half within a paired key.
const PAIRED_G1_OFFSET: usize = 33;
/// Size of a compressed G1 point.
const G1_LEN: usize = 48;

/// `Beefy::NextAuthorities` on the relay chain.
///
/// The *next* set, not the current one, and the distinction is what makes the scheme work. A client
/// verifying a header signed by set N reads this digest and thereby learns the commitment for set
/// N+1, so it can verify the following update. Committing the current set instead would be
/// circular: you would need set N's commitment to verify the header carrying set N's commitment.
///
/// Also note this is not `well_known_keys::AUTHORITIES`, which is `Babe::Authorities`; the BEEFY
/// keys live under the `Beefy` prefix.
pub const RELAY_BEEFY_NEXT_AUTHORITIES: [u8; 32] = [
	0x08, 0xc4, 0x19, 0x74, 0xa9, 0x7d, 0xbf, 0x15, 0xcf, 0xbe, 0xc2, 0x83, 0x65, 0xbe, 0xa2, 0xda,
	0xaa, 0xcf, 0x00, 0xb9, 0xb4, 0x1f, 0xda, 0x7a, 0x92, 0x68, 0x82, 0x1c, 0x2a, 0x2b, 0x3e, 0x4c,
];

/// `Beefy::ValidatorSetId` on the relay chain, the id of the *current* set. The digest reports
/// `set_id + 1`, since it describes [`RELAY_BEEFY_NEXT_AUTHORITIES`].
pub const RELAY_BEEFY_VALIDATOR_SET_ID: [u8; 32] = [
	0x08, 0xc4, 0x19, 0x74, 0xa9, 0x7d, 0xbf, 0x15, 0xcf, 0xbe, 0xc2, 0x83, 0x65, 0xbe, 0xa2, 0xda,
	0x8f, 0x05, 0xbc, 0xcc, 0x2f, 0x70, 0xec, 0x66, 0xa3, 0x29, 0x99, 0xc5, 0x76, 0x11, 0x56, 0xbe,
];

/// Engine id for the digest item carrying the commitment.
pub const APK_ENGINE_ID: [u8; 4] = *b"APKC";

/// The payload of the digest item this pallet writes.
///
/// This is a wire format: an off-chain verifier reads it out of a header it has already
/// authenticated through the BEEFY MMR's parachain heads root, and feeds `commitment` to
/// `ApkProof.verify` as `publicKeysCommitment`. The set id is what lets it tell which authority
/// set the commitment belongs to.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct ApkCommitmentDigest {
	/// The BEEFY validator set id these keys belong to, which is the relay's current set id plus
	/// one, since the commitment describes the next set.
	pub set_id: u64,
	/// Poseidon2 over the set's G1 keys, padded to the circuit width with the identity point.
	pub commitment: [u8; 32],
}

/// A paired BEEFY authority key exactly as the relay chain stores it.
#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct PairedAuthority(pub [u8; PAIRED_LEN]);

impl PairedAuthority {
	/// The BLS G1 half, which is what the APK circuit consumes. `DoublePublicKey` publishes the
	/// same secret in both groups, so this is the counterpart of the G2 key BEEFY verifies with.
	pub fn g1(&self) -> [u8; G1_LEN] {
		let mut out = [0u8; G1_LEN];
		out.copy_from_slice(&self.0[PAIRED_G1_OFFSET..PAIRED_G1_OFFSET + G1_LEN]);
		out
	}
}

impl ApkCommitmentDigest {
	/// Pull the commitment out of a header's digest logs, if this pallet wrote one into it.
	///
	/// This is the client side of the design. A verifier that has already authenticated a
	/// hyperbridge header, through the parachain heads root in the BEEFY MMR leaf, can read the
	/// commitment straight out of that header with no further proof, and hand it to
	/// `ApkProof.verify` as `publicKeysCommitment`.
	///
	/// Returns the first matching item. The pallet only ever writes one per block, and only on the
	/// block a set completes.
	pub fn find_in(digest: &sp_runtime::generic::Digest) -> Option<Self> {
		digest.logs().iter().find_map(|log| match log {
			sp_runtime::DigestItem::Consensus(id, payload) if *id == APK_ENGINE_ID =>
				Self::decode(&mut &payload[..]).ok(),
			_ => None,
		})
	}
}

impl Progress {
	/// A chain that has absorbed nothing yet.
	pub fn fresh(set_digest: [u8; 32]) -> Self {
		Self { set_digest, absorbed: 0, state: PartialCommitment::new().to_bytes() }
	}
}

/// Where the running commitment has got to.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, Default, MaxEncodedLen)]
pub struct Progress {
	/// Identifies the set being absorbed, so a rotation part way through restarts rather than
	/// mixing keys from two sets into one commitment.
	pub set_digest: [u8; 32],
	/// How many of the [`NUM_VALIDATORS`] slots have been absorbed.
	pub absorbed: u32,
	/// The Merkle-Damgard state, carried between blocks.
	pub state: [u8; 32],
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config + cumulus_pallet_parachain_system::Config
	{
		/// How many validator slots to absorb per block. Trades block weight against how many
		/// blocks a full set takes: at roughly 410us per slot in wasm, 64 is about 26ms.
		#[pallet::constant]
		type SlotsPerBlock: Get<u32>;

		/// Cost of absorbing a chunk. `()` carries a measured default, see [`WeightInfo`].
		///
		/// Disambiguated at use as `<T as Config>::WeightInfo`, since
		/// `cumulus_pallet_parachain_system::Config` also has one.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// The commitment currently being absorbed, if any.
	#[pallet::storage]
	pub type Pending<T: Config> = StorageValue<_, Progress, OptionQuery>;

	/// The last commitment published to a header digest, and the set it describes.
	#[pallet::storage]
	pub type Published<T: Config> = StorageValue<_, ([u8; 32], [u8; 32]), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Started absorbing a new authority set.
		CommitmentStarted { set_digest: [u8; 32] },
		/// Finished, and wrote the commitment to this block's header.
		CommitmentPublished { set_id: u64, set_digest: [u8; 32], commitment: [u8; 32] },
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Runs in `on_finalize` for two reasons: the relay state proof is only fresh after the
		/// validation data inherent, and a digest has to be deposited before the header is sealed.
		///
		/// `on_finalize` cannot return weight, so the cost is registered explicitly. Absorbing a
		/// chunk is real work, tens of milliseconds, and a block that does not account for it can
		/// overrun its budget.
		fn on_finalize(_now: BlockNumberFor<T>) {
			let slots = match Self::advance() {
				Ok(slots) => slots,
				Err(e) => {
					log::debug!(target: "apk-digest", "commitment did not advance: {e:?}");
					0
				},
			};
			if slots > 0 {
				frame_system::Pallet::<T>::register_extra_weight_unchecked(
					<T as Config>::WeightInfo::absorb(slots),
					DispatchClass::Mandatory,
				);
			}
		}
	}

	impl<T: Config> Pallet<T> {
		/// Absorb the next chunk, starting or restarting if the set changed, and publish once the
		/// whole set is in.
		fn advance() -> Result<u32, Error<T>> {
			let keys = Self::relay_beefy_g1_keys()?;
			// Read the set id up front even though it is only needed at the end. It is cheap, and
			// discovering it missing after absorbing a chunk would throw that work away: the error
			// propagates before `Pending` is written, so the same chunk would be re-absorbed and
			// re-fail every block.
			let set_id = Self::relay_beefy_set_id()?;
			let set_digest = sp_io::hashing::blake2_256(&keys.encode());

			let mut progress = match next_progress(
				Pending::<T>::get().as_ref(),
				Published::<T>::get().map(|(d, _)| d),
				set_digest,
			) {
				Step::Done => return Ok(0),
				Step::Restart => {
					Self::deposit_event(Event::CommitmentStarted { set_digest });
					Progress::fresh(set_digest)
				},
				Step::Continue(p) => p,
			};

			let from = progress.absorbed as usize;
			let take = (T::SlotsPerBlock::get() as usize).min(NUM_VALIDATORS - from);
			if take == 0 {
				return Ok(0);
			}

			progress.state = absorb_slots(&keys, from, take, progress.state)
				.map_err(|_| Error::<T>::MalformedAuthorityKey)?;
			progress.absorbed += take as u32;

			if progress.absorbed as usize == NUM_VALIDATORS {
				let commitment = progress.state;
				let payload = ApkCommitmentDigest { set_id, commitment };

				// The header is the delivery mechanism: a client that has already authenticated
				// this header through the BEEFY MMR's parachain heads root can read the commitment
				// straight out of it, with no further proof.
				frame_system::Pallet::<T>::deposit_log(sp_runtime::DigestItem::Consensus(
					APK_ENGINE_ID,
					payload.encode(),
				));

				Pending::<T>::kill();
				Published::<T>::put((set_digest, commitment));
				Self::deposit_event(Event::CommitmentPublished { set_id, set_digest, commitment });
			} else {
				Pending::<T>::put(&progress);
			}
			Ok(take as u32)
		}

		/// The id of the set the commitment describes: the relay's current set id plus one, since
		/// the keys come from `NextAuthorities`.
		fn relay_beefy_set_id() -> Result<u64, Error<T>> {
			let current: u64 = Self::relay_state()?
				.read_entry(&RELAY_BEEFY_VALIDATOR_SET_ID, None)
				.map_err(|_| Error::<T>::KeyNotProven)?;
			Ok(current.saturating_add(1))
		}

		/// The relay state proof for this block, checked against the relay parent's state root.
		///
		/// Only carries the keys the runtime asks for through
		/// `KeyToIncludeInRelayProof::keys_to_prove`; anything else reads back as `KeyNotProven`.
		fn relay_state() -> Result<RelayChainStateProof, Error<T>> {
			let proof = cumulus_pallet_parachain_system::RelayStateProof::<T>::get()
				.ok_or(Error::<T>::NoRelayProof)?;
			let validation_data = cumulus_pallet_parachain_system::ValidationData::<T>::get()
				.ok_or(Error::<T>::NoRelayProof)?;

			RelayChainStateProof::new(
				T::SelfParaId::get(),
				validation_data.relay_parent_storage_root,
				proof,
			)
			.map_err(|_| Error::<T>::BadRelayProof)
		}

		/// The G1 halves of the relay chain's *next* BEEFY authority set.
		fn relay_beefy_g1_keys() -> Result<Vec<[u8; G1_LEN]>, Error<T>> {
			let authorities: Vec<PairedAuthority> = Self::relay_state()?
				.read_entry(&RELAY_BEEFY_NEXT_AUTHORITIES, None)
				.map_err(|_| Error::<T>::KeyNotProven)?;

			Ok(authorities.iter().map(|a| a.g1()).collect())
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No relay state proof in storage yet, which is normal before the first inherent.
		NoRelayProof,
		/// The relay state proof did not verify against the relay parent's state root.
		BadRelayProof,
		/// The BEEFY authorities key was absent from the proof, so `keys_to_prove` is not asking
		/// for it.
		KeyNotProven,
		/// An authority's G1 half did not decode as a curve point.
		MalformedAuthorityKey,
	}
}

/// Cost of absorbing `slots` validator slots into the running commitment.
pub trait WeightInfo {
	fn absorb(slots: u32) -> Weight;
}

/// Measured rather than benchmarked, and should be replaced by a generated `WeightInfo` before
/// this runs anywhere real.
///
/// The commitment was timed in wasm at roughly 410us per slot, linear in the number of slots from
/// 64 up to the full 1024. Weight ref time is picoseconds, so a slot is about 410_000_000 units,
/// and the default `SlotsPerBlock` of 64 comes to ~26ms, a little over one percent of a two second
/// block. The storage side is one read and one write of a fixed-size value.
impl WeightInfo for () {
	fn absorb(slots: u32) -> Weight {
		Weight::from_parts(410_000_000u64.saturating_mul(slots as u64), 0)
			.saturating_add(Weight::from_parts(0, 4096))
	}
}

/// What to do with the commitment this block.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
	/// This set is already published; nothing to do.
	Done,
	/// Begin, or begin again because the set changed under us.
	Restart,
	/// Carry on from where the last block left off.
	Continue(Progress),
}

/// Decide how to proceed, given what is in progress and what has already been published.
///
/// Kept pure so the rotation case can be tested without a mock chain. The case that matters is a
/// set changing part way through: the Merkle-Damgard chain is over one specific key list, so
/// carrying the state across a rotation would silently produce a commitment belonging to neither
/// set. Restarting is the only safe answer.
pub fn next_progress(
	pending: Option<&Progress>,
	published: Option<[u8; 32]>,
	set_digest: [u8; 32],
) -> Step {
	match pending {
		Some(p) if p.set_digest != set_digest => Step::Restart,
		Some(p) => Step::Continue(p.clone()),
		None if published == Some(set_digest) => Step::Done,
		None => Step::Restart,
	}
}

/// A key that did not decode as a G1 curve point.
#[derive(Debug, PartialEq, Eq)]
pub struct MalformedKey;

/// Absorb slots `from .. from + take` of a validator set into a running commitment.
///
/// Kept free of the pallet so the part that can actually be wrong, the chunking and the padding,
/// is testable without a mock chain. Slots past the end of `keys` are the identity point, which is
/// how the circuit pads a set shorter than [`NUM_VALIDATORS`].
///
/// `state` is the Merkle-Damgard state carried between blocks; pass
/// `PartialCommitment::new().to_bytes()` to start.
pub fn absorb_slots(
	keys: &[[u8; G1_LEN]],
	from: usize,
	take: usize,
	state: [u8; 32],
) -> Result<[u8; 32], MalformedKey> {
	let chunk: Vec<G1Affine> = (from..from + take)
		.map(|i| match keys.get(i) {
			Some(k) => G1Affine::deserialize_compressed(&k[..]).map_err(|_| MalformedKey),
			None => Ok(G1Affine::identity()),
		})
		.collect::<Result<_, _>>()?;

	let mut partial = PartialCommitment::from_bytes(&state);
	partial.absorb(&chunk);
	Ok(partial.to_bytes())
}

#[cfg(test)]
mod tests {
	use super::*;
	use apk_commitment::{padded_to_circuit_width, public_keys_commitment_bytes};
	use ark_serialize::CanonicalSerialize;

	/// Real G1 halves from a live BLS relay's `Beefy` authorities.
	const RELAY_KEYS: [&str; 2] = [
		"b7235087b611457915f812c4c9af17fe9c590f0a9c9d2f3b62f5d31673a5d5ae02309ea191fe6ebeb66cb3ad23db3b04",
		"a3948b7bd16acfa3b7a113826a1a8b192c2c462f31ddd9dc90131f5b014d85ff9669ad1a686f33f89e6b0d821761b2cc",
	];

	fn relay_keys() -> Vec<[u8; G1_LEN]> {
		RELAY_KEYS
			.iter()
			.map(|h| {
				let mut out = [0u8; G1_LEN];
				out.copy_from_slice(&hex::decode(h).unwrap());
				out
			})
			.collect()
	}

	fn expected_commitment(keys: &[[u8; G1_LEN]]) -> [u8; 32] {
		let points: Vec<G1Affine> =
			keys.iter().map(|k| G1Affine::deserialize_compressed(&k[..]).unwrap()).collect();
		public_keys_commitment_bytes(&padded_to_circuit_width(&points))
	}

	fn run(keys: &[[u8; G1_LEN]], slots_per_block: usize) -> [u8; 32] {
		let mut state = PartialCommitment::new().to_bytes();
		let mut absorbed = 0usize;
		while absorbed < NUM_VALIDATORS {
			let take = slots_per_block.min(NUM_VALIDATORS - absorbed);
			state = absorb_slots(keys, absorbed, take, state).unwrap();
			absorbed += take;
		}
		state
	}

	/// The whole point of absorbing across blocks: the block size must not change the answer.
	#[test]
	fn any_chunk_size_reaches_the_same_commitment() {
		let keys = relay_keys();
		let expected = expected_commitment(&keys);
		for slots in [1usize, 64, 100, 512, NUM_VALIDATORS] {
			assert_eq!(run(&keys, slots), expected, "slots_per_block {slots} changed the result");
		}
	}

	/// A chunk that straddles the boundary between real keys and padding is the case most likely
	/// to be got wrong, so pin it explicitly.
	#[test]
	fn padding_boundary_is_handled_within_a_chunk() {
		let keys = relay_keys();
		// 2 real keys, so a chunk of 3 from slot 0 crosses into padding immediately.
		assert_eq!(run(&keys, 3), expected_commitment(&keys));
	}

	/// An empty set is all padding, and must still be well defined.
	#[test]
	fn an_empty_set_is_all_padding() {
		let commitment = run(&[], 64);
		let all_identity: Vec<G1Affine> =
			(0..NUM_VALIDATORS).map(|_| G1Affine::identity()).collect();
		assert_eq!(commitment, public_keys_commitment_bytes(&all_identity));
	}

	/// Order matters, since the bitlist selects signers positionally.
	#[test]
	fn key_order_changes_the_commitment() {
		let keys = relay_keys();
		let mut swapped = keys.clone();
		swapped.swap(0, 1);
		assert_ne!(run(&keys, 64), run(&swapped, 64));
	}

	/// A key whose x coordinate is the field modulus, which is not a canonical field element.
	///
	/// Note an all-`0xff` key is *not* a good negative case: the top bits are the compression and
	/// infinity flags, so it decodes happily as the identity point.
	#[test]
	fn a_malformed_key_is_rejected_rather_than_absorbed() {
		let mut key = hex::decode(
			"1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f624\
			 1eabfffeb153ffffb9feffffffffaaab",
		)
		.unwrap();
		key[0] |= 0x80; // compressed form, so the x bytes are actually parsed
		let mut fixed = [0u8; G1_LEN];
		fixed.copy_from_slice(&key);

		let state = PartialCommitment::new().to_bytes();
		assert_eq!(absorb_slots(&[fixed], 0, 1, state), Err(MalformedKey));
	}

	/// The identity point encodes as a valid compressed key, so a set genuinely containing one is
	/// absorbed rather than rejected. Worth pinning so the malformed-key check is not mistaken for
	/// an identity check.
	#[test]
	fn an_identity_key_is_valid_input() {
		let mut encoded = Vec::new();
		G1Affine::identity().serialize_compressed(&mut encoded).unwrap();
		let mut key = [0u8; G1_LEN];
		key.copy_from_slice(&encoded);
		let state = PartialCommitment::new().to_bytes();
		assert!(absorb_slots(&[key], 0, 1, state).is_ok());
	}

	// ── rotation ────────────────────────────────────────────────────────────────────────────

	const SET_A: [u8; 32] = [0xaa; 32];
	const SET_B: [u8; 32] = [0xbb; 32];

	#[test]
	fn a_fresh_chain_starts() {
		assert_eq!(next_progress(None, None, SET_A), Step::Restart);
	}

	#[test]
	fn an_already_published_set_is_left_alone() {
		assert_eq!(next_progress(None, Some(SET_A), SET_A), Step::Done);
	}

	/// A new set arriving after one was published starts rather than stopping.
	#[test]
	fn a_new_set_starts_even_though_another_was_published() {
		assert_eq!(next_progress(None, Some(SET_A), SET_B), Step::Restart);
	}

	#[test]
	fn work_in_progress_on_the_same_set_continues() {
		let p = Progress { set_digest: SET_A, absorbed: 128, state: [1u8; 32] };
		assert_eq!(next_progress(Some(&p), None, SET_A), Step::Continue(p));
	}

	/// The case this whole function exists for: the authority set changed while a commitment was
	/// part way through.
	#[test]
	fn a_rotation_part_way_through_restarts() {
		let p = Progress { set_digest: SET_A, absorbed: 512, state: [1u8; 32] };
		assert_eq!(next_progress(Some(&p), None, SET_B), Step::Restart);
	}

	/// And restarting has to mean *restarting*, not resuming with a relabelled set. If the state
	/// carried over, the commitment would be a Merkle-Damgard chain over the first set's keys
	/// followed by the second's, belonging to neither, and nothing downstream would notice.
	#[test]
	fn a_restart_discards_the_partial_state() {
		let fresh = Progress::fresh(SET_B);
		assert_eq!(fresh.absorbed, 0);
		assert_eq!(fresh.state, PartialCommitment::new().to_bytes());
		assert_eq!(fresh.set_digest, SET_B);
	}

	/// End to end over the absorption itself: absorb part of one set, rotate, and the commitment
	/// that comes out must be the second set's, identical to having never seen the first.
	#[test]
	fn a_commitment_interrupted_by_a_rotation_is_not_a_mixture() {
		let first = relay_keys();
		let mut second = relay_keys();
		second.swap(0, 1); // a different set, same size

		// absorb 300 slots of the first set, then rotate
		let mut state = PartialCommitment::new().to_bytes();
		state = absorb_slots(&first, 0, 300, state).unwrap();
		assert_eq!(
			next_progress(Some(&Progress { set_digest: SET_A, absorbed: 300, state }), None, SET_B),
			Step::Restart
		);

		// restart discards that state, so the result is the second set's commitment alone
		let restarted = run(&second, 64);
		assert_eq!(restarted, expected_commitment(&second));
		assert_ne!(restarted, expected_commitment(&first), "the two sets must not collide");
	}

	/// A client finds the commitment in a header carrying unrelated digest items too, which is the
	/// normal case: aura and the parachain system both write their own.
	#[test]
	fn commitment_is_found_among_other_digest_items() {
		let payload = ApkCommitmentDigest { set_id: 577, commitment: [3u8; 32] };
		let digest = sp_runtime::generic::Digest {
			logs: alloc::vec![
				sp_runtime::DigestItem::PreRuntime(*b"aura", alloc::vec![1, 2, 3]),
				sp_runtime::DigestItem::Consensus(APK_ENGINE_ID, payload.encode()),
				sp_runtime::DigestItem::Seal(*b"aura", alloc::vec![4, 5, 6]),
			],
		};
		assert_eq!(ApkCommitmentDigest::find_in(&digest), Some(payload));
	}

	/// A header from a block that did not complete a set carries nothing, and a client must treat
	/// that as "no update" rather than an error.
	#[test]
	fn a_header_without_our_digest_yields_nothing() {
		let digest = sp_runtime::generic::Digest {
			logs: alloc::vec![sp_runtime::DigestItem::PreRuntime(*b"aura", alloc::vec![1])],
		};
		assert_eq!(ApkCommitmentDigest::find_in(&digest), None);
	}

	/// Another engine's consensus item must not be mistaken for ours, even though the variant is
	/// the same. This is what the engine id is for.
	#[test]
	fn another_engines_consensus_item_is_ignored() {
		let payload = ApkCommitmentDigest { set_id: 1, commitment: [9u8; 32] };
		let digest = sp_runtime::generic::Digest {
			logs: alloc::vec![sp_runtime::DigestItem::Consensus(*b"BEEF", payload.encode())],
		};
		assert_eq!(ApkCommitmentDigest::find_in(&digest), None);
	}

	/// The digest is a wire format, so its encoding is pinned here: a client decodes these bytes
	/// out of a header.
	#[test]
	fn digest_payload_round_trips() {
		let payload = ApkCommitmentDigest { set_id: 42, commitment: [7u8; 32] };
		let encoded = payload.encode();
		assert_eq!(encoded.len(), 8 + 32, "set id then commitment, no padding");
		assert_eq!(ApkCommitmentDigest::decode(&mut &encoded[..]).unwrap(), payload);
	}
}
