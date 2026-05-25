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

//! # Pallet BEEFY Consensus Proofs
//!
//! Verifies BEEFY consensus proofs (primarily SP1 ZK) submitted by off-chain provers and
//! feeds the finalized parachain state commitments into `pallet-ismp`. Rewards submitters
//! from the treasury when a proof does useful work — either carries the expected next
//! authority-set rotation, or advances the latest proven parachain height past a block
//! in which new ISMP requests were dispatched.
//!
//! Proofs are submitted via **signed** extrinsics: the signer of the extrinsic is the
//! reward payee. The pallet sets `Pays::No` on accepted proofs so a successful prover
//! gets their fee refunded along with the reward; failed proofs pay the transaction
//! fee normally, which keeps spam off the chain.
//!
//! ## Uncle proofs
//!
//! Multiple SP1 provers running independently can each get rewarded for the same
//! finality target via decreasing-curve uncle rewards. The first prover to land a
//! proof advances state and gets position 0; subsequent independent provers
//! (different proof bytes thanks to SP1 Groth16 witness randomization) for the
//! same target are accepted as uncles, up to `MaxUncleProvers`, and rewarded at
//! decreasing positions.
//!
//! Uncle verification reuses the consensus state snapshot taken before the first
//! proof mutated it, so uncle proofs are checked cryptographically against the
//! same trusted state the first prover used. `keccak256(proof)` is recorded per
//! parachain height to reject re-submission of bytes that were already accepted.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
pub mod types;
pub mod weights;

use polkadot_sdk::*;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::{vec, vec::Vec};
	use alloy_sol_types::SolType;
	use codec::{Decode, Encode};
	use frame_support::{
		dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo},
		pallet_prelude::*,
		traits::{
			fungible::{self, Inspect, Mutate},
			tokens::Preservation,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use ismp::{
		consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
		handlers,
		host::IsmpHost,
		messaging::{ConsensusMessage as IsmpConsensusMessage, Message, StateCommitmentHeight},
	};
	use ismp_abi::ecdsa_beefy::BeefyConsensusState as SolBeefyConsensusState;
	use primitive_types::H256;
	use sp_runtime::traits::AccountIdConversion;

	type BalanceOf<T> =
		<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

	/// `Get<u32>` adapter that yields `MaxUncleProvers + 1`, the total number of provers
	/// (one first + `MaxUncleProvers` uncles) that may be rewarded per parachain height.
	/// Used as the bound for `AcceptedProofHashes` and `RewardCurve`.
	pub struct MaxStoredProvers<T>(core::marker::PhantomData<T>);
	impl<T: Config> Get<u32> for MaxStoredProvers<T> {
		fn get() -> u32 {
			T::MaxUncleProvers::get().saturating_add(1)
		}
	}

	/// Current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// Origin permitted to run privileged calls (`initialize_state`, `set_proof_reward`,
		/// `set_sp1_vkey_hash`, `set_reward_curve`).
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Currency used for treasury reward payouts.
		type Currency: Mutate<Self::AccountId>;

		/// Treasury account derivation (rewards are transferred from here).
		#[pallet::constant]
		type TreasuryPalletId: Get<PalletId>;

		/// Maximum size in bytes of a single proof payload.
		#[pallet::constant]
		type MaxProofSize: Get<u32>;

		/// Per-bucket cap on the `MessagingProofs` and `RotationProofs` on-chain ring
		/// buffers (and, transitively, on the number of offchain proof blobs retained
		/// per kind).
		#[pallet::constant]
		type MaxStoredProofs: Get<u32>;

		/// The `ConsensusStateId` used for BEEFY in `pallet-ismp`.
		#[pallet::constant]
		type ConsensusStateId: Get<ConsensusStateId>;

		/// Unbonding period passed to `pallet-ismp` on first `initialize_state`, in seconds.
		#[pallet::constant]
		type UnbondingPeriod: Get<u64>;

		/// Maximum number of uncle provers rewarded per parachain height, in addition to
		/// the first prover. Total provers per height are therefore `MaxUncleProvers + 1`,
		/// occupying positions `0..=MaxUncleProvers` (position 0 is always the first prover).
		/// Naive proofs only ever occupy position 0; uncle rewards apply to SP1.
		#[pallet::constant]
		type MaxUncleProvers: Get<u32>;

		/// The pallet-assets instance used for managing the reputation token.
		/// Mints reputation tokens 1:1 with native token rewards to proof submitters.
		type ReputationAsset: fungible::Mutate<Self::AccountId, Balance = BalanceOf<Self>>;

		/// Weight info.
		type WeightInfo: crate::weights::WeightInfo;
	}

	/// `ChildTrieRoot` snapshot at the last messaging reward — dirty-bit for "new dispatches
	/// exist since we last paid".
	#[pallet::storage]
	pub type LastRewardedDispatchRoot<T: Config> = StorageValue<_, H256, OptionQuery>;

	/// Base reward amount paid to position-0 (first) provers. Uncle rewards are derived from
	/// this value by applying [`RewardCurve`].
	#[pallet::storage]
	pub type ProofReward<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// SP1 verification key hash, consumed by
	/// `beefy_verifier::sp1::verify_sp1_consensus`.
	#[pallet::storage]
	pub type Sp1VkeyHash<T: Config> = StorageValue<_, H256, ValueQuery>;

	/// Heights of recent messaging proofs (no authority-set rotation). Values are
	/// strictly increasing because every accepted proof advances the proven height,
	/// so `vec[0]` is always the oldest — FIFO eviction via `remove(0)` when full.
	/// The proof bytes live in offchain storage under
	/// [`offchain_key(latest_height)`](types::offchain_key).
	#[pallet::storage]
	pub type MessagingProofs<T: Config> =
		StorageValue<_, BoundedVec<u64, T::MaxStoredProofs>, ValueQuery>;

	/// Map of `set_id → latest_height` for rotation proofs. Lets relayers catch a lagging
	/// EVM destination up across multiple epochs: given the last-known authority set id,
	/// walk entries forward, fetching each rotation proof from offchain storage under
	/// [`offchain_key(latest_height)`](types::offchain_key). BEEFY set ids are monotone,
	/// so `iter().next()` gives FIFO eviction on overflow.
	#[pallet::storage]
	pub type RotationProofs<T: Config> =
		StorageValue<_, BoundedBTreeMap<u64, u64, T::MaxStoredProofs>, ValueQuery>;

	/// Pre-proof BEEFY consensus state snapshot keyed by the parachain height the
	/// first accepted proof advanced to. Uncle proofs verify against this snapshot
	/// because the live consensus state has already been advanced by the first proof.
	/// Pruned alongside `MessagingProofs`/`RotationProofs` eviction.
	#[pallet::storage]
	pub type ProofContext<T: Config> = StorageMap<_, Blake2_128Concat, u64, Vec<u8>, OptionQuery>;

	/// Number of provers rewarded so far per parachain height. Used as the position
	/// index when the next uncle lands.
	#[pallet::storage]
	pub type ProverCount<T: Config> = StorageMap<_, Blake2_128Concat, u64, u32, ValueQuery>;

	/// `keccak256(proof_bytes)` for every proof accepted at a given parachain height.
	/// SP1 Groth16 randomizes the witness so independent provers produce different
	/// bytes; re-submission of the exact same bytes hits this set and is rejected.
	/// Bounded by `MaxUncleProvers + 1` (one first + `MaxUncleProvers` uncles).
	#[pallet::storage]
	pub type AcceptedProofHashes<T: Config> =
		StorageMap<_, Blake2_128Concat, u64, BoundedVec<H256, MaxStoredProvers<T>>, ValueQuery>;

	/// Reward fractions `(numerator, denominator)` indexed by prover position. The base
	/// reward [`ProofReward`] is multiplied by the fraction at the prover's position.
	/// An empty curve falls back to `(1, 1)` for position 0 and zero for uncles, so the
	/// pallet keeps the existing single-prover behaviour until an admin sets a curve.
	/// Bounded by `MaxUncleProvers + 1`, matching the position range `0..=MaxUncleProvers`.
	#[pallet::storage]
	pub type RewardCurve<T: Config> =
		StorageValue<_, BoundedVec<(u32, u32), MaxStoredProvers<T>>, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Consensus state has not been initialized yet.
		NotInitialized,
		/// Proof is stale: `latest_height ≤ latest_state_machine_height`, or the proof rotated to
		/// an unexpected authority set.
		StaleProof,
		/// First proof byte is not a recognized proof type.
		UnknownProofType,
		/// ABI decoding or conversion failed.
		AbiDecodeFailed,
		/// The submitted proof is not in canonical ABI form (e.g. trailing padding).
		MalformedProof,
		/// The BEEFY verifier rejected the proof.
		VerificationFailed,
		/// Rotation proof did not rotate to `NextAuthoritySetId`.
		UnexpectedAuthoritySet,
		/// Failed to transfer the reward from the treasury.
		RewardTransferFailed,
		/// `pallet-ismp` rejected the consensus message.
		IsmpUpdateFailed,
		/// The child trie root was not provided in the proof.
		MissingChildTrieRoot,
		/// The proof does not advance state: no authority set rotation and no new
		/// messages since the last rewarded proof.
		NoNewWork,
		/// `MaxUncleProvers` already accepted at this height.
		UncleSlotsFull,
		/// This exact proof has already been accepted at this height.
		ProofAlreadySubmitted,
		/// No first proof has been seen at this height, so no uncle slot exists.
		NoUncleContext,
		/// `set_reward_curve` received a fraction with a zero denominator.
		InvalidRewardCurve,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A first proof was accepted and state advanced to `height`.
		ProofAccepted {
			submitter: T::AccountId,
			height: u64,
			new_set_id: Option<u64>,
			rewarded: BalanceOf<T>,
		},
		/// An uncle proof was accepted at `height` (which the first proof already
		/// advanced state to). No state advance; reward only.
		UncleProofAccepted {
			submitter: T::AccountId,
			height: u64,
			rewarded: BalanceOf<T>,
			/// `1..=MaxUncleProvers`. Position 0 always belongs to the first proof.
			position: u32,
		},
		/// Consensus state was (re)initialized by admin.
		StateInitialized { current_set_id: u64, next_set_id: u64, latest_beefy_height: u32 },
		/// Reward amount updated.
		ProofRewardUpdated { new_reward: BalanceOf<T> },
		/// SP1 verification key hash updated.
		Sp1VkeyHashUpdated,
		/// Reward curve updated.
		RewardCurveUpdated,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Initialize or reset the BEEFY consensus state from its solidity-ABI encoding.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::initialize_state())]
		pub fn initialize_state(origin: OriginFor<T>, abi_state: Vec<u8>) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;

			let state: beefy_verifier_primitives::ConsensusState =
				SolBeefyConsensusState::abi_decode(&abi_state)
					.map_err(|e| {
						log::warn!(
							target: "ismp",
							"[beefy-consensus-proofs]: abi_decode(BeefyConsensusState) failed: {e}",
						);
						Error::<T>::AbiDecodeFailed
					})?
					.into();
			let current_set_id = state.current_authorities.id;
			let next_set_id = state.next_authorities.id;
			let latest_beefy_height = state.latest_beefy_height;
			let host = pallet_ismp::Pallet::<T>::default();

			// Seed an initial commitment for the host state machine at the current block height.
			let height = sp_runtime::SaturatedConversion::saturated_into::<u64>(
				frame_system::Pallet::<T>::block_number(),
			);
			pallet_ismp::Pallet::<T>::create_consensus_client(
				frame_system::RawOrigin::Root.into(),
				ismp::messaging::CreateConsensusState {
					consensus_state: state.encode(),
					consensus_client_id: ismp_beefy::BEEFY_CONSENSUS_ID,
					consensus_state_id: ismp_beefy::BEEFY_CONSENSUS_ID,
					unbonding_period: T::UnbondingPeriod::get(),
					challenge_periods: Default::default(),
					state_machine_commitments: vec![(
						StateMachineId {
							consensus_state_id: ismp_beefy::BEEFY_CONSENSUS_ID,
							state_id: host.host_state_machine(),
						},
						StateCommitmentHeight {
							height,
							commitment: StateCommitment {
								timestamp: host.timestamp().as_secs(),
								overlay_root: None,
								state_root: H256::zero(),
							},
						},
					)],
				},
			)
			.map_err(|e| {
				log::warn!(
					target: "ismp",
					"[beefy-consensus-proofs]: pallet_ismp::create_consensus_client failed: {e:?}",
				);
				Error::<T>::IsmpUpdateFailed
			})?;

			LastRewardedDispatchRoot::<T>::kill();

			Self::deposit_event(Event::StateInitialized {
				current_set_id,
				next_set_id,
				latest_beefy_height,
			});
			Ok(())
		}

		/// Submit a BEEFY consensus proof. Signed: the signer is the reward payee.
		///
		/// `proof` is a `BoundedVec` so SCALE decoding rejects oversized payloads inside
		/// the txpool, before the runtime ever pays for the call. Successful proofs
		/// (first or uncle) refund their transaction fee via `Pays::No`; failed proofs
		/// pay the fee, which is the spam deterrent.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::submit_proof())]
		pub fn submit_proof(
			origin: OriginFor<T>,
			proof: BoundedVec<u8, T::MaxProofSize>,
		) -> DispatchResultWithPostInfo {
			let submitter = ensure_signed(origin)?;
			Self::do_submit_proof(submitter, proof.into_inner())
		}

		/// Update the base reward amount paid to position-0 provers.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_proof_reward())]
		pub fn set_proof_reward(origin: OriginFor<T>, reward: BalanceOf<T>) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;
			ProofReward::<T>::put(reward);
			Self::deposit_event(Event::ProofRewardUpdated { new_reward: reward });
			Ok(())
		}

		/// Update the SP1 verification key hash.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::set_sp1_vkey_hash())]
		pub fn set_sp1_vkey_hash(origin: OriginFor<T>, vkey_hash: H256) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;
			Sp1VkeyHash::<T>::put(vkey_hash);
			Self::deposit_event(Event::Sp1VkeyHashUpdated);
			Ok(())
		}

		/// Set the decreasing reward curve. Position `i` gets `ProofReward * curve[i].0 /
		/// curve[i].1`. Empty curve means default behaviour (position 0 = full reward,
		/// no uncle rewards). Bounded by `MaxUncleProvers + 1` to match the storage,
		/// covering position 0 (first proof) plus all uncle slots.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::set_reward_curve())]
		pub fn set_reward_curve(
			origin: OriginFor<T>,
			curve: BoundedVec<(u32, u32), MaxStoredProvers<T>>,
		) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;
			// `numerator > denominator` would multiply the base reward above 100% for that
			// position, turning a fat-fingered curve into a treasury drain. Reject it
			// outright — uncle positions are meant to *decrease* from the position-0 base.
			if curve.iter().any(|(num, denom)| *denom == 0 || num > denom) {
				Err(Error::<T>::InvalidRewardCurve)?
			}
			RewardCurve::<T>::put(curve);
			Self::deposit_event(Event::RewardCurveUpdated);
			Ok(())
		}
	}

	/// Outcome of a successful [`Pallet::verify_and_apply`] call.
	pub struct VerifyOutcome {
		/// Highest parachain height finalized by this proof (0 if none).
		pub latest_height: u64,
		/// `current_authorities.id` of the consensus state *after* the update.
		pub current_set_id: u64,
		/// True iff the proof rotated the current authority set.
		pub rotated: bool,
		/// True iff the child trie root changed since the last rewarded proof.
		pub has_new_messages: bool,
		/// Root of the child trie verified by this proof.
		pub child_trie_root: H256,
	}

	impl<T: Config> Pallet<T> {
		/// Returns the latest proven parachain height from `pallet-ismp` for the
		/// coprocessor state machine.
		fn latest_height() -> u64 {
			let host = pallet_ismp::Pallet::<T>::default();
			let id = ismp::consensus::StateMachineId {
				state_id: T::Coprocessor::get()
					.expect("coprocessor must be set in hyperbridge runtime; qed"),
				consensus_state_id: T::ConsensusStateId::get(),
			};
			host.latest_commitment_height(id).unwrap_or_default()
		}

		/// Top-level dispatch. On success, settle as the first proof; on any failure for
		/// an SP1 proof, retry through the uncle path which runs its own cryptographic
		/// check against the saved snapshot.
		fn do_submit_proof(submitter: T::AccountId, proof: Vec<u8>) -> DispatchResultWithPostInfo {
			// Size is enforced by the `BoundedVec<u8, T::MaxProofSize>` parameter on
			// `submit_proof` — oversized payloads fail SCALE decoding inside the txpool
			// and never reach this dispatch.
			// The set of accepted proof types is gated by `ismp-beefy`'s
			// `BeefyClientConfig::allowed_proof_types` during `verify_and_apply`; here we only
			// need the type byte to drive canonical re-encoding. Unknown bytes still fall
			// through to the `_ => UnknownProofType` arm of the match below.
			let proof_type = *proof.first().ok_or(Error::<T>::UnknownProofType)?;

			// Decode the ABI payload then re-encode it canonically and hash *that* instead
			// of the raw input. `alloy_sol_types::abi_decode_params` silently ignores
			// trailing bytes after the encoded sequence ends, so without this a submitter
			// could pad a valid proof with junk to mint a fresh `keccak256(proof)` and
			// bypass the `AcceptedProofHashes` dedup. Hashing the canonical re-encoding
			// collapses every ABI-equivalent input to the same hash by construction.
			let abi_payload = &proof[1..];
			let canonical_payload = match proof_type {
				types::PROOF_TYPE_SP1 => {
					let p =
						<ismp_abi::sp1_beefy::SP1Beefy::SP1BeefyProof as SolType>::abi_decode_params(
							abi_payload,
						)
						.map_err(|_| Error::<T>::AbiDecodeFailed)?;
					<ismp_abi::sp1_beefy::SP1Beefy::SP1BeefyProof as SolType>::abi_encode_params(&p)
				},
				types::PROOF_TYPE_NAIVE => {
					let p =
						<ismp_abi::ecdsa_beefy::BeefyConsensusProof as SolType>::abi_decode_params(
							abi_payload,
						)
						.map_err(|_| Error::<T>::AbiDecodeFailed)?;
					<ismp_abi::ecdsa_beefy::BeefyConsensusProof as SolType>::abi_encode_params(&p)
				},
				_ => Err(Error::<T>::UnknownProofType)?,
			};
			let mut canonical_proof = Vec::with_capacity(1 + canonical_payload.len());
			canonical_proof.push(proof_type);
			canonical_proof.extend_from_slice(&canonical_payload);
			let proof_hash: H256 = sp_io::hashing::keccak_256(&canonical_proof).into();

			// Reject any submission that isn't already in canonical form. If the raw input
			// hashes differently from its canonical re-encoding it carries non-canonical
			// bytes (trailing padding, alternate encodings), so it is malformed.
			let submitted_hash: H256 = sp_io::hashing::keccak_256(&proof).into();
			if submitted_hash != proof_hash {
				Err(Error::<T>::MalformedProof)?
			}

			// Read the pre-proof consensus state before `verify_and_apply` mutates it.
			// Used to seed `ProofContext` for the first-proof path.
			let host = pallet_ismp::Pallet::<T>::default();
			let prev_state_bytes = host
				.consensus_state(ismp_beefy::BEEFY_CONSENSUS_ID)
				.map_err(|_| Error::<T>::NotInitialized)?;

			match Self::verify_and_apply(&canonical_proof) {
				Ok(outcome) => Self::settle_first_proof(
					submitter,
					canonical_proof,
					proof_hash,
					proof_type,
					prev_state_bytes,
					outcome,
				),
				// `verify_and_apply` returns `StaleProof` for SP1 proofs whose
				// `block_number <= prev.latest_beefy_height` via the upfront height check,
				// which is exactly the legitimate-uncle case. Other failures (corrupt
				// bytes, bad signatures, wrong vkey) propagate so the submitter pays the
				// fee instead of paying for a wasted second SP1 verification.
				Err(Error::<T>::StaleProof) if proof_type == types::PROOF_TYPE_SP1 =>
					Self::settle_uncle_proof(submitter, canonical_proof, proof_hash),
				Err(e) => Err(e.into()),
			}
		}

		/// First-proof path: state has been advanced inside `verify_and_apply`. Save the
		/// pre-proof snapshot, record the proof hash, pay the reward at position 0, and
		/// run ring-buffer eviction across `MessagingProofs`/`RotationProofs`. When an
		/// entry falls off either ring, prune the matching uncle rows.
		fn settle_first_proof(
			submitter: T::AccountId,
			proof: Vec<u8>,
			proof_hash: H256,
			proof_type: u8,
			prev_state_bytes: Vec<u8>,
			outcome: VerifyOutcome,
		) -> DispatchResultWithPostInfo {
			if outcome.has_new_messages {
				LastRewardedDispatchRoot::<T>::put(outcome.child_trie_root);
			}

			// Record uncle metadata for SP1 proofs only. Naive proofs are ineligible.
			if proof_type == types::PROOF_TYPE_SP1 {
				Self::record_uncle_metadata(outcome.latest_height, prev_state_bytes, proof_hash)?;
			}

			let reward_paid = Self::pay_position_reward(&submitter, 0)?;

			sp_io::offchain_index::set(&types::offchain_key(outcome.latest_height), &proof);
			let evicted_height = if outcome.rotated {
				RotationProofs::<T>::mutate(|map| {
					let evicted = (map.len() as u32 == T::MaxStoredProofs::get())
						.then(|| map.iter().next().map(|(k, _)| *k))
						.flatten()
						.and_then(|set_id| map.remove(&set_id));
					let _ = map.try_insert(outcome.current_set_id, outcome.latest_height);
					evicted
				})
			} else {
				MessagingProofs::<T>::mutate(|vec| {
					let evicted =
						(vec.len() as u32 == T::MaxStoredProofs::get()).then(|| vec.remove(0));
					let _ = vec.try_push(outcome.latest_height);
					evicted
				})
			};

			if let Some(height) = evicted_height {
				sp_io::offchain_index::clear(&types::offchain_key(height));
				ProofContext::<T>::remove(height);
				ProverCount::<T>::remove(height);
				AcceptedProofHashes::<T>::remove(height);
			}

			Self::deposit_event(Event::ProofAccepted {
				submitter,
				height: outcome.latest_height,
				new_set_id: outcome.rotated.then_some(outcome.current_set_id),
				rewarded: reward_paid,
			});

			Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
		}

		/// Uncle path: the live consensus state is already past this proof's target
		/// because a prior prover landed first. Look up the pre-proof snapshot saved at
		/// that time and verify the SP1 proof directly against it.
		fn settle_uncle_proof(
			submitter: T::AccountId,
			proof: Vec<u8>,
			proof_hash: H256,
		) -> DispatchResultWithPostInfo {
			// The first proof for the most-recent finality target advanced state to
			// `latest_height` and snapshotted the pre-state under that key. Uncles that
			// arrive while that race is still open look up the same key. If a different
			// first proof has since bumped state past this height, the snapshot under
			// `latest_height` is for the *newer* first proof; the uncle's lower
			// `proof.block_number` will fail SP1 verification against it (StaleHeight),
			// so a stale uncle pays the tx fee.
			let parachain_height = Self::latest_height();

			let snapshot_bytes =
				ProofContext::<T>::get(parachain_height).ok_or(Error::<T>::NoUncleContext)?;

			// `ProverCount` is incremented after each successful uncle (the first proof
			// is position 0). With `MaxUncleProvers = N`, valid uncle positions are
			// `1..=N`, so reject once the position the next uncle would occupy exceeds N.
			let position = ProverCount::<T>::get(parachain_height);
			if position > T::MaxUncleProvers::get() {
				Err(Error::<T>::UncleSlotsFull)?
			}

			let hashes = AcceptedProofHashes::<T>::get(parachain_height);
			if hashes.contains(&proof_hash) {
				Err(Error::<T>::ProofAlreadySubmitted)?
			}

			// Verify the proof cryptographically against the saved trusted state. We
			// don't apply state mutations because the live state is already past this point.
			let snapshot: beefy_verifier_primitives::ConsensusState =
				Decode::decode(&mut &snapshot_bytes[..]).map_err(|_| Error::<T>::NotInitialized)?;

			let abi_payload = &proof[1..];
			let abi_proof =
				<ismp_abi::sp1_beefy::SP1Beefy::SP1BeefyProof as SolType>::abi_decode_params(
					abi_payload,
				)
				.map_err(|_| Error::<T>::AbiDecodeFailed)?;
			let scale_proof: beefy_verifier_primitives::Sp1BeefyProof = abi_proof.into();

			let vkey_hash = Sp1VkeyHash::<T>::get();
			let vkey = alloc::format!("0x{:x}", vkey_hash);

			beefy_verifier::sp1::verify_sp1_consensus::<types::SubstrateCrypto>(
				snapshot,
				scale_proof,
				&vkey,
			)
			.map_err(|e| {
				log::debug!(
					target: "ismp",
					"[beefy-consensus-proofs] uncle SP1 verification failed: {e:?}",
				);
				Error::<T>::VerificationFailed
			})?;

			let reward_paid = Self::pay_position_reward(&submitter, position)?;

			AcceptedProofHashes::<T>::try_mutate(parachain_height, |vec| vec.try_push(proof_hash))
				.map_err(|_| Error::<T>::UncleSlotsFull)?;
			ProverCount::<T>::insert(parachain_height, position.saturating_add(1));

			Self::deposit_event(Event::UncleProofAccepted {
				submitter,
				height: parachain_height,
				rewarded: reward_paid,
				position,
			});

			Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
		}

		/// Save the pre-proof snapshot keyed by `parachain_height` and register the
		/// first proof's hash. Uncles for this height land in the same rows; eviction
		/// from `MessagingProofs`/`RotationProofs` removes them in lockstep.
		fn record_uncle_metadata(
			parachain_height: u64,
			prev_state_bytes: Vec<u8>,
			proof_hash: H256,
		) -> Result<(), Error<T>> {
			ProofContext::<T>::insert(parachain_height, prev_state_bytes);

			AcceptedProofHashes::<T>::try_mutate(parachain_height, |vec| vec.try_push(proof_hash))
				.map_err(|_| Error::<T>::UncleSlotsFull)?;
			ProverCount::<T>::mutate(parachain_height, |c| *c = c.saturating_add(1));

			Ok(())
		}

		/// Apply the curve at `position` to [`ProofReward`], transfer from the treasury,
		/// and mint reputation 1:1.
		fn pay_position_reward(
			submitter: &T::AccountId,
			position: u32,
		) -> Result<BalanceOf<T>, Error<T>> {
			let zero = BalanceOf::<T>::default();
			let base = ProofReward::<T>::get();
			if base == zero {
				return Ok(zero);
			}

			let reward = Self::position_reward(base, position);
			if reward == zero {
				return Ok(zero);
			}

			let treasury: T::AccountId =
				<T as Config>::TreasuryPalletId::get().into_account_truncating();
			<T as Config>::Currency::transfer(&treasury, submitter, reward, Preservation::Preserve)
				.map_err(|e| {
					log::warn!(
						target: "ismp",
						"[beefy-consensus-proofs] treasury reward transfer failed: {e:?}",
					);
					Error::<T>::RewardTransferFailed
				})?;

			if let Err(e) = T::ReputationAsset::mint_into(submitter, reward) {
				log::warn!(
					target: "ismp",
					"[beefy-consensus-proofs] reputation mint failed: {e:?}",
				);
			}

			Ok(reward)
		}

		/// Apply the configured curve to the base reward. Empty curve means full reward
		/// at position 0 and zero at later positions, preserving pre-uncle behaviour.
		///
		/// Uses a u128 round-trip for `* num / denom`. Hyperbridge runtimes use a u128
		/// balance, so the trip is lossless; on any runtime where balances exceed u128
		/// we'd saturate, which is acceptable since rewards are small relative to total
		/// supply.
		fn position_reward(base: BalanceOf<T>, position: u32) -> BalanceOf<T> {
			let curve = RewardCurve::<T>::get();
			if curve.is_empty() {
				return if position == 0 { base } else { BalanceOf::<T>::default() };
			}
			let Some((num, denom)) = curve.get(position as usize).copied() else {
				return BalanceOf::<T>::default();
			};
			if denom == 0 {
				return BalanceOf::<T>::default();
			}
			use sp_runtime::SaturatedConversion;
			let base_u128: u128 = base.saturated_into();
			let scaled = base_u128.saturating_mul(num as u128).saturating_div(denom as u128);
			scaled.saturated_into::<BalanceOf<T>>()
		}

		/// First-proof verification path:
		///
		/// 1. ABI-decode the proof into the SCALE shape `ismp-beefy` consumes.
		/// 2. Dispatch `Message::Consensus` through `ismp::handlers::handle_incoming_message`,
		///    which routes to `BeefyConsensusClient::verify_consensus`. That runs the full BEEFY /
		///    SP1 check and persists consensus state + parachain commitments. The verifier's own
		///    upfront stale check (`beefy_verifier::error::Error::StaleHeight`) propagates back
		///    wrapped in `ismp::Error::AnyHow` here; we surface it as `StaleProof` so the
		///    dispatcher can route an SP1 uncle to `settle_uncle_proof`.
		/// 3. Extract the proven parachain height from the returned `StateMachineUpdated` events
		///    and the new authority-set id from the stored consensus state so the caller can
		///    classify the proof as rotation / messaging.
		pub fn verify_and_apply(proof: &[u8]) -> Result<VerifyOutcome, Error<T>> {
			let proof_type = *proof.first().ok_or(Error::<T>::UnknownProofType)?;
			let abi_payload = &proof[1..];

			let host = pallet_ismp::Pallet::<T>::default();
			let prev_state_bytes = host
				.consensus_state(ismp_beefy::BEEFY_CONSENSUS_ID)
				.map_err(|_| Error::<T>::NotInitialized)?;
			let prev_state: beefy_verifier_primitives::ConsensusState =
				Decode::decode(&mut &prev_state_bytes[..])
					.map_err(|_| Error::<T>::NotInitialized)?;
			let prev_height = Self::latest_height();

			let consensus_proof = match proof_type {
				types::PROOF_TYPE_SP1 => {
					let abi_proof =
						<ismp_abi::sp1_beefy::SP1Beefy::SP1BeefyProof as SolType>::abi_decode_params(
							abi_payload,
						)
						.map_err(|_| Error::<T>::AbiDecodeFailed)?;
					let scale_proof: beefy_verifier_primitives::Sp1BeefyProof = abi_proof.into();
					[&[types::PROOF_TYPE_SP1], scale_proof.encode().as_slice()].concat()
				},
				types::PROOF_TYPE_NAIVE => {
					let abi_proof =
						<ismp_abi::ecdsa_beefy::BeefyConsensusProof as SolType>::abi_decode_params(
							abi_payload,
						)
						.map_err(|_| Error::<T>::AbiDecodeFailed)?;
					let scale_proof: beefy_verifier_primitives::ConsensusMessage = abi_proof.into();
					[&[types::PROOF_TYPE_NAIVE], scale_proof.encode().as_slice()].concat()
				},
				_ => Err(Error::<T>::UnknownProofType)?,
			};

			let result = handlers::handle_incoming_message(
				&host,
				Message::Consensus(IsmpConsensusMessage {
					consensus_proof,
					consensus_state_id: ismp_beefy::BEEFY_CONSENSUS_ID,
					signer: vec![],
				}),
			)
			.map_err(|e| {
				log::warn!(
					target: "ismp",
					"[beefy-consensus-proofs] handle_incoming_message failed: {e:?}",
				);
				// `BeefyConsensusClient::verify_consensus` wraps verifier failures as
				// `ismp::Error::AnyHow(anyhow::Error)`, preserving the typed
				// `beefy_verifier::Error` inside. Walk the chain — `anyhow::Error` (from
				// `update_client`'s return type) → `ismp::Error::AnyHow` →
				// `beefy_verifier::Error` — and route `StaleHeight` to the uncle path.
				let stale = e
					.downcast_ref::<ismp::error::Error>()
					.and_then(|err| match err {
						ismp::error::Error::AnyHow(any) => Some(&any.0),
						_ => None,
					})
					.and_then(|inner| inner.downcast_ref::<beefy_verifier::error::Error>())
					.map(|verr| matches!(verr, beefy_verifier::error::Error::StaleHeight { .. }))
					.unwrap_or(false);
				if stale {
					Error::<T>::StaleProof
				} else {
					Error::<T>::VerificationFailed
				}
			})?;

			// Highest parachain height finalized by this proof
			let ismp::handlers::MessageResult::ConsensusMessage(events) = result else {
				Err(Error::<T>::StaleProof)?
			};

			let coprocessor = T::Coprocessor::get().unwrap();
			let latest_height = Self::latest_height();

			if latest_height <= prev_height {
				Err(Error::<T>::StaleProof)?
			}

			let state_commitment = host
				.state_machine_commitment(StateMachineHeight {
					height: latest_height,
					id: StateMachineId {
						consensus_state_id: T::ConsensusStateId::get(),
						state_id: coprocessor,
					},
				})
				.unwrap_or_default();

			// Emit pallet-ismp events for all state machine updates
			for ev in events {
				pallet_ismp::Pallet::<T>::deposit_event(ev.into());
			}

			// Read post-update consensus state to derive the new set id.
			let new_state_bytes = host
				.consensus_state(ismp_beefy::BEEFY_CONSENSUS_ID)
				.map_err(|_| Error::<T>::VerificationFailed)?;
			let new_state: beefy_verifier_primitives::ConsensusState =
				Decode::decode(&mut &new_state_bytes[..])
					.map_err(|_| Error::<T>::VerificationFailed)?;

			// BEEFY invariant: `next` is always `current + 1`.
			if new_state.next_authorities.id != new_state.current_authorities.id.saturating_add(1) {
				Err(Error::<T>::UnexpectedAuthoritySet)?;
			}

			let rotated = new_state.current_authorities.id > prev_state.current_authorities.id;
			let child_trie_root =
				state_commitment.overlay_root.ok_or_else(|| Error::<T>::MissingChildTrieRoot)?;

			// Reject proofs that would be no-ops: no rotation and no new messages.
			let last_rewarded = LastRewardedDispatchRoot::<T>::get().unwrap_or_default();
			let has_new_messages = child_trie_root != last_rewarded && latest_height > prev_height;
			if !rotated && !has_new_messages {
				Err(Error::<T>::NoNewWork)?
			}

			Ok(VerifyOutcome {
				latest_height,
				current_set_id: new_state.current_authorities.id,
				rotated,
				has_new_messages,
				child_trie_root,
			})
		}
	}
}
