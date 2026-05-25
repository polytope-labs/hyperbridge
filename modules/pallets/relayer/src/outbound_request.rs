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

//! Outbound request delivery rewards.
//!
//! Relayers that deliver a hyperbridge-originated request (host-executive,
//! intents-coprocessor, token-governor, the relayer pallet's own withdrawal
//! path, etc.) to a destination earn the per-`module_id`
//! [`crate::pallet::OutboundRequestDeliveryReward`]. The on-chain attribution
//! lives in the destination's `RequestReceipts[commitment]` slot, written by
//! the destination's ISMP host the first time the request is delivered. This
//! module proves that slot against Hyperbridge's stored state commitment for
//! the destination and transfers the configured reward.
//!
//! Unlike [`crate::outbound_consensus`], this claim supports both EVM and
//! substrate destinations: the receipt key and relayer decoding branch on the
//! destination state machine type.

use crate::{
	BalanceOf, Config, Error, Event, ModuleIdBound, Nonce, OutboundRequestDeliveryReward,
	OutboundRequestsClaimed, Pallet,
};
use alloc::vec;
use codec::Encode;
use crypto_utils::verification::Signature;
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	traits::tokens::{fungible::Mutate, Preservation},
	BoundedVec,
};
use ismp::{
	host::{IsmpHost, StateMachine},
	messaging::{hash_request, Proof},
	router::{PostRequest, Request},
};
use pallet_ismp::child_trie::RequestCommitments;
use polkadot_sdk::*;
use sp_core::{Get, H256};
use sp_runtime::traits::AccountIdConversion;

/// Claim payload for [`Pallet::claim_outbound_request_delivery_reward`].
///
/// Carries the full [`PostRequest`] so the pallet can hash it on chain and
/// look up the reward by `request.from`. A module with zero reward is
/// treated as not on the allowlist and rejected before any state proof
/// verification runs.
///
/// Verification pipeline:
///
/// 1. Hash `request` to derive the commitment.
/// 2. Reject if `request.source` is not this hyperbridge instance.
/// 3. Reject if the commitment is not in `pallet_ismp::child_trie::RequestCommitments` (defence in
///    depth; the dispatcher already enforces source on insert).
/// 4. Reject if the commitment has already been claimed.
/// 5. Reject if `OutboundRequestDeliveryReward[request.from]` is zero (allowlist).
/// 6. State proof against Hyperbridge's stored commitment for the destination yields a value at
///    `RequestReceipts[commitment]`.
/// 7. The `signature` recovers the same address (EVM) or bytes (substrate) that the destination
///    recorded as the delivering relayer, signing [`outbound_request_delivery_message`] of `(nonce,
///    commitment, destination, payee)`.
///
/// Replay is guarded twice: the per-relayer `Nonce` folded into the signature, and the on-chain
/// `commitment` tag in [`crate::pallet::OutboundRequestsClaimed`].
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
pub struct OutboundRequestDeliveryClaim {
	/// The hyperbridge-originated request being claimed against. Hashed on
	/// chain to derive the commitment; `source` is verified against
	/// `IsmpHost::host_state_machine()`; `from` keys the reward map.
	pub request: PostRequest,
	/// State proof of the destination chain at the height the relayer is
	/// proving against. `state_proof.height.id.state_id` is the destination;
	/// Hyperbridge must already have a state commitment at `state_proof.height`.
	pub state_proof: Proof,
	/// Sr25519 public key on Hyperbridge that the reward is paid to.
	pub payee: [u8; 32],
	/// Signature over [`outbound_request_delivery_message`] of
	/// `(nonce, commitment, destination, payee)`. For EVM destinations the recovered
	/// secp256k1 address must equal the address proven in the receipt slot;
	/// for substrate destinations the recovered signer bytes must equal the
	/// relayer bytes proven in the receipt slot.
	pub signature: Signature,
}

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::Hash: From<H256>,
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	T::Balance: Into<u128>,
{
	/// Verify and pay out an outbound-request delivery claim.
	///
	/// See [`OutboundRequestDeliveryClaim`] for the verification pipeline.
	/// Ordering is deliberate: every cheap rejection runs before the
	/// state-proof verification, so non-allowlisted claims and replays are
	/// dropped without ever touching the trie verifier.
	pub fn process_outbound_request_delivery_claim(
		claim: OutboundRequestDeliveryClaim,
	) -> DispatchResult {
		let OutboundRequestDeliveryClaim { request, state_proof, payee, signature } = claim;
		let destination = state_proof.height.id.state_id;

		let commitment = hash_request::<<T as Config>::IsmpHost>(&Request::Post(request.clone()));

		let host = <T as Config>::IsmpHost::default();
		ensure!(
			request.source == host.host_state_machine(),
			Error::<T>::OutboundRequestSourceNotHyperbridge,
		);

		ensure!(
			RequestCommitments::<T>::get(commitment).is_some(),
			Error::<T>::OutboundRequestNotKnown,
		);

		ensure!(
			!OutboundRequestsClaimed::<T>::contains_key(commitment),
			Error::<T>::OutboundRequestAlreadyClaimed,
		);

		let module_id: BoundedVec<u8, ModuleIdBound> = request
			.from
			.clone()
			.try_into()
			.map_err(|_| Error::<T>::OutboundRequestModuleIdTooLong)?;
		let reward = OutboundRequestDeliveryReward::<T>::get(&module_id);
		ensure!(reward > BalanceOf::<T>::default(), Error::<T>::OutboundRequestNoRewardConfigured);

		ensure!(destination == request.dest, Error::<T>::MismatchedStateMachine);

		let state_machine = ismp::handlers::validate_state_machine(&host, state_proof.height)
			.map_err(|_| Error::<T>::OutboundDestinationStateNotKnown)?;
		let receipt_key = state_machine
			.receipts_state_trie_key(vec![commitment])
			.into_iter()
			.next()
			.ok_or(Error::<T>::OutboundRequestUnsupportedDestination)?;
		let proof_results =
			Self::verify_withdrawal_proof(&*state_machine, &state_proof, vec![receipt_key.clone()])
				.map_err(|_| Error::<T>::OutboundDestinationStateNotKnown)?;
		let raw = proof_results
			.get(&receipt_key)
			.cloned()
			.flatten()
			.ok_or(Error::<T>::OutboundDeliveryNotProven)?;

		let delivered_by = Self::decode_receipt_relayer(destination, &raw)?;

		let nonce = Nonce::<T>::get(&delivered_by, destination);
		let msg = outbound_request_delivery_message(nonce, commitment, destination, payee);
		let recovered = signature.verify(&msg, None).map_err(|_| Error::<T>::InvalidSignature)?;
		ensure!(recovered == delivered_by, Error::<T>::OutboundRequestSignerMismatch);

		let treasury: T::AccountId =
			<T as Config>::TreasuryPalletId::get().into_account_truncating();
		let payee_account: T::AccountId = payee.into();
		<<T as pallet_ismp::Config>::Currency as Mutate<T::AccountId>>::transfer(
			&treasury,
			&payee_account,
			reward,
			Preservation::Preserve,
		)
		.map_err(|_| Error::<T>::OutboundRequestRewardTransferFailed)?;

		OutboundRequestsClaimed::<T>::insert(commitment, ());

		Nonce::<T>::try_mutate(&delivered_by, destination, |value| {
			*value += 1;
			Ok::<(), ()>(())
		})
		.map_err(|_: ()| Error::<T>::ErrorCompletingCall)?;

		Self::deposit_event(Event::OutboundRequestDeliveryRewarded {
			commitment,
			state_machine: destination,
			module_id,
			relayer: payee_account,
			amount: reward,
		});

		Ok(())
	}
}

/// Signed payload for [`OutboundRequestDeliveryClaim`]. The per-relayer `nonce`
/// keeps each signature single use, the same way the withdrawal and fee flows do,
/// and [`crate::pallet::OutboundRequestsClaimed`] guards replay on chain.
pub fn outbound_request_delivery_message(
	nonce: u64,
	commitment: H256,
	dest_chain: StateMachine,
	payee: [u8; 32],
) -> [u8; 32] {
	sp_io::hashing::keccak_256(&(nonce, commitment, dest_chain, payee).encode())
}
