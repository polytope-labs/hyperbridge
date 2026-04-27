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
//! by the monotonic advance of `pallet-ismp::LatestStateMachineHeight` and the BEEFY authority set
//! id (tracked in `pallet-ismp`'s consensus state): resubmitting
//! the same bytes after a proof is applied trips `StaleProof` or `UnexpectedAuthoritySet`.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod types;
pub mod weights;

use polkadot_sdk::*;

pub use pallet::*;
pub use types::{Signature, SubmitProofPayload};
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::{vec, vec::Vec};
	use alloy_sol_types::SolType;
	use codec::{Decode, Encode};
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{self, Inspect, Mutate},
			tokens::Preservation,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use ismp::{
		consensus::{ConsensusStateId, StateMachineHeight, StateMachineId},
		handlers,
		host::IsmpHost,
		messaging::{ConsensusMessage as IsmpConsensusMessage, Message},
	};
	use ismp_solidity_abi::beefy::BeefyConsensusState as SolBeefyConsensusState;
	use primitive_types::H256;
	use sp_core::sr25519;
	use sp_runtime::{
		traits::AccountIdConversion,
		transaction_validity::{
			InvalidTransaction, TransactionLongevity, TransactionSource, TransactionValidity,
			TransactionValidityError, ValidTransaction,
		},
	};

	use crate::types::{Signature, SubmitProofPayload};

	/// Longevity for proofs in the tx pool, in blocks.
	const PROOF_LONGEVITY: TransactionLongevity = 15;

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

		/// Allowed proof types. Controls which consensus proof formats the pallet
		/// will accept. On mainnet set to `&[PROOF_TYPE_SP1]`, on testnets set to
		/// `&[PROOF_TYPE_NAIVE, PROOF_TYPE_SP1]`.
		#[pallet::constant]
		type AllowedProofTypes: Get<&'static [u8]>;

		/// The pallet-assets instance used for managing the reputation token.
		/// Mints reputation tokens 1:1 with native token rewards to proof submitters.
		type ReputationAsset: fungible::Mutate<
			Self::AccountId,
			Balance = <<Self as Config>::Currency as Inspect<Self::AccountId>>::Balance,
		>;

		/// Weight info.
		type WeightInfo: crate::weights::WeightInfo;
	}

	/// `ChildTrieRoot` snapshot at the last messaging reward — dirty-bit for "new dispatches
	/// exist since we last paid".
	#[pallet::storage]
	pub type LastRewardedDispatchRoot<T: Config> = StorageValue<_, H256, OptionQuery>;

	/// Fixed reward amount per eligible proof.
	#[pallet::storage]
	pub type ProofReward<T: Config> =
		StorageValue<_, <<T as Config>::Currency as Inspect<T::AccountId>>::Balance, ValueQuery>;

	/// SP1 verification key hash (ASCII hex), consumed by
	/// `beefy_verifier::sp1::verify_sp1_consensus`.
	#[pallet::storage]
	pub type Sp1VkeyHash<T: Config> = StorageValue<_, Vec<u8>, ValueQuery>;

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
		/// Proof is stale: `latest_height ≤ latest_state_machine_height`, or the proof rotated to
		/// an unexpected authority set.
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
		/// The child trie root was not provided in the proof.
		MissingChildTrieRoot,
		/// The proof does not advance state: no authority set rotation and no new
		/// messages since the last rewarded proof.
		NoNewWork,
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
					consensus_client_id: ismp_beefy::BEEFY_CONSENSUS_ID,
					consensus_state_id: ismp_beefy::BEEFY_CONSENSUS_ID,
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

			let outcome = Self::verify_and_apply(&payload, &signature)?;
			if outcome.has_new_messages {
				LastRewardedDispatchRoot::<T>::put(outcome.child_trie_root);
			}

			let zero = <<T as Config>::Currency as Inspect<T::AccountId>>::Balance::default();
			let reward = ProofReward::<T>::get();
			let reward_paid = if reward > zero {
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

				// Mint reputation tokens 1:1 with the native reward
				if let Err(e) = T::ReputationAsset::mint_into(&payload.submitter, reward) {
					log::warn!(
						target: "ismp",
						"[beefy-consensus-proofs] reputation mint failed: {e:?}",
					);
				}

				reward
			} else {
				zero
			};

			sp_io::offchain_index::set(&types::offchain_key(outcome.latest_height), &payload.proof);
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
			}

			Self::deposit_event(Event::ProofAccepted {
				submitter: payload.submitter.clone(),
				height: outcome.latest_height,
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
					Error::<T>::NoNewWork => 10,
					_ => 0,
				};
				TransactionValidityError::Invalid(InvalidTransaction::Custom(code))
			})?;

			ValidTransaction::with_tag_prefix("BeefyConsensusProofs")
				.longevity(PROOF_LONGEVITY)
				.propagate(true)
				.priority(outcome.latest_height)
				.and_provides(types::PROOF_TAG.encode())
				.build()
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

	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]>,
	{
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
			let msg_preimage = (types::SIGNATURE_DOMAIN, &payload.submitter, proof_digest).encode();
			let signed_msg = sp_io::hashing::keccak_256(&msg_preimage);
			if !sp_io::crypto::sr25519_verify(signature, &signed_msg, &public) {
				Err(Error::<T>::BadSignature)?
			}

			let proof_type = *payload.proof.first().ok_or(Error::<T>::UnknownProofType)?;
			if !T::AllowedProofTypes::get().contains(&proof_type) {
				Err(Error::<T>::UnknownProofType)?
			}
			let abi_payload = &payload.proof[1..];

			let consensus_proof = match proof_type {
				types::PROOF_TYPE_SP1 => {
					let abi_proof =
						<ismp_solidity_abi::sp1_beefy::SP1Beefy::SP1BeefyProof as SolType>::abi_decode_params(
							abi_payload,
						)
						.map_err(|_| Error::<T>::AbiDecodeFailed)?;
					let scale_proof: beefy_verifier_primitives::Sp1BeefyProof = abi_proof.into();
					[&[types::PROOF_TYPE_SP1], scale_proof.encode().as_slice()].concat()
				},
				types::PROOF_TYPE_NAIVE => {
					let abi_proof =
						<ismp_solidity_abi::beefy::BeefyConsensusProof as SolType>::abi_decode_params(
							abi_payload,
						)
						.map_err(|_| Error::<T>::AbiDecodeFailed)?;
					let scale_proof: beefy_verifier_primitives::ConsensusMessage = abi_proof.into();
					[&[types::PROOF_TYPE_NAIVE], scale_proof.encode().as_slice()].concat()
				},
				_ => Err(Error::<T>::UnknownProofType)?,
			};

			// Hand off to pallet-ismp with SCALE-encoded proof for verification.
			let host = pallet_ismp::Pallet::<T>::default();
			let prev_state_bytes = host
				.consensus_state(ismp_beefy::BEEFY_CONSENSUS_ID)
				.map_err(|_| Error::<T>::NotInitialized)?;
			let prev_state: beefy_verifier_primitives::ConsensusState =
				Decode::decode(&mut &prev_state_bytes[..])
					.map_err(|_| Error::<T>::NotInitialized)?;
			let prev_height = Self::latest_height();
			let result = handlers::handle_incoming_message(
				&host,
				Message::Consensus(IsmpConsensusMessage {
					consensus_proof,
					consensus_state_id: ismp_beefy::BEEFY_CONSENSUS_ID,
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
