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
use frame_support::{dispatch::DispatchResult, ensure};
use frame_system::pallet_prelude::OriginFor;
use ismp::{
	dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
	handlers::validate_state_machine,
	host::{IsmpHost, StateMachine},
	messaging::Proof,
};
pub use pallet::*;
use pallet_hyperbridge::{Message, WithdrawalRequest, PALLET_HYPERBRIDGE};
use pallet_ismp::child_trie::{RequestCommitments, ResponseCommitments};
use pallet_ismp_host_executive::{withdrawal::*, HostParam, HostParams};
use polkadot_sdk::*;
use sp_core::{Get, H256, U256};
use sp_runtime::{AccountId32, DispatchError};

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
		/// Fee accumulation proof contains no address
		IncompleteProof,
		/// Signature Decoding Error
		SignatureDecodingError,
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
			/// destination state machine
			state_machine: StateMachine,
			/// Amount withdrawn
			amount: U256,
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
				_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
			};

			if let Err(err) = res {
				log::error!(target: "ismp", "Pallet Relayer Fees error {err:?}");
				Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?
			}

			let encoding = match call {
				Call::accumulate_fees { withdrawal_proof } => withdrawal_proof.encode(),
				Call::withdraw_fees { withdrawal_data } => withdrawal_data.encode(),
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
	pub fn withdraw(withdrawal_data: WithdrawalInputData) -> DispatchResult {
		let address = match &withdrawal_data.signature {
			Signature::Evm { address, .. } => {
				let nonce = Nonce::<T>::get(address.clone(), withdrawal_data.dest_chain);
				let msg = message(nonce, withdrawal_data.dest_chain);
				let eth_address = withdrawal_data
					.signature
					.verify(&msg, None)
					.map_err(|_| Error::<T>::InvalidSignature)?;
				if &eth_address != address {
					Err(Error::<T>::InvalidPublicKey)?
				}
				address
			},
			Signature::Sr25519 { public_key, .. } => {
				let nonce = Nonce::<T>::get(public_key.clone(), withdrawal_data.dest_chain);
				let msg = message(nonce, withdrawal_data.dest_chain);
				// Verify signature with public key provided in signature enum
				withdrawal_data
					.signature
					.verify(&msg, None)
					.map_err(|_| Error::<T>::InvalidSignature)?;
				public_key
			},
			Signature::Ed25519 { public_key, .. } => {
				let nonce = Nonce::<T>::get(public_key.clone(), withdrawal_data.dest_chain);
				let msg = message(nonce, withdrawal_data.dest_chain);
				// Verify signature with public key provided in signature enum
				withdrawal_data
					.signature
					.verify(&msg, None)
					.map_err(|_| Error::<T>::InvalidSignature)?;
				public_key
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
		let relayer_manager_address = match withdrawal_data.dest_chain {
			s if s.is_substrate() => PALLET_HYPERBRIDGE.0.to_vec(),
			_ => {
				let HostParam::EvmHostParam(params) =
					HostParams::<T>::get(withdrawal_data.dest_chain)
						.ok_or_else(|| Error::<T>::MissingMangerAddress)?
				else {
					Err(Error::<T>::MismatchedStateMachine)?
				};

				params.host_manager.0.to_vec()
			},
		};
		Nonce::<T>::try_mutate(address.clone(), withdrawal_data.dest_chain, |value| {
			*value += 1;
			Ok::<(), ()>(())
		})
		.map_err(|_| Error::<T>::ErrorCompletingCall)?;
		let params = WithdrawalParams {
			beneficiary_address: address.clone(),
			amount: available_amount.into(),
			native: false,
		};

		let data = match withdrawal_data.dest_chain {
			s if s.is_evm() => params.abi_encode(),
			_ => Message::WithdrawRelayerFees(WithdrawalRequest {
				amount: params.amount.low_u128(),
				account: AccountId32::try_from(&address[..])
					.map_err(|_| Error::<T>::InvalidPublicKey)?,
			})
			.encode(),
		};

		let post = DispatchPost {
			dest: withdrawal_data.dest_chain,
			from: MODULE_ID.to_vec(),
			to: relayer_manager_address,
			timeout: 0,
			body: data,
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
			state_machine: withdrawal_data.dest_chain,
			amount: available_amount,
		});

		Ok(())
	}

	pub fn accumulate(mut withdrawal_proof: WithdrawalProof) -> DispatchResult {
		// Filter out duplicate commitments
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

		if result.is_empty() {
			Err(Error::<T>::IncompleteProof)?
		}

		let mut total_fee = U256::zero();
		// We expect the relayer used the same address for all deliveries in this batch
		// That's the only behaviour supported by tesseract relayer
		let mut delivery_address = Default::default();
		for (address, fee) in result.clone().into_iter() {
			delivery_address = address;
			total_fee += fee;
		}

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

pub fn message(nonce: u64, dest_chain: StateMachine) -> [u8; 32] {
	sp_io::hashing::keccak_256(&(nonce, dest_chain).encode())
}
