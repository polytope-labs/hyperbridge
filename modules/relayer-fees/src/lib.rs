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
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
#[cfg(test)]
mod test;
pub mod withdrawal;

use crate::withdrawal::{Key, Signature, WithdrawalInputData, WithdrawalParams, WithdrawalProof};
use alloc::{collections::BTreeMap, vec::Vec};
use alloy_primitives::Address;
use codec::Encode;
use ethabi::ethereum_types::H256;
use frame_support::{dispatch::DispatchResult, ensure};
use frame_system::pallet_prelude::OriginFor;
use ismp::{
    handlers::validate_state_machine,
    host::{IsmpHost, StateMachine},
    messaging::Proof,
    router::{DispatchPost, DispatchRequest, IsmpDispatcher},
};
use ismp_sync_committee::{
    presets::{
        REQUEST_COMMITMENTS_SLOT, REQUEST_RECEIPTS_SLOT, RESPONSE_COMMITMENTS_SLOT,
        RESPONSE_RECEIPTS_SLOT,
    },
    utils::{add_off_set_to_map_key, derive_unhashed_map_key},
};
pub use pallet::*;
use pallet_ismp::{dispatcher::Dispatcher, host::Host};
use sp_core::U256;
use sp_runtime::DispatchError;
use sp_std::prelude::*;

pub const MODULE_ID: [u8; 32] = [1; 32];

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::{OptionQuery, *};
    use frame_system::pallet_prelude::*;
    use ismp::host::StateMachine;

    use crate::withdrawal::{WithdrawalInputData, WithdrawalProof};
    use codec::Encode;
    use sp_core::H256;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {}

    /// double map of address to source chain, which holds the amount of the relayer address
    #[pallet::storage]
    #[pallet::getter(fn relayer_fees)]
    pub type RelayerFees<T: Config> =
        StorageDoubleMap<_, Twox64Concat, StateMachine, Twox64Concat, Vec<u8>, U256, ValueQuery>;

    /// Latest nonce for each address when they withdraw
    #[pallet::storage]
    #[pallet::getter(fn nonce)]
    pub type Nonce<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, u64, ValueQuery>;

    /// Relayer Manager Address on different chains
    #[pallet::storage]
    #[pallet::getter(fn manager)]
    pub type RelayerManager<T: Config> =
        StorageMap<_, Twox64Concat, StateMachine, Vec<u8>, OptionQuery>;

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
        InvalidAmount,
        /// Relayer Manager Address on Dest chain not set
        MissingMangerAddress,
        /// Failed to dispatch request
        DispatchFailed,
        /// Error
        ErrorCompletingCall,
        /// Missing commitments
        MissingCommitments,
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

        /// Set the relayer manager addresses for different state machines
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().writes(addresses.len() as u64))]
        #[pallet::call_index(2)]
        pub fn set_relayer_manager_addresses(
            origin: OriginFor<T>,
            addresses: BTreeMap<StateMachine, Vec<u8>>,
        ) -> DispatchResult {
            <T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;

            for (state_machine, address) in addresses {
                RelayerManager::<T>::insert(state_machine, address);
            }

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

            if let Err(_) = res {
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
                let nonce = Nonce::<T>::get(address.clone());
                let msg = message(nonce, withdrawal_data.dest_chain, withdrawal_data.amount);
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
                let nonce = Nonce::<T>::get(public_key.clone());
                let msg = message(nonce, withdrawal_data.dest_chain, withdrawal_data.amount);
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
                let nonce = Nonce::<T>::get(public_key.clone());
                let msg = message(nonce, withdrawal_data.dest_chain, withdrawal_data.amount);
                let signature = signature.as_slice().try_into().expect("Infallible");
                let pub_key = public_key.as_slice().try_into().expect("Infallible");
                if !sp_io::crypto::ed25519_verify(&signature, &msg, &pub_key) {
                    Err(Error::<T>::InvalidSignature)?
                }
                public_key
            },
        };
        let available_amount = RelayerFees::<T>::get(withdrawal_data.dest_chain, address.clone());

        if available_amount < withdrawal_data.amount {
            Err(Error::<T>::InvalidAmount)?
        }
        let dispatcher = Dispatcher::<T>::default();
        let relayer_manager_address = match withdrawal_data.dest_chain {
            StateMachine::Beefy(_) |
            StateMachine::Grandpa(_) |
            StateMachine::Kusama(_) |
            StateMachine::Polkadot(_) => MODULE_ID.to_vec(),
            _ => RelayerManager::<T>::get(withdrawal_data.dest_chain)
                .ok_or_else(|| Error::<T>::MissingMangerAddress)?,
        };
        Nonce::<T>::try_mutate(address.clone(), |value| {
            *value += 1;
            Ok::<(), ()>(())
        })
        .map_err(|_| Error::<T>::ErrorCompletingCall)?;
        let params = WithdrawalParams {
            beneficiary_address: address.clone(),
            amount: withdrawal_data.amount.into(),
        };

        let data = match withdrawal_data.dest_chain {
            StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc =>
                params.abi_encode(),
            _ => params.encode(),
        };

        let post = DispatchPost {
            dest: withdrawal_data.dest_chain,
            from: MODULE_ID.to_vec(),
            to: relayer_manager_address,
            timeout_timestamp: 0,
            data,
            gas_limit: withdrawal_data.gas_limit,
        };

        // Account is not useful in this case
        dispatcher
            .dispatch_request(DispatchRequest::Post(post), H256::default().0.into(), 0u32.into())
            .map_err(|_| Error::<T>::DispatchFailed)?;

        RelayerFees::<T>::insert(
            withdrawal_data.dest_chain,
            address,
            available_amount.saturating_sub(withdrawal_data.amount),
        );
        Ok(())
    }

    pub fn accumulate(withdrawal_proof: WithdrawalProof) -> DispatchResult {
        ensure!(!withdrawal_proof.commitments.is_empty(), Error::<T>::MissingCommitments);
        let source_keys = Self::get_commitment_keys(&withdrawal_proof);
        let dest_keys = Self::get_receipt_keys(&withdrawal_proof);
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
        let result = Self::validate_results(
            &withdrawal_proof,
            source_keys,
            dest_keys,
            source_result,
            dest_result,
        )?;
        for (address, fee) in result.into_iter() {
            let _ = RelayerFees::<T>::try_mutate(
                withdrawal_proof.source_proof.height.id.state_id,
                address,
                |inner| {
                    *inner += fee;
                    Ok::<(), ()>(())
                },
            );
        }

        Ok(())
    }
    pub fn verify_withdrawal_proof(
        proof: &Proof,
        keys: Vec<Vec<u8>>,
    ) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, DispatchError> {
        let ismp_host = Host::<T>::default();
        let state_machine = validate_state_machine(&ismp_host, proof.height)
            .map_err(|_| Error::<T>::ProofValidationError)?;
        let state = ismp_host
            .state_machine_commitment(proof.height)
            .map_err(|_| Error::<T>::ProofValidationError)?;
        let result = state_machine
            .verify_state_proof(&ismp_host, keys, state, proof)
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
                            derive_unhashed_map_key::<Host<T>>(
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
                    StateMachine::Beefy(_) =>
                        keys.push(pallet_ismp::RequestCommitments::<T>::hashed_key_for(commitment)),
                },
                Key::Response { response_commitment, .. } => {
                    match proof.source_proof.height.id.state_id {
                        StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc => {
                            keys.push(
                                derive_unhashed_map_key::<Host<T>>(
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
                            keys.push(pallet_ismp::ResponseCommitments::<T>::hashed_key_for(
                                response_commitment,
                            )),
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
                            derive_unhashed_map_key::<Host<T>>(
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
                    StateMachine::Polkadot(_) =>
                        keys.push(pallet_ismp::RequestReceipts::<T>::hashed_key_for(commitment)),
                },
                Key::Response { request_commitment, .. } => {
                    match proof.dest_proof.height.id.state_id {
                        StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc => {
                            keys.push(
                                derive_unhashed_map_key::<Host<T>>(
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
                        StateMachine::Polkadot(_) => keys.push(
                            pallet_ismp::ResponseReceipts::<T>::hashed_key_for(request_commitment),
                        ),
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
    ) -> Result<BTreeMap<Vec<u8>, U256>, Error<T>> {
        let mut result = BTreeMap::new();
        for ((key, source_key), dest_key) in
            proof.commitments.clone().into_iter().zip(source_keys).zip(dest_keys)
        {
            match key {
                Key::Request(_) => {
                    let encoded_metadata = source_result
                        .get(&source_key)
                        .cloned()
                        .flatten()
                        .ok_or_else(|| Error::<T>::ProofValidationError)?;
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
                                let fee: u128 = pallet_ismp::dispatcher::LeafMetadata::<T>::decode(
                                    &mut &*encoded_metadata,
                                )
                                .map_err(|_| Error::<T>::ProofValidationError)?
                                .meta
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
                },
                Key::Response { response_commitment, .. } => {
                    let encoded_metadata = source_result
                        .get(&source_key)
                        .cloned()
                        .flatten()
                        .ok_or_else(|| Error::<T>::ProofValidationError)?;
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
                                let fee: u128 = pallet_ismp::dispatcher::LeafMetadata::<T>::decode(
                                    &mut &*encoded_metadata,
                                )
                                .map_err(|_| Error::<T>::ProofValidationError)?
                                .meta
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
                },
            }
        }

        Ok(result)
    }
}

pub fn message(nonce: u64, dest_chain: StateMachine, amount: U256) -> [u8; 32] {
    sp_io::hashing::keccak_256(&(nonce, dest_chain, amount).encode())
}
