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
//! feeds the finalized parachain state commitments into `pallet-ismp`. Rewards submitters a
//! fixed amount from the treasury when a proof does useful work — either carries the
//! expected next authority-set rotation, or advances the latest proven parachain height
//! past a block in which new ISMP requests were dispatched.
//!
//! Proofs are submitted via **authenticated unsigned** extrinsics: the payload carries an
//! SR25519 signature over `(domain, submitter, keccak256(proof))`. The submitter account
//! is both the reward payee and the claimed signer. Full proof verification runs in
//! `ValidateUnsigned` so the tx pool only ever retains valid proofs. Replay is prevented
//! by the monotonic advance of `LastProvenHeight` and the BEEFY authority set id
//! (tracked in `pallet-ismp`'s consensus state): resubmitting
//! the same bytes after a proof is applied trips `StaleProof` or `UnexpectedAuthoritySet`.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod benchmarking;
pub mod types;
pub mod weights;

use polkadot_sdk::*;

pub use pallet::*;
pub use types::{Signature, SubmitProofPayload};
pub use weights::WeightInfo;

/// Offchain-storage key for the rotation proof that advanced the authority set to
/// `set_id`. Relayers reconstruct this key off of a [`RotationProofs`](crate::pallet::RotationProofs)
/// entry's key and read the raw ABI-encoded proof bytes from node-local offchain storage.
pub fn rotation_offchain_key(set_id: u64) -> alloc::vec::Vec<u8> {
	let mut key = alloc::vec::Vec::with_capacity(
		types::OFFCHAIN_PREFIX.len() + types::OFFCHAIN_ROT.len() + 8,
	);
	key.extend_from_slice(types::OFFCHAIN_PREFIX);
	key.extend_from_slice(types::OFFCHAIN_ROT);
	key.extend_from_slice(&set_id.to_be_bytes());
	key
}

/// Offchain-storage key for the messaging proof that advanced the proven parachain
/// height to `proven_height`. Relayers reconstruct this key off of a
/// [`MessagingProofs`](crate::pallet::MessagingProofs) entry's key.
pub fn messaging_offchain_key(proven_height: u64) -> alloc::vec::Vec<u8> {
	let mut key = alloc::vec::Vec::with_capacity(
		types::OFFCHAIN_PREFIX.len() + types::OFFCHAIN_MSG.len() + 8,
	);
	key.extend_from_slice(types::OFFCHAIN_PREFIX);
	key.extend_from_slice(types::OFFCHAIN_MSG);
	key.extend_from_slice(&proven_height.to_be_bytes());
	key
}

/// BEEFY host-function backed crypto used by `beefy-verifier`.
pub struct SubstrateCrypto;

impl ismp::messaging::Keccak256 for SubstrateCrypto {
	fn keccak256(bytes: &[u8]) -> primitive_types::H256 {
		sp_io::hashing::keccak_256(bytes).into()
	}
}

impl beefy_verifier::EcdsaRecover for SubstrateCrypto {
	fn secp256k1_recover(prehash: &[u8; 32], signature: &[u8; 65]) -> anyhow::Result<[u8; 64]> {
		sp_io::crypto::secp256k1_ecdsa_recover(signature, prehash)
			.map_err(|_| anyhow::anyhow!("Failed to recover secp256k1 public key"))
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::{vec, vec::Vec};
	use alloy_sol_types::SolType;
	use codec::{Decode, Encode};
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{Inspect, Mutate},
			tokens::Preservation,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use ismp::{
		consensus::{ConsensusClientId, ConsensusStateId},
		events::StateMachineUpdated,
		handlers,
		host::IsmpHost,
		messaging::{ConsensusMessage as IsmpConsensusMessage, Message},
	};
	use ismp_solidity_abi::beefy::BeefyConsensusState as SolBeefyConsensusState;
	use sp_core::sr25519;
	use sp_runtime::{
		traits::AccountIdConversion,
		transaction_validity::{
			InvalidTransaction, TransactionLongevity, TransactionPriority, TransactionSource,
			TransactionValidity, TransactionValidityError, ValidTransaction,
		},
	};

	use crate::types::{
		Signature, SubmitProofPayload, MSG_TAG, PROOF_TYPE_SP1, ROT_TAG, SIGNATURE_DOMAIN,
	};

	/// BEEFY consensus client id. Matches the solidity constant.
	pub const BEEFY_CONSENSUS_ID: ConsensusClientId = *b"BEEF";

	/// Longevity for both messaging and rotation proofs, in blocks.
	const PROOF_LONGEVITY: TransactionLongevity = 5;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// Origin permitted to run privileged calls (`initialize_state`, `set_proof_reward`,
		/// `set_sp1_vkey_hash`).
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Currency used for treasury reward payouts.
		type Currency: Mutate<Self::AccountId>;

		/// Treasury account derivation (rewards are transferred from here).
		#[pallet::constant]
		type TreasuryPalletId: Get<PalletId>;

		/// Maximum SCALE-encoded size of a single `SubmitProofPayload`.
		#[pallet::constant]
		type MaxProofSize: Get<u32>;

		/// Shared cap on the `RotationProofs` and `MessagingProofs` on-chain ring buffers
		/// (and, transitively, on the number of offchain proof blobs retained per stream).
		#[pallet::constant]
		type MaxStoredProofs: Get<u32>;

		/// The `ConsensusStateId` used for BEEFY in `pallet-ismp`.
		#[pallet::constant]
		type ConsensusStateId: Get<ConsensusStateId>;

		/// Unbonding period passed to `pallet-ismp` on first `initialize_state`, in seconds.
		#[pallet::constant]
		type UnbondingPeriod: Get<u64>;

		/// Weight info.
		type WeightInfo: crate::weights::WeightInfo;
	}

	/// Highest parachain height proven so far.
	#[pallet::storage]
	pub type LastProvenHeight<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// `ChildTrieRoot` snapshot at the last messaging reward — dirty-bit for "new dispatches
	/// exist since we last paid".
	#[pallet::storage]
	pub type LastRewardedDispatchRoot<T: Config> =
		StorageValue<_, <T as frame_system::Config>::Hash, OptionQuery>;

	/// Fixed reward amount per eligible proof.
	#[pallet::storage]
	pub type ProofReward<T: Config> =
		StorageValue<_, <<T as Config>::Currency as Inspect<T::AccountId>>::Balance, ValueQuery>;

	/// SP1 verification key hash (ASCII hex), consumed by
	/// `beefy_verifier::sp1::verify_sp1_consensus`.
	#[pallet::storage]
	pub type Sp1VkeyHash<T: Config> = StorageValue<_, Vec<u8>, ValueQuery>;

	/// Bounded map of `set_id → block number` for the most recent accepted rotation
	/// proofs. The raw ABI-encoded proof bytes live in offchain storage under
	/// [`rotation_offchain_key(set_id)`](crate::rotation_offchain_key); keys here and
	/// in offchain storage move in lock-step (oldest evicted from both when the map
	/// reaches `T::MaxStoredProofs`). BEEFY set ids are monotone, so `pop_first` gives
	/// FIFO eviction for free.
	#[pallet::storage]
	pub type RotationProofs<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<u64, BlockNumberFor<T>, T::MaxStoredProofs>,
		ValueQuery,
	>;

	/// Bounded map of `proven_height → block number` for the most recent accepted
	/// messaging proofs. See [`messaging_offchain_key`](crate::messaging_offchain_key)
	/// for the matching offchain-storage lookup.
	#[pallet::storage]
	pub type MessagingProofs<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<u64, BlockNumberFor<T>, T::MaxStoredProofs>,
		ValueQuery,
	>;

	#[pallet::error]
	pub enum Error<T> {
		/// Consensus state has not been initialized yet.
		NotInitialized,
		/// Payload exceeds `MaxProofSize`.
		ProofTooLarge,
		/// `submitter` could not be interpreted as an SR25519 public key.
		InvalidAccountId,
		/// Signature did not verify against the signed message.
		BadSignature,
		/// Proof is stale (height ≤ `LastProvenHeight` for messaging, or not the expected
		/// rotation).
		StaleProof,
		/// First proof byte is not a recognized proof type.
		UnknownProofType,
		/// ABI decoding or conversion failed.
		AbiDecodeFailed,
		/// The BEEFY verifier rejected the proof.
		VerificationFailed,
		/// Rotation proof did not rotate to `NextAuthoritySetId`.
		UnexpectedAuthoritySet,
		/// Failed to transfer the reward from the treasury.
		RewardTransferFailed,
		/// `pallet-ismp` rejected the consensus message.
		IsmpUpdateFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A proof was accepted and state advanced.
		ProofAccepted {
			submitter: T::AccountId,
			height: u64,
			new_set_id: Option<u64>,
			rewarded: <<T as Config>::Currency as Inspect<T::AccountId>>::Balance,
		},
		/// Consensus state was (re)initialized by admin.
		StateInitialized { current_set_id: u64, next_set_id: u64, latest_beefy_height: u32 },
		/// Reward amount updated.
		ProofRewardUpdated {
			new_reward: <<T as Config>::Currency as Inspect<T::AccountId>>::Balance,
		},
		/// SP1 verification key hash updated.
		Sp1VkeyHashUpdated,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]>,
	{
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

			pallet_ismp::Pallet::<T>::create_consensus_client(
				frame_system::RawOrigin::Root.into(),
				ismp::messaging::CreateConsensusState {
					consensus_state: state.encode(),
					consensus_client_id: BEEFY_CONSENSUS_ID,
					consensus_state_id: T::ConsensusStateId::get(),
					unbonding_period: T::UnbondingPeriod::get(),
					challenge_periods: Default::default(),
					state_machine_commitments: Default::default(),
				},
			)
			.map_err(|e| {
				log::warn!(
					target: "ismp",
					"[beefy-consensus-proofs]: pallet_ismp::create_consensus_client failed: {e:?}",
				);
				Error::<T>::IsmpUpdateFailed
			})?;

			LastProvenHeight::<T>::kill();
			LastRewardedDispatchRoot::<T>::kill();

			Self::deposit_event(Event::StateInitialized {
				current_set_id,
				next_set_id,
				latest_beefy_height,
			});
			Ok(())
		}

		/// Submit a BEEFY consensus proof. Unsigned; authenticated via the payload's
		/// SR25519 signature.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::submit_proof())]
		pub fn submit_proof(
			origin: OriginFor<T>,
			payload: SubmitProofPayload<T::AccountId>,
			signature: Signature,
		) -> DispatchResult {
			ensure_none(origin)?;

			// Single verification path — same helper as `validate_unsigned`. Here the
			// writes (new consensus state, parachain commitments) persist.
			let outcome = Self::verify_and_apply(&payload, &signature)?;

			// Determine reward eligibility.
			let child_trie_root = pallet_ismp::ChildTrieRoot::<T>::get();
			let last_rewarded = LastRewardedDispatchRoot::<T>::get();
			let prev_proven = LastProvenHeight::<T>::get();

			let messaging_reward =
				Some(child_trie_root) != last_rewarded && outcome.proven_height > prev_proven;
			let should_reward = outcome.rotated || messaging_reward;
			
			if !should_reward {
				return Ok(());
			}

			if messaging_reward {
				LastRewardedDispatchRoot::<T>::put(child_trie_root);
			}
			if outcome.proven_height > prev_proven {
				LastProvenHeight::<T>::put(outcome.proven_height);
			}

			let zero = <<T as Config>::Currency as Inspect<T::AccountId>>::Balance::default();
			let reward = ProofReward::<T>::get();
			let reward_paid = if should_reward && reward > zero {
				let treasury: T::AccountId =
					<T as Config>::TreasuryPalletId::get().into_account_truncating();
				<T as Config>::Currency::transfer(
					&treasury,
					&payload.submitter,
					reward,
					Preservation::Preserve,
				)
				.map_err(|e| {
					log::warn!(
						target: "ismp",
						"[beefy-consensus-proofs] treasury reward transfer failed: {e:?}",
					);
					Error::<T>::RewardTransferFailed
				})?;
				reward
			} else {
				zero
			};

			// Fan out to offchain storage + on-chain metadata. Rotation and messaging
			// are disjoint streams: a proof that rotates the authority set is only
			// recorded on the rotation stream even if it also advances proven height.
			// Matches the `validate_unsigned` classification (rotation preempts
			// messaging in the pool), avoids storing the same proof bytes twice.
			//
			// BEEFY set ids and parachain heights are both strictly monotone, so the
			// smallest key in each BoundedBTreeMap is always the oldest entry —
			// `iter().next()` + `remove` gives FIFO eviction without an explicit
			// insertion-order index.
			let at = frame_system::Pallet::<T>::block_number();

			if outcome.rotated {
				let key = crate::rotation_offchain_key(outcome.current_set_id);
				sp_io::offchain_index::set(&key, &payload.proof);

				RotationProofs::<T>::mutate(|map| {
					if map.len() as u32 == T::MaxStoredProofs::get() {
						if let Some(evicted_set_id) = map.iter().next().map(|(k, _)| *k) {
							let _ = map.remove(&evicted_set_id);
							sp_io::offchain_index::clear(
								&crate::rotation_offchain_key(evicted_set_id),
							);
						}
					}
					let _ = map.try_insert(outcome.current_set_id, at);
				});
			} else if outcome.proven_height > prev_proven {
				let key = crate::messaging_offchain_key(outcome.proven_height);
				sp_io::offchain_index::set(&key, &payload.proof);

				MessagingProofs::<T>::mutate(|map| {
					if map.len() as u32 == T::MaxStoredProofs::get() {
						if let Some(evicted_height) = map.iter().next().map(|(k, _)| *k) {
							let _ = map.remove(&evicted_height);
							sp_io::offchain_index::clear(
								&crate::messaging_offchain_key(evicted_height),
							);
						}
					}
					let _ = map.try_insert(outcome.proven_height, at);
				});
			}

			Self::deposit_event(Event::ProofAccepted {
				submitter: payload.submitter.clone(),
				height: outcome.proven_height,
				new_set_id: outcome.rotated.then_some(outcome.current_set_id),
				rewarded: reward_paid,
			});

			Ok(())
		}

		/// Update the fixed reward amount.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_proof_reward())]
		pub fn set_proof_reward(
			origin: OriginFor<T>,
			reward: <<T as Config>::Currency as Inspect<T::AccountId>>::Balance,
		) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;
			ProofReward::<T>::put(reward);
			Self::deposit_event(Event::ProofRewardUpdated { new_reward: reward });
			Ok(())
		}

		/// Update the SP1 verification key hash.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::set_sp1_vkey_hash())]
		pub fn set_sp1_vkey_hash(origin: OriginFor<T>, vkey_hash: Vec<u8>) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;
			Sp1VkeyHash::<T>::put(vkey_hash);
			Self::deposit_event(Event::Sp1VkeyHashUpdated);
			Ok(())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T>
	where
		T::AccountId: Into<[u8; 32]>,
	{
		type Call = Call<T>;

		// empty pre-dispatch so we don't modify storage
		fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
			Ok(())
		}

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			let Call::submit_proof { payload, signature } = call else {
				return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
			};

			// Single verification path: signature + handle_incoming_message (which itself
			// runs the full BEEFY / SP1 check and persists state). In `validate_unsigned`
			// the persistence happens in a discarded overlay; `submit_proof` re-runs this
			// and the writes stick.
			let outcome = Self::verify_and_apply(payload, signature).map_err(|e| {
				log::debug!(target: "ismp", "validate_unsigned rejected: {e:?}");
				// Discriminate reject reasons with distinct Custom codes so tooling can
				// tell why a proof was dropped without relying on log scraping.
				let code: u8 = match e {
					Error::<T>::ProofTooLarge => 1,
					Error::<T>::InvalidAccountId => 2,
					Error::<T>::BadSignature => 3,
					Error::<T>::NotInitialized => 4,
					Error::<T>::UnknownProofType => 5,
					Error::<T>::AbiDecodeFailed => 6,
					Error::<T>::VerificationFailed => 7,
					Error::<T>::UnexpectedAuthoritySet => 8,
					Error::<T>::StaleProof => 9,
					_ => 0,
				};
				TransactionValidityError::Invalid(InvalidTransaction::Custom(code))
			})?;

			let builder = ValidTransaction::with_tag_prefix("BeefyConsensusProofs")
				.longevity(PROOF_LONGEVITY)
				.propagate(true);

			let tx = if outcome.rotated {
				// One slot per pending rotation target.
				builder
					.priority(TransactionPriority::MAX)
					.and_provides((ROT_TAG, outcome.current_set_id).encode())
			} else {
				// Single fixed slot — highest `proven_height` wins.
				builder.priority(outcome.proven_height).and_provides(MSG_TAG.encode())
			};

			tx.build()
		}
	}

	/// Outcome of a successful [`Pallet::verify_and_apply`] call.
	pub struct VerifyOutcome {
		/// Highest parachain height finalized by this proof (0 if none).
		pub proven_height: u64,
		/// `current_authorities.id` of the consensus state *after* the update.
		pub current_set_id: u64,
		/// True iff the proof rotated the current authority set.
		pub rotated: bool,
	}

	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]>,
	{
		/// Single verification path shared by `validate_unsigned` and `submit_proof`:
		///
		/// 1. SR25519 signature check over the payload.
		/// 2. ABI-decode the proof into the SCALE shape `ismp-beefy` consumes.
		/// 3. Dispatch `Message::Consensus` through `ismp::handlers::handle_incoming_message` —
		///    `pallet-ismp` routes to `BeefyConsensusClient::verify_consensus` which runs the full
		///    BEEFY / SP1 check and persists consensus state + parachain commitments.
		/// 4. Extract the proven parachain height from the returned `StateMachineUpdated` events
		///    and the new authority-set id from the stored consensus state so the caller can
		///    classify the proof as rotation / messaging.
		///
		/// Staleness rejection (messaging proofs must push height forward; rotation must
		/// target the expected next set id) is enforced here so both `validate_unsigned`
		/// (which runs this in a discarded overlay) and `submit_proof` (which persists)
		/// share the same accept/reject decision.
		pub fn verify_and_apply(
			payload: &SubmitProofPayload<T::AccountId>,
			signature: &Signature,
		) -> Result<VerifyOutcome, Error<T>> {
			// Size check.
			if (payload.proof.len() as u32) > T::MaxProofSize::get() {
				Err(Error::<T>::ProofTooLarge)?
			}

			// Signature.
			let public = sr25519::Public::from(payload.submitter.clone().into());
			let proof_digest = sp_io::hashing::keccak_256(&payload.proof);
			let msg_preimage = (SIGNATURE_DOMAIN, &payload.submitter, proof_digest).encode();
			let signed_msg = sp_io::hashing::keccak_256(&msg_preimage);
			if !sp_io::crypto::sr25519_verify(signature, &signed_msg, &public) {
				Err(Error::<T>::BadSignature)?
			}

			// expects (proof type byte || SCALE-encoded proof).
			let proof_type = *payload.proof.first().ok_or(Error::<T>::UnknownProofType)?;
			if proof_type != PROOF_TYPE_SP1 {
				Err(Error::<T>::UnknownProofType)?
			}

			// Hand off to pallet-ismp.
			let host = pallet_ismp::Pallet::<T>::default();
			let prev_state_bytes = host
				.consensus_state(BEEFY_CONSENSUS_ID)
				.map_err(|_| Error::<T>::NotInitialized)?;
			let prev_state: beefy_verifier_primitives::ConsensusState =
				Decode::decode(&mut &prev_state_bytes[..])
					.map_err(|_| Error::<T>::NotInitialized)?;
			let result = handlers::handle_incoming_message(
				&host,
				Message::Consensus(IsmpConsensusMessage {
					consensus_proof: payload.proof.clone(),
					consensus_state_id: T::ConsensusStateId::get(),
					signer: public.to_vec(),
				}),
			)
			.map_err(|e| {
				log::warn!(
					target: "ismp",
					"[beefy-consensus-proofs] handle_incoming_message failed: {e}",
				);
				Error::<T>::VerificationFailed
			})?;

			// Highest parachain height finalized by this proof
			let ismp::handlers::MessageResult::ConsensusMessage(events) = result else {
				Err(Error::<T>::StaleProof)?
			};
			let coprocessor = T::Coprocessor::get().unwrap();
			let proven_height = events
				.into_iter()
				.filter_map(|ev| match ev {
					ismp::events::Event::StateMachineUpdated(StateMachineUpdated {
						latest_height,
						state_machine_id,
					}) if state_machine_id.state_id == coprocessor => Some(latest_height),
					_ => None,
				})
				.max()
				.unwrap_or(0);

			// Read post-update consensus state to derive the new set id.
			let new_state_bytes = host
				.consensus_state(BEEFY_CONSENSUS_ID)
				.map_err(|_| Error::<T>::VerificationFailed)?;
			let new_state: beefy_verifier_primitives::ConsensusState =
				Decode::decode(&mut &new_state_bytes[..])
					.map_err(|_| Error::<T>::VerificationFailed)?;
			let rotated = new_state.current_authorities.id > prev_state.current_authorities.id;

			// BEEFY invariant: `next` is always `current + 1`. This also subsumes the
			// N → N + 1 rotation check: when rotated, `new_current == prev_next ==
			// prev_current + 1`, and `new_next == new_current + 1` is the same rule.
			if new_state.next_authorities.id != new_state.current_authorities.id.saturating_add(1) {
				Err(Error::<T>::UnexpectedAuthoritySet)?;
			}
			// Messaging-only proofs must push height forward, otherwise it's a replay.
			if !rotated && proven_height <= LastProvenHeight::<T>::get() {
				Err(Error::<T>::StaleProof)?
			}

			Ok(VerifyOutcome {
				proven_height,
				current_set_id: new_state.current_authorities.id,
				rotated,
			})
		}
	}
}
