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

pub mod accumulate;
pub mod outbound_consensus;
pub mod outbound_request;
pub mod withdrawal;

pub use accumulate::beneficiary_message;
pub use outbound_consensus::*;
pub use outbound_request::*;
pub use withdrawal::message;

use alloc::{vec, vec::Vec};
use frame_support::{dispatch::DispatchResult, PalletId};
use frame_system::pallet_prelude::OriginFor;
use ismp::{
	dispatcher::IsmpDispatcher,
	host::{IsmpHost, StateMachine},
};
pub use pallet::*;
use polkadot_sdk::*;
use sp_core::{H256, U256};

/// Convenience alias for the configured currency's balance.
///
/// This is `pallet-ismp`'s currency balance, which is the runtime's BRIDGE
/// token in production. Reusing `pallet-ismp::Currency` rather than declaring
/// a new `type Currency` on this pallet's `Config` avoids ambiguous
/// associated-type errors in downstream pallets that bound on
/// `pallet_ismp_relayer::Config` (e.g. `pallet-messaging-incentives`).
pub type BalanceOf<T> = <T as pallet_ismp::Config>::Balance;

pub const MODULE_ID: &'static [u8] = b"ISMP-RLYR";

/// Upper bound on the `module_id` (request `from` field) used as a storage
/// key in [`pallet::OutboundRequestDeliveryReward`]. Substrate pallet ids are
/// 8 bytes, EVM contract addresses 20, and various module identifiers go up
/// to 32 bytes in this codebase. 64 leaves headroom without bloating the
/// storage key.
pub type ModuleIdBound = sp_core::ConstU32<64>;

/// Re-exported destination-side storage slot of `_requestReceipts` so
/// off-chain callers (tesseract) and on-chain callers see the same source
/// of truth.
pub use evm_state_machine::presets::REQUEST_RECEIPTS_SLOT;

/// Returns true if `state_id` is a Pharos state machine. Pharos stores trie
/// values as raw ABI-encoded bytes rather than RLP, so the proof-decoding
/// paths in this pallet branch on it.
pub fn is_pharos(state_id: &StateMachine) -> bool {
	use pharos_primitives::{PHAROS_ATLANTIC_CHAIN_ID, PHAROS_MAINNET_CHAIN_ID};
	matches!(
		state_id,
		StateMachine::Evm(id) if *id == PHAROS_MAINNET_CHAIN_ID || *id == PHAROS_ATLANTIC_CHAIN_ID
	)
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::withdrawal::{WithdrawalInputData, WithdrawalProof};
	use codec::Encode;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

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

	/// Per-`module_id` reward, in the runtime's [`Config::Currency`], paid to
	/// the relayer that delivers a hyperbridge-originated request from that
	/// module to a destination. `0` (the default) means both "no reward" and
	/// "module not on the allowlist". Governance enables a module by setting a
	/// non-zero value.
	#[pallet::storage]
	#[pallet::getter(fn outbound_request_delivery_reward)]
	pub type OutboundRequestDeliveryReward<T: Config> =
		StorageMap<_, Blake2_128Concat, BoundedVec<u8, ModuleIdBound>, BalanceOf<T>, ValueQuery>;

	/// Idempotency map for outbound request delivery claims. The presence of
	/// `commitment` means some relayer has already collected the reward for
	/// delivering that request.
	#[pallet::storage]
	#[pallet::getter(fn outbound_requests_claimed)]
	pub type OutboundRequestsClaimed<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, (), OptionQuery>;

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
		/// `EvmHost._epochs[set_id]` slot, or the slot is the zero
		/// address.
		OutboundDeliveryNotProven,
		/// Treasury → relayer transfer failed (typically because the
		/// treasury balance is below the configured reward).
		OutboundRewardTransferFailed,
		/// No reward is configured for the destination
		/// (`OutboundConsensusDeliveryReward` is `0`).
		OutboundNoRewardConfigured,
		/// No `EvmHost` address recorded for the destination, so we can't
		/// scope the storage key. Set via `pallet-ismp-host-executive`.
		OutboundHostNotKnown,
		/// Per-destination `HostParams` entry is not the EVM variant. The
		/// outbound consensus delivery reward is EVM-only.
		OutboundDestinationNotEvm,
		/// The address recovered from `signature` does not match the
		/// EVM relayer recorded in `EvmHost._epochs[set_id]`.
		OutboundSignerMismatch,
		/// The signature provided on the outbound consensus delivery claim
		/// is not the [`Signature::Evm`] variant. The attribution is keyed
		/// by an EVM address recovered from a secp256k1 signature, so
		/// substrate-style signatures cannot be matched against the
		/// `EvmHost._epochs[set_id]` slot.
		OutboundSignatureNotEvm,
		/// `commitment` has already been claimed by some relayer.
		OutboundRequestAlreadyClaimed,
		/// The commitment is not in Hyperbridge's local
		/// `pallet_ismp::child_trie::RequestCommitments`, so it was not
		/// dispatched from Hyperbridge and is not eligible for this reward.
		OutboundRequestNotKnown,
		/// `request.source` is not this hyperbridge instance.
		OutboundRequestSourceNotHyperbridge,
		/// `request.from` exceeds [`ModuleIdBound`] and therefore cannot be
		/// keyed into the reward map; treated as not on the allowlist.
		OutboundRequestModuleIdTooLong,
		/// No reward is configured for `request.from`. Either the module is
		/// not on the allowlist, or governance has paused it.
		OutboundRequestNoRewardConfigured,
		/// Treasury → relayer transfer failed (typically because the treasury
		/// balance is below the configured reward).
		OutboundRequestRewardTransferFailed,
		/// The signer recovered from `signature` does not match the relayer
		/// recorded in the destination's `RequestReceipts[commitment]`.
		OutboundRequestSignerMismatch,
		/// The destination state machine is neither EVM nor substrate, so we
		/// don't know how to decode `RequestReceipts[commitment]`.
		OutboundRequestUnsupportedDestination,
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
		/// A relayer has been paid for delivering a hyperbridge-originated
		/// request to a destination chain.
		OutboundRequestDeliveryRewarded {
			/// Request commitment that was delivered.
			commitment: H256,
			/// Destination chain the request was delivered to.
			state_machine: StateMachine,
			/// Source module id (`request.from`) used as the allowlist key.
			module_id: BoundedVec<u8, ModuleIdBound>,
			/// Hyperbridge account credited.
			relayer: T::AccountId,
			/// Amount paid out, in the runtime's `Currency`.
			amount: BalanceOf<T>,
		},
		/// Governance updated the per-`module_id` outbound request delivery
		/// reward.
		OutboundRequestDeliveryRewardUpdated {
			/// Source module id whose reward was updated.
			module_id: BoundedVec<u8, ModuleIdBound>,
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
		/// relayer attributed in the destination's `EvmHost._epochs[set_id]`.
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

		/// Pay the configured `OutboundRequestDeliveryReward` to the relayer
		/// that delivered a hyperbridge-originated request to the destination.
		///
		/// Unsigned. Spam-protected by `validate_unsigned` (the encoded
		/// payload becomes a unique tag, so a duplicate submission with the
		/// same `commitment` is rejected at the txpool stage).
		#[pallet::call_index(5)]
		#[pallet::weight({1_000_000})]
		pub fn claim_outbound_request_delivery_reward(
			origin: OriginFor<T>,
			claim: OutboundRequestDeliveryClaim,
		) -> DispatchResult {
			ensure_none(origin)?;
			Self::process_outbound_request_delivery_claim(claim)
		}

		/// Governance-set per-`module_id` reward for delivering a
		/// hyperbridge-originated request from that module. Setting
		/// `amount = 0` removes the module from the allowlist.
		#[pallet::call_index(6)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(0, 1))]
		pub fn set_outbound_request_delivery_reward(
			origin: OriginFor<T>,
			module_id: BoundedVec<u8, ModuleIdBound>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::RelayerOrigin::ensure_origin(origin)?;
			OutboundRequestDeliveryReward::<T>::insert(&module_id, amount);
			Self::deposit_event(Event::OutboundRequestDeliveryRewardUpdated {
				module_id,
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
				Call::claim_outbound_request_delivery_reward { claim } =>
					Self::process_outbound_request_delivery_claim(claim.clone()),
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
				Call::claim_outbound_request_delivery_reward { claim } => claim.encode(),
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
