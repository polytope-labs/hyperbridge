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

//! Fee accumulation.
//!
//! Relayers prove deliveries on the source/destination chains using a
//! [`WithdrawalProof`] and accumulate the earned fees into the
//! [`crate::pallet::Fees`] map. This module owns the proof verification
//! pipeline, the storage-key derivations for the two chain families (EVM and
//! Substrate), and the per-leaf result validation that ties source-side fee
//! metadata to destination-side delivery receipts.

use crate::{withdrawal::WithdrawalProof, Config, Error, Event, Fees, Pallet};
use alloc::{collections::BTreeMap, vec::Vec};
use alloy_primitives::Address;
use codec::Encode;
use crypto_utils::verification::Signature;
use frame_support::{dispatch::DispatchResult, ensure};
use ismp::{
	handlers::validate_state_machine,
	host::{IsmpHost, StateMachine},
	messaging::Proof,
};
use pallet_ismp::child_trie::RequestCommitments;
use polkadot_sdk::*;
use sp_core::{H256, U256};
use sp_runtime::DispatchError;

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::Hash: From<H256>,
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	T::Balance: Into<u128>,
{
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
			.filter(|req| match RequestCommitments::<T>::get(*req) {
				Some(leaf_meta) => !leaf_meta.claimed,
				// If request commitment does not exist in storage which should not be
				// possible, we skip it
				None => false,
			})
			.collect();
		ensure!(!withdrawal_proof.commitments.is_empty(), Error::<T>::MissingCommitments);
		let host = <T as Config>::IsmpHost::default();
		let source_sm = validate_state_machine(&host, withdrawal_proof.source_proof.height)
			.map_err(|_| Error::<T>::ProofValidationError)?;
		let dest_sm = validate_state_machine(&host, withdrawal_proof.dest_proof.height)
			.map_err(|_| Error::<T>::ProofValidationError)?;
		let source_keys = source_sm.commitment_state_trie_key(withdrawal_proof.commitments.clone());
		let dest_keys = dest_sm.receipts_state_trie_key(withdrawal_proof.commitments.clone());
		let state_machine = withdrawal_proof.source_proof.height.id.state_id;

		let source_result = Self::verify_withdrawal_proof(
			&*source_sm,
			&withdrawal_proof.source_proof,
			source_keys.clone(),
		)?;
		let dest_result = Self::verify_withdrawal_proof(
			&*dest_sm,
			&withdrawal_proof.dest_proof,
			dest_keys.clone(),
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

		for req in withdrawal_proof.commitments {
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
		}

		Self::deposit_event(Event::<T>::AccumulateFees {
			address: sp_runtime::BoundedVec::truncate_from(beneficiary_address),
			state_machine,
			amount: total_fee,
		});

		Ok(())
	}

	pub fn verify_withdrawal_proof(
		state_machine: &dyn ismp::consensus::StateMachineClient,
		proof: &Proof,
		keys: Vec<Vec<u8>>,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, DispatchError> {
		let host = <T as Config>::IsmpHost::default();
		let state = host
			.state_machine_commitment(proof.height)
			.map_err(|_| Error::<T>::ProofValidationError)?;
		let result = state_machine
			.verify_state_proof(&host, keys, state, proof)
			.map_err(|_| Error::<T>::ProofValidationError)?;

		Ok(result)
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
		for ((commitment, source_key), dest_key) in
			proof.commitments.clone().into_iter().zip(source_keys).zip(dest_keys)
		{
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
