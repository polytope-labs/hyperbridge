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
//! The commitment is expensive, around 800ms of wasm for a full 1024 slot set, most of it spent
//! decompressing the keys rather than hashing them. It is committed in one block all the same,
//! since it happens once per authority set, which is once every four hours on polkadot, and a set
//! that has not changed is republished from the stored commitment rather than hashed again.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

use ark_bls12_381::G1Affine;
use ark_serialize::CanonicalDeserialize;
pub use beefy_verifier_primitives::{
	ApkCommitmentDigest, APK_ENGINE_ID, RELAY_BEEFY_NEXT_AUTHORITIES,
};
use beefy_verifier_primitives::{PairedAuthority, BLS_G1_SIGNATURE_LEN};
use codec::{Decode, Encode, MaxEncodedLen};
use cumulus_pallet_parachain_system::RelayChainStateProof;
use frame_support::weights::Weight;
use gnark_plonk_verifier::{padded_to_circuit_width, public_keys_commitment_bytes};
use polkadot_sdk::*;
use scale_info::TypeInfo;

pub use pallet::*;

/// `Beefy::ValidatorSetId` on the relay chain, the id of the *current* set. The digest reports
/// `set_id + 1`, since it describes [`RELAY_BEEFY_NEXT_AUTHORITIES`].
pub const RELAY_BEEFY_VALIDATOR_SET_ID: [u8; 32] = [
	0x08, 0xc4, 0x19, 0x74, 0xa9, 0x7d, 0xbf, 0x15, 0xcf, 0xbe, 0xc2, 0x83, 0x65, 0xbe, 0xa2, 0xda,
	0x8f, 0x05, 0xbc, 0xcc, 0x2f, 0x70, 0xec, 0x66, 0xa3, 0x29, 0x99, 0xc5, 0x76, 0x11, 0x56, 0xbe,
];

pub mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config + cumulus_pallet_parachain_system::Config
	{
		/// Cost of committing to a set. `()` carries a rough default, see [`WeightInfo`].
		///
		/// Disambiguated at use as `<T as Config>::WeightInfo`, since
		/// `cumulus_pallet_parachain_system::Config` also has one.
		type WeightInfo: WeightInfo;
	}

	/// Bumped whenever the commitment or the record around it changes shape, so [`migration`]
	/// knows to throw away what was published under the old one.
	pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// Whether the next block should carry the relay's authority keys in its proof.
	///
	/// Set by the block that sees the set id move, read by [`crate::wants_keys`] when the next
	/// block is built. Keeping the keys out of the proof the rest of the time is the point.
	#[pallet::storage]
	pub type KeysWanted<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// The last commitment published to a header digest, and the set it describes.
	#[pallet::storage]
	pub type Published<T: Config> = StorageValue<_, (u64, u32, [u8; 32], [u8; 32]), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Committed to a new authority set, and wrote the commitment to this block's header.
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
			let hashed = match Self::advance() {
				Ok(hashed) => hashed,
				Err(e) => {
					log::debug!(target: "apk-digest", "commitment did not advance: {e:?}");
					false
				},
			};
			if hashed {
				frame_system::Pallet::<T>::register_extra_weight_unchecked(
					<T as Config>::WeightInfo::commit(),
					DispatchClass::Mandatory,
				);
			}
		}
	}

	impl<T: Config> Pallet<T> {
		/// Publish this block's digest, committing to a new set when the relay has rotated.
		///
		/// The set id is in every proof and costs nothing to read. The keys are only asked for
		/// once the id has moved, so the block that notices a rotation records that it needs them
		/// and the next one gets them. That keeps 1024 keys out of the relay proof on the ordinary
		/// block, which is every block but one or two a session.
		fn advance() -> Result<bool, Error<T>> {
			let set_id = Self::relay_beefy_set_id()?;

			// Nothing has moved, so the commitment already computed still describes this set.
			if let Some((published, len, _, commitment)) = Published::<T>::get() {
				if published == set_id {
					Self::deposit_digest(set_id, len, commitment);
					return Ok(false);
				}
			}

			// The id moved. The keys are only in the proof if a previous block asked for them,
			// so the first block after a rotation asks and the next one does the work.
			let Ok(keys) = Self::relay_beefy_g1_keys() else {
				KeysWanted::<T>::put(true);
				// The old commitment is still the truth about the old set, and a client files
				// commitments by set id, so republishing it under the id it was computed for
				// keeps this block's header useful rather than empty.
				if let Some((published, len, _, commitment)) = Published::<T>::get() {
					Self::deposit_digest(published, len, commitment);
				}
				return Ok(false);
			};
			KeysWanted::<T>::kill();

			let set_digest = sp_io::hashing::blake2_256(&keys.encode());
			let len = keys.len() as u32;
			match next_step(Published::<T>::get().as_ref(), set_digest) {
				// The membership has not changed, only the id, which is common. Rehashing would
				// cost the better part of a second for a commitment already held.
				Step::Republish(commitment) => {
					Self::deposit_digest(set_id, len, commitment);
					Published::<T>::put((set_id, len, set_digest, commitment));
					Ok(false)
				},
				Step::Commit => {
					let commitment =
						commit(&keys).map_err(|_| Error::<T>::MalformedAuthorityKey)?;
					Self::deposit_digest(set_id, len, commitment);
					Published::<T>::put((set_id, len, set_digest, commitment));
					Self::deposit_event(Event::CommitmentPublished {
						set_id,
						set_digest,
						commitment,
					});
					Ok(true)
				},
			}
		}

		/// Put the commitment in this block's header.
		///
		/// The header is the delivery mechanism: a client that has already authenticated it
		/// through the BEEFY MMR's parachain heads root reads the commitment straight out of it,
		/// with no further proof. It goes in every header the set covers rather than only the one
		/// where the hashing finished, since a verifier only ever sees the single header a proof
		/// finalizes and cannot choose which.
		fn deposit_digest(set_id: u64, len: u32, commitment: [u8; 32]) {
			let payload = ApkCommitmentDigest { set_id, len, commitment };
			frame_system::Pallet::<T>::deposit_log(sp_runtime::DigestItem::Consensus(
				APK_ENGINE_ID,
				payload.encode(),
			));
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
		fn relay_beefy_g1_keys() -> Result<Vec<[u8; BLS_G1_SIGNATURE_LEN]>, Error<T>> {
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

/// Cost of committing to a validator set.
pub trait WeightInfo {
	fn commit() -> Weight;
}

/// A rough default for tests and for a chain that has not generated its own.
///
/// A full set is around 800ms in wasm, of which roughly four fifths is decompressing the keys
/// rather than hashing them: a key arrives as 48 compressed bytes and recovering `y` needs a
/// square root in the base field. It is only paid when the membership actually changes.
impl WeightInfo for () {
	fn commit() -> Weight {
		Weight::from_parts(821_000_000_000, 0).saturating_add(Weight::from_parts(0, 4096))
	}
}

/// Whether the next block's relay proof should carry the authority keys.
///
/// A runtime answers `KeyToIncludeInRelayProof` with this: asking for 1024 keys on every block
/// costs proof size for data that only changes once a session. The block that sees the set id
/// move records that it wants them, and this reports it while the next block is built. Also true
/// before anything has been published, since the first commitment has to come from somewhere.
pub fn wants_keys<T: Config>() -> bool {
	KeysWanted::<T>::get() || Published::<T>::get().is_none()
}

/// Throw away a commitment computed under an older scheme.
///
/// A commitment is only recomputed when the membership changes, so a runtime upgrade that changes
/// how keys are hashed would otherwise keep republishing the old value indefinitely, on any chain
/// whose validators happen to stay the same. The stored digest cannot notice: it describes the
/// keys, not the arithmetic applied to them. Clearing the record forces one recomputation and the
/// pallet carries on from there.
pub mod migration {
	use super::*;
	use frame_support::traits::{Get, GetStorageVersion, OnRuntimeUpgrade};

	pub struct ClearStaleCommitment<T>(core::marker::PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for ClearStaleCommitment<T> {
		fn on_runtime_upgrade() -> Weight {
			if <Pallet<T> as GetStorageVersion>::on_chain_storage_version() >=
				pallet::STORAGE_VERSION
			{
				return T::DbWeight::get().reads(1);
			}
			Published::<T>::kill();
			pallet::STORAGE_VERSION.put::<Pallet<T>>();
			T::DbWeight::get().reads_writes(1, 2)
		}
	}
}

/// What to do with the commitment this block.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
	/// The keys are unchanged, so republish the commitment already computed for them.
	Republish([u8; 32]),
	/// Nothing published, or a different set of keys: hash them.
	Commit,
}

/// Decide how to proceed, given what was published last.
///
/// Kept pure so it can be tested without a mock chain. The case worth being careful about is a
/// session rotating without the membership changing, which is common: the commitment is the same,
/// so rehashing it would spend the better part of a second for nothing, but it still has to be
/// republished under the new set id or a client tracking commitments per set never learns it.
pub fn next_step(published: Option<&(u64, u32, [u8; 32], [u8; 32])>, set_digest: [u8; 32]) -> Step {
	match published {
		Some((_, _, digest, commitment)) if *digest == set_digest => Step::Republish(*commitment),
		_ => Step::Commit,
	}
}

/// A key that did not decode as a G1 curve point.
#[derive(Debug, PartialEq, Eq)]
pub struct MalformedKey;

/// The commitment over a validator set, padded to the circuit's width with the identity point.
///
/// Kept free of the pallet so the part that can actually be wrong, the decompression and the
/// padding, is testable without a mock chain.
pub fn commit(keys: &[[u8; BLS_G1_SIGNATURE_LEN]]) -> Result<[u8; 32], MalformedKey> {
	let points: Vec<G1Affine> = keys
		.iter()
		.map(|k| G1Affine::deserialize_compressed(&k[..]).map_err(|_| MalformedKey))
		.collect::<Result<_, _>>()?;

	Ok(public_keys_commitment_bytes(&padded_to_circuit_width(&points)))
}

#[cfg(test)]
mod tests {
	use super::*;
	use ark_serialize::CanonicalSerialize;
	use gnark_plonk_verifier::{padded_to_circuit_width, public_keys_commitment_bytes};

	/// Real G1 halves from a live BLS relay's `Beefy` authorities.
	const RELAY_KEYS: [&str; 2] = [
		"b7235087b611457915f812c4c9af17fe9c590f0a9c9d2f3b62f5d31673a5d5ae02309ea191fe6ebeb66cb3ad23db3b04",
		"a3948b7bd16acfa3b7a113826a1a8b192c2c462f31ddd9dc90131f5b014d85ff9669ad1a686f33f89e6b0d821761b2cc",
	];

	fn relay_keys() -> Vec<[u8; BLS_G1_SIGNATURE_LEN]> {
		RELAY_KEYS
			.iter()
			.map(|h| {
				let mut out = [0u8; BLS_G1_SIGNATURE_LEN];
				out.copy_from_slice(&hex::decode(h).unwrap());
				out
			})
			.collect()
	}

	fn expected_commitment(keys: &[[u8; BLS_G1_SIGNATURE_LEN]]) -> [u8; 32] {
		let points: Vec<G1Affine> =
			keys.iter().map(|k| G1Affine::deserialize_compressed(&k[..]).unwrap()).collect();
		public_keys_commitment_bytes(&padded_to_circuit_width(&points))
	}

	/// A set shorter than the circuit's width is padded, which is most of what `commit` does
	/// beyond hashing.
	#[test]
	fn a_short_set_is_padded_to_the_circuit_width() {
		let keys = relay_keys();
		assert_eq!(commit(&keys).unwrap(), expected_commitment(&keys));
	}

	/// An empty set is all padding, and must still be well defined.
	#[test]
	fn an_empty_set_is_all_padding() {
		let all_identity: Vec<G1Affine> = (0..gnark_plonk_verifier::NUM_VALIDATORS)
			.map(|_| G1Affine::identity())
			.collect();
		assert_eq!(commit(&[]).unwrap(), public_keys_commitment_bytes(&all_identity));
	}

	/// Order matters, since the bitlist selects signers positionally.
	#[test]
	fn key_order_changes_the_commitment() {
		let keys = relay_keys();
		let mut swapped = keys.clone();
		swapped.swap(0, 1);
		assert_ne!(commit(&keys).unwrap(), commit(&swapped).unwrap());
	}

	/// A key whose x coordinate is the field modulus, which is not a canonical field element.
	///
	/// Note an all-`0xff` key is *not* a good negative case: the top bits are the compression and
	/// infinity flags, so it decodes happily as the identity point.
	#[test]
	fn a_malformed_key_is_rejected_rather_than_hashed() {
		let mut key = hex::decode(
			"1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f624\
			 1eabfffeb153ffffb9feffffffffaaab",
		)
		.unwrap();
		key[0] |= 0x80; // compressed form, so the x bytes are actually parsed
		let mut fixed = [0u8; BLS_G1_SIGNATURE_LEN];
		fixed.copy_from_slice(&key);

		assert_eq!(commit(&[fixed]), Err(MalformedKey));
	}

	/// The identity point encodes as a valid compressed key, so a set genuinely containing one is
	/// absorbed rather than rejected. Worth pinning so the malformed-key check is not mistaken for
	/// an identity check.
	#[test]
	fn an_identity_key_is_valid_input() {
		let mut encoded = Vec::new();
		G1Affine::identity().serialize_compressed(&mut encoded).unwrap();
		let mut key = [0u8; BLS_G1_SIGNATURE_LEN];
		key.copy_from_slice(&encoded);
		assert!(commit(&[key]).is_ok());
	}

	const SET_A: [u8; 32] = [0xaa; 32];
	const SET_B: [u8; 32] = [0xbb; 32];

	#[test]
	fn a_fresh_chain_commits() {
		assert_eq!(next_step(None, SET_A), Step::Commit);
	}

	#[test]
	fn keys_already_committed_to_are_republished_rather_than_rehashed() {
		let published = (1u64, 2u32, SET_A, [7u8; 32]);
		assert_eq!(next_step(Some(&published), SET_A), Step::Republish([7u8; 32]));
	}

	/// A session can rotate without the membership changing, which is common on small networks and
	/// possible anywhere. Rehashing would cost the better part of a second for a commitment we
	/// already hold, so the stored one is republished under the new set id instead.
	#[test]
	fn the_same_keys_under_a_new_set_id_are_not_rehashed() {
		let published = (1u64, 2u32, SET_A, [7u8; 32]);
		assert_eq!(next_step(Some(&published), SET_A), Step::Republish([7u8; 32]));
	}

	/// Different keys mean the stored commitment says nothing about them.
	#[test]
	fn a_new_set_is_committed_to_even_though_another_was_published() {
		let published = (1u64, 2u32, SET_A, [7u8; 32]);
		assert_eq!(next_step(Some(&published), SET_B), Step::Commit);
	}

	/// Two different sets must not reach the same commitment, which is the property the set digest
	/// is standing in for when deciding whether to rehash.
	#[test]
	fn a_different_set_commits_differently() {
		let first = relay_keys();
		let mut second = relay_keys();
		second.swap(0, 1);
		assert_ne!(commit(&first).unwrap(), commit(&second).unwrap());
	}

	/// normal case: aura and the parachain system both write their own.
	#[test]
	fn commitment_is_found_among_other_digest_items() {
		let payload = ApkCommitmentDigest { set_id: 577, len: 2, commitment: [3u8; 32] };
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
		let payload = ApkCommitmentDigest { set_id: 1, len: 2, commitment: [9u8; 32] };
		let digest = sp_runtime::generic::Digest {
			logs: alloc::vec![sp_runtime::DigestItem::Consensus(*b"BEEF", payload.encode())],
		};
		assert_eq!(ApkCommitmentDigest::find_in(&digest), None);
	}

	/// The digest is a wire format, so its encoding is pinned here: a client decodes these bytes
	/// out of a header.
	#[test]
	fn digest_payload_round_trips() {
		let payload = ApkCommitmentDigest { set_id: 42, len: 3, commitment: [7u8; 32] };
		let encoded = payload.encode();
		assert_eq!(encoded.len(), 8 + 4 + 32, "set id, size, then commitment, no padding");
		assert_eq!(ApkCommitmentDigest::decode(&mut &encoded[..]).unwrap(), payload);
	}
}
