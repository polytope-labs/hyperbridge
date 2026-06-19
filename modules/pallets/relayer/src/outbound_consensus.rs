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

//! Outbound consensus delivery rewards.
//!
//! Relayers that deliver mandatory (authority-set rotation) consensus proofs
//! to an EVM destination earn a per-chain `OutboundConsensusDeliveryReward`.
//! The on-chain attribution lives in the destination's
//! `EvmHost._epochs[set_id]` slot, populated by `HandlerV2.handleConsensus`
//! via `EvmHost.recordEpoch(set_id, msg.sender)` the first time a consensus
//! proof brings the new set id on chain. This module proves the slot value
//! against Hyperbridge's stored state commitment for the destination and
//! transfers the configured reward.

use crate::{
	BalanceOf, Config, Error, Event, OutboundConsensusDeliveryReward,
	OutboundConsensusRotationsClaimed, Pallet,
};
use alloc::{vec, vec::Vec};
use alloy_primitives::Address;
use codec::Encode;
use crypto_utils::verification::Signature;
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	traits::tokens::{fungible::Mutate, Preservation},
};
use ismp::{host::StateMachine, messaging::Proof};
use pallet_ismp_host_executive::EvmHosts;
use polkadot_sdk::*;
use sp_core::{Get, H256, U256};
use sp_runtime::traits::AccountIdConversion;

/// Storage slot of `_epochs` on the `EvmHost`. `_epochs` is declared after
/// `_consensusUpdateTimestamp` (slot 20) on `EvmHost`, putting it at slot 21.
/// Kept as a constant so a future `EvmHost` layout change is a single-line edit
/// here. Verified via `forge inspect EvmHost storage`.
pub const EVM_HOST_EPOCHS_SLOT: u64 = 21;

/// Claim payload for [`Pallet::claim_outbound_consensus_delivery_reward`].
///
/// A relayer who delivered a mandatory (authority-set rotation) consensus
/// proof to an EVM destination uses this to collect the per-chain
/// `OutboundConsensusDeliveryReward`. The on-chain attribution is in the
/// destination's `EvmHost._epochs[set_id]` slot — `HandlerV2.handleConsensus`
/// forwards to `EvmHost.recordEpoch(set_id, msg.sender)` the first time a
/// consensus proof brings the new set id on chain. Verifying the relayer:
///
/// 1. `(destination, set_id)` has not already been claimed.
/// 2. State proof against Hyperbridge's stored commitment for `(destination, height)` yields an
///    `address` at the slot `keccak256(set_id || EVM_HOST_EPOCHS_SLOT)` of the destination's
///    `EvmHost` contract.
/// 3. The `signature` (`Signature::Evm`) recovers exactly that `address`, signing the
///    [`outbound_consensus_delivery_message`] payload over `(set_id, destination, payee)`.
///
/// Replay protection comes from the on-chain `(destination, set_id)`
/// idempotency tag in [`crate::pallet::OutboundConsensusRotationsClaimed`], not
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
	/// [`crate::withdrawal::WithdrawalProof`] uses for its source/dest proofs.
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
		// signature and matches it against the `EvmHost._epochs[set_id]`
		// slot, so only secp256k1/EVM signatures are meaningful here.
		// Reject the substrate variants up front to avoid burning the rest
		// of the verification pipeline on a claim that can never match.
		ensure!(matches!(signature, Signature::Evm { .. }), Error::<T>::OutboundSignatureNotEvm,);

		ensure!(
			!OutboundConsensusRotationsClaimed::<T>::contains_key(destination, set_id),
			Error::<T>::OutboundRotationAlreadyClaimed,
		);

		// EvmHost lookup. The per-chain host address is tracked by
		// `pallet-ismp-host-executive` in `EvmHosts`. Non-EVM destinations
		// are absent from that map and are rejected here, since the
		// attribution mechanism is EVM-specific.
		let evm_host = EvmHosts::<T>::get(destination).ok_or(Error::<T>::OutboundHostNotKnown)?;

		// 52-byte storage key the EVM state proof verifier expects:
		// `evm_host (20) || keccak256(set_id || EVM_HOST_EPOCHS_SLOT) (32)`.
		let slot_hash = evm_state_machine::utils::derive_unhashed_map_key::<<T as Config>::IsmpHost>(
			U256::from(set_id).to_big_endian().to_vec(),
			EVM_HOST_EPOCHS_SLOT,
		);
		let mut key = Vec::with_capacity(52);
		key.extend_from_slice(&evm_host.0);
		key.extend_from_slice(&slot_hash.0);

		let host = <T as Config>::IsmpHost::default();
		let state_machine = ismp::handlers::validate_state_machine(&host, state_proof.height)
			.map_err(|_| Error::<T>::OutboundDestinationStateNotKnown)?;
		let proof_results =
			Self::verify_withdrawal_proof(&*state_machine, &state_proof, vec![key.clone()])
				.map_err(|_| Error::<T>::OutboundDestinationStateNotKnown)?;
		let raw = proof_results
			.get(&key)
			.cloned()
			.flatten()
			.ok_or(Error::<T>::OutboundDeliveryNotProven)?;

		// `raw` is the trie-level value of `EvmHost._epochs[set_id]`;
		// `decode_epochs_slot_address` handles the RLP-encoded form the
		// Ethereum trie stores. Returns `None` for an unset / zero-address
		// slot, which we surface as `OutboundDeliveryNotProven` (logically
		// equivalent to "no delivery proven yet").
		let evm_address = Self::decode_epochs_slot_address(destination, &raw)
			.ok_or(Error::<T>::OutboundDeliveryNotProven)?;

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

	/// Decode the `address` value from `EvmHost._epochs[set_id]` as returned
	/// by `EvmStateMachine::verify_state_proof`. Standard EVM chains RLP-encode
	/// the value; Pharos stores it as a raw 32-byte ABI-padded word.
	pub fn decode_epochs_slot_address(
		state_id: ismp::host::StateMachine,
		raw: &[u8],
	) -> Option<Address> {
		use alloy_rlp::Decodable;
		if let Ok(addr) = Address::decode(&mut &*raw) {
			return if addr == Address::ZERO { None } else { Some(addr) };
		}
		if crate::is_pharos(&state_id) && raw.len() == 32 {
			let addr = Address::from_slice(&raw[12..]);
			return if addr == Address::ZERO { None } else { Some(addr) };
		}
		None
	}
}

/// Signed payload for [`OutboundConsensusDeliveryClaim`]. Replay protection
/// comes from the on-chain `(destination, set_id)` idempotency tag in
/// [`crate::pallet::OutboundConsensusRotationsClaimed`], not from a per-relayer
/// nonce — once a `(destination, set_id)` has been claimed it cannot be
/// claimed again, so a captured signature has no way to be reused.
pub fn outbound_consensus_delivery_message(
	set_id: u64,
	dest_chain: StateMachine,
	payee: [u8; 32],
) -> [u8; 32] {
	sp_io::hashing::keccak_256(&(set_id, dest_chain, payee).encode())
}
