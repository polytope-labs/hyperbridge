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

use crate::withdrawal::{Key, Signature, WithdrawalInputData, WithdrawalParams, WithdrawalProof};
use alloc::{collections::BTreeMap, vec::Vec};
use alloy_primitives::Address;
use codec::Encode;
use ethabi::ethereum_types::H256;
use evm_common::{
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
use pallet_ismp_host_executive::{HostParam, HostParams};
use sp_core::U256;
use sp_runtime::{AccountId32, DispatchError};
use sp_std::prelude::*;

pub const MODULE_ID: &'static [u8] = b"ISMP-RLYR";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::host::StateMachine;

	use crate::withdrawal::{WithdrawalInputData, WithdrawalProof};
	use codec::Encode;
	use sp_core::{Get, H256};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_ismp::Config + pallet_ismp_host_executive::Config
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost + IsmpDispatcher<Account = Self::AccountId, Balance = Self::Balance>;
	}

	/// double map of address to source chain, which holds the amount of the relayer address
	#[pallet::storage]
	#[pallet::getter(fn relayer_fees)]
	pub type Fees<T: Config> =
		StorageDoubleMap<_, Twox64Concat, StateMachine, Twox64Concat, Vec<u8>, U256, ValueQuery>;

	/// Latest nonce for each address and the state machine they want to withdraw from
	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	pub type Nonce<T: Config> =
		StorageDoubleMap<_, Twox64Concat, Vec<u8>, Twox64Concat, StateMachine, u64, ValueQuery>;

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
	pub type MinimumWithdrawalAmount<T: Config> = StorageValue<_, U256, ValueQuery, MinWithdrawal>;

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

		/// Sets the minimum withdrawal amount in dollars
		#[pallet::call_index(2)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(0, 1))]
		pub fn set_minimum_withdrawal(origin: OriginFor<T>, amount: u128) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			MinimumWithdrawalAmount::<T>::put(U256::from(amount * 1_000_000_000_000_000_000));
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
		let address = match withdrawal_data.signature.clone() {
			Signature::Ethereum { address, signature } => {
				if signature.len() != 65 {
					Err(Error::<T>::InvalidSignature)?
				}
				let nonce = Nonce::<T>::get(address.clone(), withdrawal_data.dest_chain);
				let msg = message(nonce, withdrawal_data.dest_chain);
				let mut sig = [0u8; 65];
				sig.copy_from_slice(&signature);
				let pub_key = sp_io::crypto::secp256k1_ecdsa_recover(&sig, &msg)
					.map_err(|_| Error::<T>::InvalidSignature)?;
				let signer = sp_io::hashing::keccak_256(&pub_key[..])[12..].to_vec();
				if signer != address {
					Err(Error::<T>::InvalidPublicKey)?
				}
				address
			},
			Signature::Sr25519 { public_key, signature } => {
				if signature.len() != 64 {
					Err(Error::<T>::InvalidSignature)?
				}

				if public_key.len() != 32 {
					Err(Error::<T>::InvalidPublicKey)?
				}
				let nonce = Nonce::<T>::get(public_key.clone(), withdrawal_data.dest_chain);
				let msg = message(nonce, withdrawal_data.dest_chain);
				let signature = signature.as_slice().try_into().expect("Infallible");
				let pub_key = public_key.as_slice().try_into().expect("Infallible");
				if !sp_io::crypto::sr25519_verify(&signature, &msg, &pub_key) {
					Err(Error::<T>::InvalidSignature)?
				}
				public_key
			},
			Signature::Ed25519 { public_key, signature } => {
				if signature.len() != 64 {
					Err(Error::<T>::InvalidSignature)?
				}

				if public_key.len() != 32 {
					Err(Error::<T>::InvalidPublicKey)?
				}
				let nonce = Nonce::<T>::get(public_key.clone(), withdrawal_data.dest_chain);
				let msg = message(nonce, withdrawal_data.dest_chain);
				let signature = signature.as_slice().try_into().expect("Infallible");
				let pub_key = public_key.as_slice().try_into().expect("Infallible");
				if !sp_io::crypto::ed25519_verify(&signature, &msg, &pub_key) {
					Err(Error::<T>::InvalidSignature)?
				}
				public_key
			},
		};
		let available_amount = Fees::<T>::get(withdrawal_data.dest_chain, address.clone());

		if available_amount < Self::min_withdrawal_amount() {
			Err(Error::<T>::NotEnoughBalance)?
		}

		let dispatcher = <T as Config>::IsmpHost::default();
		let relayer_manager_address = match withdrawal_data.dest_chain {
			StateMachine::Beefy(_) |
			StateMachine::Grandpa(_) |
			StateMachine::Kusama(_) |
			StateMachine::Polkadot(_) => PALLET_HYPERBRIDGE.0.to_vec(),
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
			StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc =>
				params.abi_encode(),
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
			address: sp_runtime::BoundedVec::truncate_from(address),
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
			StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc => {
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
		let mut total_fee = hashbrown::HashMap::<Vec<u8>, U256>::new();
		for (address, fee) in result.clone().into_iter() {
			let _ = Fees::<T>::try_mutate(state_machine, address.clone(), |inner| {
				*inner += fee;
				let inner_fee = total_fee.entry(address).or_insert(U256::zero());
				*inner_fee += fee;
				Ok::<(), ()>(())
			});
		}

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

		for address in result.keys().collect::<hashbrown::HashSet<_>>().into_iter() {
			Self::deposit_event(Event::<T>::AccumulateFees {
				address: sp_runtime::BoundedVec::truncate_from(address.to_vec()),
				state_machine,
				amount: total_fee.remove(address).unwrap_or_default(),
			});
		}

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
					StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc => {
						keys.push(
							derive_unhashed_map_key::<<T as Config>::IsmpHost>(
								commitment.0.to_vec(),
								REQUEST_COMMITMENTS_SLOT,
							)
							.0
							.to_vec(),
						);
					},
					StateMachine::Polkadot(_) |
					StateMachine::Kusama(_) |
					StateMachine::Grandpa(_) |
					StateMachine::Beefy(_) => keys.push(RequestCommitments::<T>::storage_key(*commitment)),
				},
				Key::Response { response_commitment, .. } => {
					match proof.source_proof.height.id.state_id {
						StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc => {
							keys.push(
								derive_unhashed_map_key::<<T as Config>::IsmpHost>(
									response_commitment.0.to_vec(),
									RESPONSE_COMMITMENTS_SLOT,
								)
								.0
								.to_vec(),
							);
						},
						StateMachine::Polkadot(_) |
						StateMachine::Kusama(_) |
						StateMachine::Grandpa(_) |
						StateMachine::Beefy(_) =>
							keys.push(ResponseCommitments::<T>::storage_key(*response_commitment)),
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
					StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc => {
						keys.push(
							derive_unhashed_map_key::<<T as Config>::IsmpHost>(
								commitment.0.to_vec(),
								REQUEST_RECEIPTS_SLOT,
							)
							.0
							.to_vec(),
						);
					},
					StateMachine::Beefy(_) |
					StateMachine::Grandpa(_) |
					StateMachine::Kusama(_) |
					StateMachine::Polkadot(_) => keys.push(
						pallet_ismp::child_trie::RequestReceipts::<T>::storage_key(*commitment),
					),
				},
				Key::Response { request_commitment, .. } => {
					match proof.dest_proof.height.id.state_id {
						StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc => {
							keys.push(
								derive_unhashed_map_key::<<T as Config>::IsmpHost>(
									request_commitment.0.to_vec(),
									RESPONSE_RECEIPTS_SLOT,
								)
								.0
								.to_vec(),
							);
						},
						StateMachine::Beefy(_) |
						StateMachine::Grandpa(_) |
						StateMachine::Kusama(_) |
						StateMachine::Polkadot(_) =>
							keys.push(pallet_ismp::child_trie::ResponseReceipts::<T>::storage_key(
								*request_commitment,
							)),
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

					let fee = {
						match proof.source_proof.height.id.state_id {
							StateMachine::Ethereum(_) |
							StateMachine::Polygon |
							StateMachine::Bsc => {
								use alloy_rlp::Decodable;
								let fee = alloy_primitives::U256::decode(&mut &*encoded_metadata)
									.map_err(|_| Error::<T>::ProofValidationError)?;
								U256::from_big_endian(&fee.to_be_bytes::<32>())
							},
							StateMachine::Beefy(_) |
							StateMachine::Grandpa(_) |
							StateMachine::Kusama(_) |
							StateMachine::Polkadot(_) => {
								use codec::Decode;
								let fee: u128 =
									pallet_ismp::dispatcher::RequestMetadata::<T>::decode(
										&mut &*encoded_metadata,
									)
									.map_err(|_| Error::<T>::ProofValidationError)?
									.fee
									.fee
									.into();
								U256::from(fee)
							},
						}
					};
					let encoded_receipt = dest_result
						.get(&dest_key)
						.cloned()
						.flatten()
						.ok_or_else(|| Error::<T>::ProofValidationError)?;
					let address = {
						match proof.dest_proof.height.id.state_id {
							StateMachine::Ethereum(_) |
							StateMachine::Polygon |
							StateMachine::Bsc => {
								use alloy_rlp::Decodable;
								Address::decode(&mut &*encoded_receipt)
									.map_err(|_| Error::<T>::ProofValidationError)?
									.0
									.to_vec()
							},
							StateMachine::Beefy(_) |
							StateMachine::Grandpa(_) |
							StateMachine::Kusama(_) |
							StateMachine::Polkadot(_) => {
								use codec::Decode;
								<Vec<u8>>::decode(&mut &*encoded_receipt)
									.map_err(|_| Error::<T>::ProofValidationError)?
							},
						}
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
					let fee = {
						match proof.source_proof.height.id.state_id {
							StateMachine::Ethereum(_) |
							StateMachine::Polygon |
							StateMachine::Bsc => {
								use alloy_rlp::Decodable;
								let fee = alloy_primitives::U256::decode(&mut &*encoded_metadata)
									.map_err(|_| Error::<T>::ProofValidationError)?;
								U256::from_big_endian(&fee.to_be_bytes::<32>())
							},
							StateMachine::Beefy(_) |
							StateMachine::Grandpa(_) |
							StateMachine::Kusama(_) |
							StateMachine::Polkadot(_) => {
								use codec::Decode;
								let fee: u128 =
									pallet_ismp::dispatcher::RequestMetadata::<T>::decode(
										&mut &*encoded_metadata,
									)
									.map_err(|_| Error::<T>::ProofValidationError)?
									.fee
									.fee
									.into();
								U256::from(fee)
							},
						}
					};
					let encoded_receipt = dest_result
						.get(&dest_key)
						.cloned()
						.flatten()
						.ok_or_else(|| Error::<T>::ProofValidationError)?;
					let (relayer, res) = {
						match proof.dest_proof.height.id.state_id {
							StateMachine::Ethereum(_) |
							StateMachine::Polygon |
							StateMachine::Bsc => {
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
							StateMachine::Beefy(_) |
							StateMachine::Grandpa(_) |
							StateMachine::Kusama(_) |
							StateMachine::Polkadot(_) => {
								use codec::Decode;
								let receipt =
									pallet_ismp::ResponseReceipt::decode(&mut &*encoded_receipt)
										.map_err(|_| Error::<T>::ProofValidationError)?;
								(receipt.relayer, receipt.response.0)
							},
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

pub fn message(nonce: u64, dest_chain: StateMachine) -> [u8; 32] {
	sp_io::hashing::keccak_256(&(nonce, dest_chain).encode())
}
