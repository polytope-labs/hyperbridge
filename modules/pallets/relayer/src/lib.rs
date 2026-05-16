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

//! The pallet-ismp-relayer allows relayers track their deliveries and withdraw their accrued
//! revenues.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod withdrawal;
use crate::withdrawal::{Key, WithdrawalInputData, WithdrawalProof};
use alloc::{collections::BTreeMap, vec, vec::Vec};
use alloy_primitives::Address;
use codec::Encode;
use crypto_utils::verification::Signature;
use evm_state_machine::{
	presets::{
		REQUEST_COMMITMENTS_SLOT, REQUEST_RECEIPTS_SLOT, RESPONSE_COMMITMENTS_SLOT,
		RESPONSE_RECEIPTS_SLOT,
	},
	utils::{add_off_set_to_map_key, derive_unhashed_map_key},
};
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	traits::tokens::{fungible::Mutate, Preservation},
	PalletId,
};
use frame_system::pallet_prelude::OriginFor;
use ismp::{
	dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
	handlers::validate_state_machine,
	host::{IsmpHost, StateMachine},
	messaging::Proof,
};
pub use pallet::*;
use pallet_ismp::{
	child_trie::{RequestCommitments, ResponseCommitments},
	dispatcher::{Message, WithdrawalRequest, HYPERBRIDGE_MODULE_ID},
};
use pallet_ismp_host_executive::{HostParam, HostParams, WithdrawalParams};
use polkadot_sdk::*;
use sp_core::{Get, H256, U256};
use sp_runtime::{traits::AccountIdConversion, AccountId32, DispatchError};

/// Convenience alias for the configured currency's balance.
///
/// This is `pallet-ismp`'s currency balance, which is the runtime's BRIDGE
/// token in production. Reusing `pallet-ismp::Currency` rather than declaring
/// a new `type Currency` on this pallet's `Config` avoids ambiguous
/// associated-type errors in downstream pallets that bound on
/// `pallet_ismp_relayer::Config` (e.g. `pallet-messaging-incentives`).
pub type BalanceOf<T> = <T as pallet_ismp::Config>::Balance;

/// Storage slot of `_epochs` in `HandlerV2`. `_epochs` is HandlerV2's first
/// declared instance variable and HandlerV1 has no instance storage, so the
/// slot is 0. Kept as a constant so a future HandlerV2 layout change is a
/// single-line edit here. Verified via `forge inspect HandlerV2 storage`.
pub const HANDLER_V2_EPOCHS_SLOT: u64 = 0;

/// Claim payload for [`Pallet::claim_outbound_consensus_delivery_reward`].
///
/// A relayer who delivered a mandatory (authority-set rotation) consensus
/// proof to an EVM destination uses this to collect the per-chain
/// `OutboundConsensusDeliveryReward`. The on-chain attribution is in the
/// destination's `HandlerV2._epochs[set_id]` slot — the contract assigns
/// it to `msg.sender` the first time a consensus proof brings the new set
/// id on chain. Verifying the relayer:
///
/// 1. `(destination, set_id)` has not already been claimed.
/// 2. State proof against Hyperbridge's stored commitment for `(destination, height)` yields an
///    `address` at the slot `keccak256(set_id || HANDLER_V2_EPOCHS_SLOT)` of the destination's
///    HandlerV2 contract.
/// 3. The `signature` (`Signature::Evm`) recovers exactly that `address`, signing the
///    [`outbound_consensus_delivery_message`] payload over `(set_id, destination, payee)`.
///
/// Replay protection comes from the on-chain `(destination, set_id)`
/// idempotency tag in [`pallet::OutboundConsensusRotationsClaimed`], not
/// from a per-relayer nonce — once a `(destination, set_id)` has been
/// claimed it cannot be claimed again, so a captured signature has no
/// way to be reused.
///
/// The reward is paid from the treasury to `payee` (an sr25519 account on
/// Hyperbridge that the relayer designates).
#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	codec::Encode,
	codec::Decode,
	codec::DecodeWithMemTracking,
	scale_info::TypeInfo,
)]
pub struct OutboundConsensusDeliveryClaim {
	/// State proof of the destination chain at the height the relayer is
	/// proving against. `state_proof.height.id.state_id` is the EVM
	/// destination; Hyperbridge must already have a state commitment at
	/// `state_proof.height`. Same shape `accumulate_fees`'s
	/// [`WithdrawalProof`] uses for its source/dest proofs.
	pub state_proof: Proof,
	/// Authority set id brought in by the rotation.
	pub set_id: u64,
	/// Sr25519 public key on Hyperbridge that the reward is paid to.
	pub payee: [u8; 32],
	/// `Signature::Evm { address, signature }` from `modules/utils/crypto`.
	/// The signature is over [`outbound_consensus_delivery_message`] of
	/// `(set_id, destination, payee)`. The recovered address must match
	/// the address in the slot proof.
	pub signature: Signature,
}

pub const MODULE_ID: &'static [u8] = b"ISMP-RLYR";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::host::StateMachine;

	use crate::withdrawal::{WithdrawalInputData, WithdrawalProof};
	use codec::Encode;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config
		+ pallet_ismp::Config
		+ pallet_ismp_host_executive::Config
	{
		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost + IsmpDispatcher<Account = Self::AccountId, Balance = Self::Balance>;

		/// Origin for privileged actions
		type RelayerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Treasury account derivation. Outbound consensus delivery rewards
		/// are transferred from the account derived from this `PalletId`.
		/// The treasury must be funded in the same currency as
		/// `pallet-ismp::Config::Currency`.
		#[pallet::constant]
		type TreasuryPalletId: Get<PalletId>;
	}

	/// double map of address to source chain, which holds the amount of the relayer address
	#[pallet::storage]
	#[pallet::getter(fn relayer_fees)]
	pub type Fees<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StateMachine,
		Blake2_128Concat,
		Vec<u8>,
		U256,
		ValueQuery,
	>;

	/// Latest nonce for each address and the state machine they want to withdraw from
	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	pub type Nonce<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		Vec<u8>,
		Blake2_128Concat,
		StateMachine,
		u64,
		ValueQuery,
	>;

	/// Default minimum withdrawal is $10
	pub struct MinWithdrawal;

	impl Get<U256> for MinWithdrawal {
		fn get() -> U256 {
			U256::from(10u128 * 1_000_000_000_000_000_000)
		}
	}

	/// Minimum withdrawal amount
	#[pallet::storage]
	#[pallet::getter(fn min_withdrawal_amount)]
	pub type MinimumWithdrawalAmount<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachine, U256, OptionQuery>;

	/// Per-destination reward, in the runtime's [`Config::Currency`], paid to
	/// the relayer that delivers a mandatory (authority-set rotation)
	/// consensus proof to that destination. `0` (the default) means rewards
	/// are off for the chain.
	#[pallet::storage]
	#[pallet::getter(fn outbound_consensus_delivery_reward)]
	pub type OutboundConsensusDeliveryReward<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachine, BalanceOf<T>, ValueQuery>;

	/// Idempotency map for outbound consensus delivery claims. The presence
	/// of `(destination, set_id)` means some relayer has already collected
	/// the reward for that rotation on that destination.
	#[pallet::storage]
	#[pallet::getter(fn outbound_consensus_rotations_claimed)]
	pub type OutboundConsensusRotationsClaimed<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, StateMachine, Blake2_128Concat, u64, (), OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Withdrawal Proof Validation Error
		ProofValidationError,
		/// Invalid Public Key
		InvalidPublicKey,
		/// Invalid Withdrawal signature
		InvalidSignature,
		/// Empty balance
		EmptyBalance,
		/// Invalid Amount
		NotEnoughBalance,
		/// Encountered a mis-match in the requested state machine
		MismatchedStateMachine,
		/// Relayer Manager Address on Dest chain not set
		MissingMangerAddress,
		/// Failed to dispatch request
		DispatchFailed,
		/// Error
		ErrorCompletingCall,
		/// Missing commitments
		MissingCommitments,
		/// The withdrawal proof's commitments batch contains a duplicate key.
		DuplicateCommitment,
		/// Fee accumulation proof contains no address
		IncompleteProof,
		/// Withdrawal batch contains commitments delivered by more than one
		/// relayer address.
		MixedDeliveryAddressesInBatch,
		/// Signature Decoding Error
		SignatureDecodingError,
		/// `(destination, set_id)` has already been claimed by some relayer.
		OutboundRotationAlreadyClaimed,
		/// Hyperbridge does not yet know a state commitment for the
		/// destination at the proof height. Retry once HB's consensus
		/// client for the destination has advanced past the rotation
		/// landing.
		OutboundDestinationStateNotKnown,
		/// The state proof did not produce an entry at the destination's
		/// `HandlerV2._epochs[set_id]` slot, or the slot is the zero
		/// address.
		OutboundDeliveryNotProven,
		/// Treasury → relayer transfer failed (typically because the
		/// treasury balance is below the configured reward).
		OutboundRewardTransferFailed,
		/// No reward is configured for the destination
		/// (`OutboundConsensusDeliveryReward` is `0`).
		OutboundNoRewardConfigured,
		/// No `HostParams` entry recorded for the destination, so we can't
		/// derive the HandlerV2 contract address to scope the storage key.
		OutboundHostParamsNotKnown,
		/// Per-destination `HostParams` entry is not the EVM variant. The
		/// outbound consensus delivery reward is EVM-only.
		OutboundDestinationNotEvm,
		/// The address recovered from `signature` does not match the
		/// EVM relayer recorded in `HandlerV2._epochs[set_id]`.
		OutboundSignerMismatch,
		/// The signature provided on the outbound consensus delivery claim
		/// is not the [`Signature::Evm`] variant. The attribution is keyed
		/// by an EVM address recovered from a secp256k1 signature, so
		/// substrate-style signatures cannot be matched against the
		/// `HandlerV2._epochs[set_id]` slot.
		OutboundSignatureNotEvm,
	}

	/// Events emiited by the relayer pallet
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A relayer with the `address` has accumulated some fees on the `state_machine`
		AccumulateFees {
			/// relayer address
			address: BoundedVec<u8, ConstU32<32>>,
			/// destination state machine
			state_machine: StateMachine,
			/// Amount accumulated
			amount: U256,
		},
		/// A relayer with the the `address` has initiated a withdrawal on the `state_machine`
		Withdraw {
			/// relayer address
			address: BoundedVec<u8, ConstU32<32>>,
			/// beneficiary address
			beneficiary_address: BoundedVec<u8, ConstU32<32>>,
			/// destination state machine
			state_machine: StateMachine,
			/// Amount withdrawn
			amount: U256,
		},
		/// A relayer has been paid for delivering a mandatory consensus
		/// proof (authority-set rotation) to a destination chain.
		OutboundConsensusDeliveryRewarded {
			/// Destination chain the rotation was delivered to.
			state_machine: StateMachine,
			/// New authority set id brought in by the rotation.
			set_id: u64,
			/// Hyperbridge account credited.
			relayer: T::AccountId,
			/// Amount paid out, in the runtime's `Currency`.
			amount: BalanceOf<T>,
		},
		/// Governance updated the per-chain outbound consensus delivery
		/// reward.
		OutboundConsensusDeliveryRewardUpdated {
			/// Destination chain whose reward was updated.
			state_machine: StateMachine,
			/// New reward amount.
			new_reward: BalanceOf<T>,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::Hash: From<H256>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		T::Balance: Into<u128>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight({1_000_000})]
		pub fn accumulate_fees(
			origin: OriginFor<T>,
			withdrawal_proof: WithdrawalProof,
		) -> DispatchResult {
			ensure_none(origin)?;
			Self::accumulate(withdrawal_proof)
		}

		#[pallet::call_index(1)]
		#[pallet::weight({1_000_000})]
		pub fn withdraw_fees(
			origin: OriginFor<T>,
			withdrawal_data: WithdrawalInputData,
		) -> DispatchResult {
			ensure_none(origin)?;
			Self::withdraw(withdrawal_data)
		}

		/// Sets the minimum withdrawal amount using the correct decimals
		#[pallet::call_index(2)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(0, 1))]
		pub fn set_minimum_withdrawal(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			amount: u128,
		) -> DispatchResult {
			T::RelayerOrigin::ensure_origin(origin)?;
			MinimumWithdrawalAmount::<T>::insert(state_machine, U256::from(amount));
			Ok(())
		}

		/// Pay the configured `OutboundConsensusDeliveryReward` to the EVM
		/// relayer attributed in the destination's `HandlerV2._epochs[set_id]`.
		///
		/// Unsigned. Spam-protected by `validate_unsigned` (the encoded
		/// payload becomes a unique tag, so a duplicate submission with the
		/// same `(destination, set_id)` is rejected at the txpool stage).
		#[pallet::call_index(3)]
		#[pallet::weight({1_000_000})]
		pub fn claim_outbound_consensus_delivery_reward(
			origin: OriginFor<T>,
			claim: OutboundConsensusDeliveryClaim,
		) -> DispatchResult {
			ensure_none(origin)?;
			Self::process_outbound_consensus_delivery_claim(claim)
		}

		/// Governance-set per-chain reward for delivering mandatory consensus
		/// proofs to that destination.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(0, 1))]
		pub fn set_outbound_consensus_delivery_reward(
			origin: OriginFor<T>,
			state_machine: StateMachine,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::RelayerOrigin::ensure_origin(origin)?;
			OutboundConsensusDeliveryReward::<T>::insert(state_machine, amount);
			Self::deposit_event(Event::OutboundConsensusDeliveryRewardUpdated {
				state_machine,
				new_reward: amount,
			});
			Ok(())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T>
	where
		<T as frame_system::Config>::Hash: From<H256>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		T::Balance: Into<u128>,
	{
		type Call = Call<T>;

		// empty pre-dispatch so we don't modify storage
		fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
			Ok(())
		}

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			let res = match call {
				Call::accumulate_fees { withdrawal_proof } =>
					Self::accumulate(withdrawal_proof.clone()),
				Call::withdraw_fees { withdrawal_data } => Self::withdraw(withdrawal_data.clone()),
				Call::claim_outbound_consensus_delivery_reward { claim } =>
					Self::process_outbound_consensus_delivery_claim(claim.clone()),
				_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
			};

			if let Err(err) = res {
				log::error!(target: "ismp", "Pallet Relayer Fees error {err:?}");
				Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?
			}

			let encoding = match call {
				Call::accumulate_fees { withdrawal_proof } => withdrawal_proof.encode(),
				Call::withdraw_fees { withdrawal_data } => withdrawal_data.encode(),
				Call::claim_outbound_consensus_delivery_reward { claim } => claim.encode(),
				_ => unreachable!(),
			};

			let msg_hash = sp_io::hashing::keccak_256(&encoding).to_vec();

			Ok(ValidTransaction {
				priority: 100,
				requires: vec![],
				provides: vec![msg_hash],
				longevity: TransactionLongevity::MAX,
				propagate: true,
			})
		}
	}
}

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::Hash: From<H256>,
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	T::Balance: Into<u128>,
{
	/// Verify and pay out an outbound-consensus delivery claim.
	///
	/// See [`OutboundConsensusDeliveryClaim`] for the verification pipeline.
	pub fn process_outbound_consensus_delivery_claim(
		claim: OutboundConsensusDeliveryClaim,
	) -> DispatchResult {
		let OutboundConsensusDeliveryClaim { state_proof, set_id, payee, signature } = claim;
		let destination = state_proof.height.id.state_id;

		// The attribution mechanism recovers an EVM address from the
		// signature and matches it against the `HandlerV2._epochs[set_id]`
		// slot, so only secp256k1/EVM signatures are meaningful here.
		// Reject the substrate variants up front to avoid burning the rest
		// of the verification pipeline on a claim that can never match.
		ensure!(matches!(signature, Signature::Evm { .. }), Error::<T>::OutboundSignatureNotEvm,);

		ensure!(
			!OutboundConsensusRotationsClaimed::<T>::contains_key(destination, set_id),
			Error::<T>::OutboundRotationAlreadyClaimed,
		);

		// HandlerV2 lookup. The address lives in `HostParams` alongside
		// `host_manager`; non-EVM destinations are rejected since the
		// attribution mechanism is HandlerV2-specific.
		let HostParam::EvmHostParam(evm_params) =
			HostParams::<T>::get(destination).ok_or(Error::<T>::OutboundHostParamsNotKnown)?;
		let handler_v2 = evm_params.handler;

		// 52-byte storage key the EVM state proof verifier expects:
		// `handler_v2 (20) || keccak256(set_id || HANDLER_V2_EPOCHS_SLOT) (32)`.
		let slot_hash = evm_state_machine::utils::derive_unhashed_map_key::<<T as Config>::IsmpHost>(
			U256::from(set_id).to_big_endian().to_vec(),
			HANDLER_V2_EPOCHS_SLOT,
		);
		let mut key = Vec::with_capacity(52);
		key.extend_from_slice(&handler_v2.0);
		key.extend_from_slice(&slot_hash.0);

		let proof_results = Self::verify_withdrawal_proof(&state_proof, vec![key.clone()])
			.map_err(|_| Error::<T>::OutboundDestinationStateNotKnown)?;
		let raw = proof_results
			.get(&key)
			.cloned()
			.flatten()
			.ok_or(Error::<T>::OutboundDeliveryNotProven)?;

		// `raw` is the trie-level value of `HandlerV2._epochs[set_id]`;
		// `decode_epochs_slot_address` handles the RLP-encoded form the
		// Ethereum trie stores. Returns `None` for an unset / zero-address
		// slot, which we surface as `OutboundDeliveryNotProven` (logically
		// equivalent to "no delivery proven yet").
		let evm_address =
			Self::decode_epochs_slot_address(&raw).ok_or(Error::<T>::OutboundDeliveryNotProven)?;

		// Replay protection comes from the `OutboundConsensusRotationsClaimed`
		let msg = outbound_consensus_delivery_message(set_id, destination, payee);
		let recovered = signature.verify(&msg, None).map_err(|_| Error::<T>::InvalidSignature)?;
		let recovered_address = Address::try_from(recovered.as_slice())
			.map_err(|_| Error::<T>::OutboundSignerMismatch)?;
		ensure!(recovered_address == evm_address, Error::<T>::OutboundSignerMismatch);

		let reward = OutboundConsensusDeliveryReward::<T>::get(destination);
		ensure!(reward > BalanceOf::<T>::default(), Error::<T>::OutboundNoRewardConfigured);

		let treasury: T::AccountId =
			<T as Config>::TreasuryPalletId::get().into_account_truncating();
		let payee_account: T::AccountId = payee.into();
		<<T as pallet_ismp::Config>::Currency as Mutate<T::AccountId>>::transfer(
			&treasury,
			&payee_account,
			reward,
			Preservation::Preserve,
		)
		.map_err(|_| Error::<T>::OutboundRewardTransferFailed)?;

		OutboundConsensusRotationsClaimed::<T>::insert(destination, set_id, ());

		Self::deposit_event(Event::OutboundConsensusDeliveryRewarded {
			state_machine: destination,
			set_id,
			relayer: payee_account,
			amount: reward,
		});

		Ok(())
	}

	pub fn withdraw(withdrawal_data: WithdrawalInputData) -> DispatchResult {
		let address = match &withdrawal_data.signature {
			Signature::Evm { address, .. } => address.clone(),
			Signature::Sr25519 { public_key, .. } => public_key.clone(),
			Signature::Ed25519 { public_key, .. } => public_key.clone(),
		};

		let nonce = Nonce::<T>::get(address.clone(), withdrawal_data.dest_chain);
		let msg = message(nonce, withdrawal_data.dest_chain, withdrawal_data.beneficiary.clone());

		match &withdrawal_data.signature {
			Signature::Evm { address, .. } => {
				let eth_address = withdrawal_data
					.signature
					.verify(&msg, None)
					.map_err(|_| Error::<T>::InvalidSignature)?;
				if &eth_address != address {
					Err(Error::<T>::InvalidPublicKey)?
				}
			},
			Signature::Sr25519 { .. } => {
				// Verify signature with public key provided in signature enum
				withdrawal_data
					.signature
					.verify(&msg, None)
					.map_err(|_| Error::<T>::InvalidSignature)?;
			},
			Signature::Ed25519 { .. } => {
				// Verify signature with public key provided in signature enum
				withdrawal_data
					.signature
					.verify(&msg, None)
					.map_err(|_| Error::<T>::InvalidSignature)?;
			},
		};
		let available_amount = Fees::<T>::get(withdrawal_data.dest_chain, address.clone());

		if available_amount <
			Self::min_withdrawal_amount(withdrawal_data.dest_chain)
				.unwrap_or(MinWithdrawal::get())
		{
			Err(Error::<T>::NotEnoughBalance)?
		}

		let dispatcher = <T as Config>::IsmpHost::default();

		Nonce::<T>::try_mutate(address.clone(), withdrawal_data.dest_chain, |value| {
			*value += 1;
			Ok::<(), ()>(())
		})
		.map_err(|_| Error::<T>::ErrorCompletingCall)?;

		let beneficiary_address = withdrawal_data.beneficiary.clone().unwrap_or(address.clone());
		let (to, body) = match withdrawal_data.dest_chain {
			s if s.is_substrate() => (
				HYPERBRIDGE_MODULE_ID.to_vec(),
				Message::WithdrawRelayerFees(WithdrawalRequest {
					amount: available_amount.low_u128(),
					account: AccountId32::try_from(&beneficiary_address[..])
						.map_err(|_| Error::<T>::InvalidPublicKey)?,
				})
				.encode(),
			),
			_ => {
				let HostParam::EvmHostParam(params) =
					HostParams::<T>::get(withdrawal_data.dest_chain)
						.ok_or_else(|| Error::<T>::MissingMangerAddress)?;

				let body = WithdrawalParams {
					beneficiary_address: beneficiary_address.clone(),
					amount: available_amount.into(),
					// Withdraw in the EVM host's configured fee token. Address-zero
					// here is the canonical "use the host's default" sentinel; the
					// EVM-side host treats it as the chain's native asset and the
					// fee-token path is taken when this is non-zero.
					token: Default::default(),
				}
				.abi_encode()
				.map_err(|_| Error::<T>::InvalidPublicKey)?;

				(params.host_manager.0.to_vec(), body)
			},
		};

		let post = DispatchPost {
			dest: withdrawal_data.dest_chain,
			from: MODULE_ID.to_vec(),
			to,
			body,
			timeout: 0,
		};

		// Account is not useful in this case
		dispatcher
			.dispatch_request(
				DispatchRequest::Post(post),
				FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
			)
			.map_err(|_| Error::<T>::DispatchFailed)?;

		Fees::<T>::insert(withdrawal_data.dest_chain, address.clone(), U256::zero());

		Self::deposit_event(Event::<T>::Withdraw {
			address: sp_runtime::BoundedVec::truncate_from(address.clone()),
			beneficiary_address: sp_runtime::BoundedVec::truncate_from(beneficiary_address),
			state_machine: withdrawal_data.dest_chain,
			amount: available_amount,
		});

		Ok(())
	}

	pub fn accumulate(mut withdrawal_proof: WithdrawalProof) -> DispatchResult {
		// Reject duplicate commitments within the batch. The wire format is a
		// `Vec` and this extrinsic is unsigned, so this is the line of defence
		// against an attacker padding the batch with identical commitments to
		// double-claim fees.
		let mut seen = alloc::collections::BTreeSet::new();
		for key in withdrawal_proof.commitments.iter() {
			ensure!(seen.insert(key.encode()), Error::<T>::DuplicateCommitment);
		}

		// Filter out already-claimed / missing commitments
		withdrawal_proof.commitments = withdrawal_proof
			.commitments
			.into_iter()
			.filter(|key| match key {
				Key::Request(req) => {
					match RequestCommitments::<T>::get(*req) {
						Some(leaf_meta) => !leaf_meta.claimed,
						// If request commitment does not exist in storage which should not be
						// possible, we skip it
						None => false,
					}
				},
				Key::Response { response_commitment, .. } => {
					match ResponseCommitments::<T>::get(*response_commitment) {
						Some(leaf_meta) => !leaf_meta.claimed,
						// If request commitment does not exist in storage which should not be
						// possible, we skip it
						None => false,
					}
				},
			})
			.collect();
		ensure!(!withdrawal_proof.commitments.is_empty(), Error::<T>::MissingCommitments);
		let source_keys = Self::get_commitment_keys(&withdrawal_proof);
		let dest_keys = Self::get_receipt_keys(&withdrawal_proof);
		let state_machine = withdrawal_proof.source_proof.height.id.state_id;
		// For evm chains each response receipt occupies two slots
		let mut slot_2_keys = alloc::vec![];
		match &withdrawal_proof.dest_proof.height.id.state_id {
			s if s.is_evm() => {
				for (key, commitment) in dest_keys.iter().zip(withdrawal_proof.commitments.iter()) {
					match commitment {
						Key::Response { .. } => {
							slot_2_keys.push(add_off_set_to_map_key(key, 1).0.to_vec());
						},
						_ => {},
					}
				}
			},
			_ => {},
		}

		let source_result =
			Self::verify_withdrawal_proof(&withdrawal_proof.source_proof, source_keys.clone())?;
		let dest_result = Self::verify_withdrawal_proof(
			&withdrawal_proof.dest_proof,
			dest_keys.clone().into_iter().chain(slot_2_keys).collect(),
		)?;
		let (result, claimed_commitments) = Self::validate_results(
			&withdrawal_proof,
			source_keys,
			dest_keys,
			source_result,
			dest_result,
		)?;

		let mut entries = result.into_iter();
		let (delivery_address, total_fee) = entries.next().ok_or(Error::<T>::IncompleteProof)?;
		// Every commitment in the batch must share a single delivery address.
		ensure!(entries.next().is_none(), Error::<T>::MixedDeliveryAddressesInBatch);

		// Let's verify the beneficiary address
		let beneficiary_address = if let Some((beneficiary_address, signature)) =
			withdrawal_proof.beneficiary_details
		{
			let msg = sp_io::hashing::keccak_256(&beneficiary_address);
			match &signature {
				Signature::Evm { .. } => {
					let eth_address =
						signature.verify(&msg, None).map_err(|_| Error::<T>::InvalidSignature)?;
					if eth_address != delivery_address {
						Err(Error::<T>::InvalidPublicKey)?
					}
				},
				Signature::Sr25519 { .. } | Signature::Ed25519 { .. } => {
					// verify the signature with the delivery address from the state proof
					let _ = signature
						.verify(&msg, Some(delivery_address))
						.map_err(|_| Error::<T>::InvalidSignature)?;
				},
			}

			let _ = Fees::<T>::try_mutate(state_machine, beneficiary_address.clone(), |inner| {
				*inner += total_fee;
				Ok::<(), ()>(())
			});

			beneficiary_address
		} else {
			let _ = Fees::<T>::try_mutate(state_machine, delivery_address.clone(), |inner| {
				*inner += total_fee;
				Ok::<(), ()>(())
			});

			delivery_address
		};

		for key in withdrawal_proof.commitments {
			match key {
				Key::Request(req) => {
					if !claimed_commitments.contains(&req) {
						continue;
					}
					match RequestCommitments::<T>::get(req) {
						Some(mut leaf_meta) => {
							leaf_meta.claimed = true;
							RequestCommitments::<T>::insert(req, leaf_meta)
						},
						// Unreachable
						None => {},
					}
				},
				Key::Response { response_commitment, .. } => {
					if !claimed_commitments.contains(&response_commitment) {
						continue;
					}
					match ResponseCommitments::<T>::get(response_commitment) {
						Some(mut leaf_meta) => {
							leaf_meta.claimed = true;
							ResponseCommitments::<T>::insert(response_commitment, leaf_meta);
						},
						// Unreachable
						None => {},
					}
				},
			}
		}

		Self::deposit_event(Event::<T>::AccumulateFees {
			address: sp_runtime::BoundedVec::truncate_from(beneficiary_address),
			state_machine,
			amount: total_fee,
		});

		Ok(())
	}
	/// Decode the EVM `address` value stored at
	/// `HandlerV2._epochs[set_id]`, as returned by
	/// `EvmStateMachine::verify_state_proof`.
	pub fn decode_epochs_slot_address(raw: &[u8]) -> Option<Address> {
		use alloy_rlp::Decodable;
		let addr = Address::decode(&mut &*raw).ok()?;
		if addr == Address::ZERO {
			None
		} else {
			Some(addr)
		}
	}

	pub fn verify_withdrawal_proof(
		proof: &Proof,
		keys: Vec<Vec<u8>>,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, DispatchError> {
		let host = <T as Config>::IsmpHost::default();
		let state_machine = validate_state_machine(&host, proof.height)
			.map_err(|_| Error::<T>::ProofValidationError)?;
		let state = host
			.state_machine_commitment(proof.height)
			.map_err(|_| Error::<T>::ProofValidationError)?;
		let result = state_machine
			.verify_state_proof(&host, keys, state, proof)
			.map_err(|_| Error::<T>::ProofValidationError)?;

		Ok(result)
	}

	pub fn get_commitment_keys(proof: &WithdrawalProof) -> Vec<Vec<u8>> {
		let mut keys = vec![];
		for key in &proof.commitments {
			match key {
				Key::Request(commitment) => match proof.source_proof.height.id.state_id {
					s if s.is_evm() => {
						keys.push(
							derive_unhashed_map_key::<<T as Config>::IsmpHost>(
								commitment.0.to_vec(),
								REQUEST_COMMITMENTS_SLOT,
							)
							.0
							.to_vec(),
						);
					},
					s if s.is_substrate() =>
						keys.push(RequestCommitments::<T>::storage_key(*commitment)),
					// unsupported
					_ => {},
				},
				Key::Response { response_commitment, .. } => {
					match proof.source_proof.height.id.state_id {
						s if s.is_evm() => {
							keys.push(
								derive_unhashed_map_key::<<T as Config>::IsmpHost>(
									response_commitment.0.to_vec(),
									RESPONSE_COMMITMENTS_SLOT,
								)
								.0
								.to_vec(),
							);
						},
						s if s.is_substrate() =>
							keys.push(ResponseCommitments::<T>::storage_key(*response_commitment)),
						// unsupported
						_ => {},
					}
				},
			}
		}

		keys
	}

	pub fn get_receipt_keys(proof: &WithdrawalProof) -> Vec<Vec<u8>> {
		let mut keys = vec![];
		for key in &proof.commitments {
			match key {
				Key::Request(commitment) => match proof.dest_proof.height.id.state_id {
					s if s.is_evm() => {
						keys.push(
							derive_unhashed_map_key::<<T as Config>::IsmpHost>(
								commitment.0.to_vec(),
								REQUEST_RECEIPTS_SLOT,
							)
							.0
							.to_vec(),
						);
					},
					s if s.is_substrate() => keys.push(
						pallet_ismp::child_trie::RequestReceipts::<T>::storage_key(*commitment),
					),
					// unsupported
					_ => {},
				},
				Key::Response { request_commitment, .. } => {
					match proof.dest_proof.height.id.state_id {
						s if s.is_evm() => {
							keys.push(
								derive_unhashed_map_key::<<T as Config>::IsmpHost>(
									request_commitment.0.to_vec(),
									RESPONSE_RECEIPTS_SLOT,
								)
								.0
								.to_vec(),
							);
						},
						s if s.is_substrate() =>
							keys.push(pallet_ismp::child_trie::ResponseReceipts::<T>::storage_key(
								*request_commitment,
							)),
						// unsupported
						_ => {},
					}
				},
			}
		}

		keys
	}

	pub fn validate_results(
		proof: &WithdrawalProof,
		source_keys: Vec<Vec<u8>>,
		dest_keys: Vec<Vec<u8>>,
		source_result: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
		dest_result: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
	) -> Result<(BTreeMap<Vec<u8>, U256>, Vec<H256>), Error<T>> {
		let mut result = BTreeMap::new();
		// Only store commitments that were claimed
		let mut commitments = Vec::new();
		for ((key, source_key), dest_key) in
			proof.commitments.clone().into_iter().zip(source_keys).zip(dest_keys)
		{
			match key {
				Key::Request(commitment) => {
					let encoded_metadata =
						if let Some(encoded) = source_result.get(&source_key).cloned().flatten() {
							encoded
						} else {
							// If fee is a null value skip it, evm returns non membership proof for
							// zero values
							continue;
						};

					let fee = match proof.source_proof.height.id.state_id {
						s if s.is_evm() => {
							use alloy_rlp::Decodable;
							let fee = alloy_primitives::U256::decode(&mut &*encoded_metadata)
								.map_err(|_| Error::<T>::ProofValidationError)?;
							U256::from_big_endian(&fee.to_be_bytes::<32>())
						},
						s if s.is_substrate() => {
							use codec::Decode;
							let fee: u128 = pallet_ismp::dispatcher::RequestMetadata::<T>::decode(
								&mut &*encoded_metadata,
							)
							.map_err(|_| Error::<T>::ProofValidationError)?
							.fee
							.fee
							.into();
							U256::from(fee)
						},
						// unsupported
						_ => Err(Error::<T>::MismatchedStateMachine)?,
					};
					let encoded_receipt = dest_result
						.get(&dest_key)
						.cloned()
						.flatten()
						.ok_or_else(|| Error::<T>::ProofValidationError)?;
					let address = match proof.dest_proof.height.id.state_id {
						s if s.is_evm() => {
							use alloy_rlp::Decodable;
							Address::decode(&mut &*encoded_receipt)
								.map_err(|_| Error::<T>::ProofValidationError)?
								.0
								.to_vec()
						},
						s if s.is_substrate() => {
							use codec::Decode;
							let relayer_bytes = <Vec<u8>>::decode(&mut &*encoded_receipt)
								.map_err(|_| Error::<T>::ProofValidationError)?;
							if relayer_bytes.len() > 32 {
								let signature = Signature::decode(&mut &*relayer_bytes)
									.map_err(|_| Error::<T>::SignatureDecodingError)?;
								signature.signer()
							} else {
								relayer_bytes
							}
						},
						// unsupported
						_ => Err(Error::<T>::MismatchedStateMachine)?,
					};
					let entry = result.entry(address).or_insert(U256::zero());
					*entry += fee;
					commitments.push(commitment);
				},
				Key::Response { response_commitment, .. } => {
					let encoded_metadata =
						if let Some(encoded) = source_result.get(&source_key).cloned().flatten() {
							encoded
						} else {
							continue;
						};
					let fee = match proof.source_proof.height.id.state_id {
						s if s.is_evm() => {
							use alloy_rlp::Decodable;
							let fee = alloy_primitives::U256::decode(&mut &*encoded_metadata)
								.map_err(|_| Error::<T>::ProofValidationError)?;
							U256::from_big_endian(&fee.to_be_bytes::<32>())
						},
						s if s.is_substrate() => {
							use codec::Decode;
							let fee: u128 = pallet_ismp::dispatcher::RequestMetadata::<T>::decode(
								&mut &*encoded_metadata,
							)
							.map_err(|_| Error::<T>::ProofValidationError)?
							.fee
							.fee
							.into();
							U256::from(fee)
						},
						// unsupported
						_ => Err(Error::<T>::MismatchedStateMachine)?,
					};
					let encoded_receipt = dest_result
						.get(&dest_key)
						.cloned()
						.flatten()
						.ok_or_else(|| Error::<T>::ProofValidationError)?;
					let (relayer, res) = {
						match proof.dest_proof.height.id.state_id {
							s if s.is_evm() => {
								use alloy_rlp::Decodable;
								let response_commitment =
									alloy_primitives::B256::decode(&mut &*encoded_receipt)
										.map_err(|_| Error::<T>::ProofValidationError)?;
								let slot_2_key = add_off_set_to_map_key(&dest_key, 1);
								let encoded_address = dest_result
									.get(&slot_2_key.0.to_vec())
									.cloned()
									.flatten()
									.ok_or_else(|| Error::<T>::ProofValidationError)?;
								let address = Address::decode(&mut &*encoded_address)
									.map_err(|_| Error::<T>::ProofValidationError)?
									.0
									.to_vec();
								(address, response_commitment.0)
							},
							s if s.is_substrate() => {
								use codec::Decode;
								let receipt =
									pallet_ismp::ResponseReceipt::decode(&mut &*encoded_receipt)
										.map_err(|_| Error::<T>::ProofValidationError)?;
								let relayer = if receipt.relayer.len() > 32 {
									let signature = Signature::decode(&mut &*receipt.relayer)
										.map_err(|_| Error::<T>::SignatureDecodingError)?;
									signature.signer()
								} else {
									receipt.relayer
								};
								(relayer, receipt.response.0)
							},
							// unsupported
							_ => Err(Error::<T>::MismatchedStateMachine)?,
						}
					};

					if response_commitment.0 != res {
						Err(Error::<T>::ProofValidationError)?
					}
					let entry = result.entry(relayer).or_insert(0u32.into());
					*entry += fee;
					commitments.push(response_commitment);
				},
			}
		}

		Ok((result, commitments))
	}
}

impl<T: Config> Pallet<T> {
	pub fn accumulate_fee_and_deposit_event(
		state_machine: StateMachine,
		address: Vec<u8>,
		fee: U256,
	) {
		let _ = Fees::<T>::try_mutate(state_machine, address.clone(), |inner| {
			*inner += fee;
			Ok::<(), ()>(())
		});

		Self::deposit_event(Event::<T>::AccumulateFees {
			address: sp_runtime::BoundedVec::truncate_from(address),
			state_machine,
			amount: fee,
		});
	}
}

pub fn message(nonce: u64, dest_chain: StateMachine, beneficiary: Option<Vec<u8>>) -> [u8; 32] {
	if let Some(beneficiary) = beneficiary {
		return sp_io::hashing::keccak_256(&(nonce, dest_chain, beneficiary).encode());
	}
	sp_io::hashing::keccak_256(&(nonce, dest_chain).encode())
}

/// Signed payload for [`OutboundConsensusDeliveryClaim`]. Replay protection
/// comes from the on-chain `(destination, set_id)` idempotency tag in
/// [`pallet::OutboundConsensusRotationsClaimed`], not from a per-relayer
/// nonce — once a `(destination, set_id)` has been claimed it cannot be
/// claimed again, so a captured signature has no way to be reused.
pub fn outbound_consensus_delivery_message(
	set_id: u64,
	dest_chain: StateMachine,
	payee: [u8; 32],
) -> [u8; 32] {
	sp_io::hashing::keccak_256(&(set_id, dest_chain, payee).encode())
}
