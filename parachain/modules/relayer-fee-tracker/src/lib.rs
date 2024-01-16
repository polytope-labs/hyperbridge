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

extern crate alloc;
pub mod withdrawal;

use crate::withdrawal::{FeeMetadata, Key, ResponseReceipt, WithdrawalProof};
use alloc::{collections::BTreeMap, vec::Vec};
use alloy_primitives::Address;
use ismp::{
    handlers::validate_state_machine,
    host::{IsmpHost, StateMachine},
    messaging::Proof,
};
use ismp_sync_committee::{
    presets::{
        REQUEST_COMMITMENTS_SLOT, REQUEST_RECEIPTS_SLOT, RESPONSE_COMMITMENTS_SLOT,
        RESPONSE_RECEIPTS_SLOT,
    },
    utils::derive_map_key,
};
pub use pallet::*;
use pallet_ismp::host::Host;
use sp_core::U256;
use sp_runtime::DispatchError;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use ismp::host::StateMachine;

    use crate::withdrawal::{WithdrawalInputData, WithdrawalProof};
    use codec::{Decode, Encode};
    use ismp::router::{DispatchPost, DispatchRequest, IsmpDispatcher};
    use pallet_ismp::dispatcher::Dispatcher;
    use sp_core::{H256, U256};
    use sp_runtime::traits::{IdentifyAccount, Verify};
    use sp_std::{prelude::*, vec};

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {}

    /// double map of address to source chain, which holds the amount of the relayer address
    #[pallet::storage]
    #[pallet::getter(fn accumulating_fees)]
    pub type RelayerFees<T: Config> =
        StorageDoubleMap<_, Twox64Concat, Vec<u8>, Twox64Concat, StateMachine, T::Balance, OptionQuery>;

    /// Latest nonce for each address when they withdraw
    #[pallet::storage]
    #[pallet::getter(fn nonce)]
    pub type Nonce<T: Config> = StorageMap<_, Identity, Vec<u8>, u64, ValueQuery>;

    #[pallet::error]
    pub enum Error<T> {
        /// Withdrawal Proof Validation Error
        ProofValidationError,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
        <T as frame_system::Config>::AccountId: From<[u8; 32]>,
    {
        #[pallet::call_index(0)]
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
        pub fn accumulate_fees(
            origin: OriginFor<T>,
            withdrawal_proof: WithdrawalProof,
        ) -> DispatchResult {
            ensure_none(origin)?;

            ensure!(withdrawal_proof.commitments.is_empty(), Error::<T>::ProofValidationError);

            let source_keys = Self::get_commitment_keys(&withdrawal_proof);
            let dest_keys = Self::get_receipt_keys(&withdrawal_proof);

            let source_result =
                Self::verify_withdrawal_proof(&withdrawal_proof.source_proof, source_keys.clone())?;
            let dest_result =
                Self::verify_withdrawal_proof(&withdrawal_proof.dest_proof, dest_keys.clone())?;

            let result = Self::validate_results(
                &withdrawal_proof,
                source_keys,
                dest_keys,
                source_result,
                dest_result,
            )?;
            for (address, fee) in result.into_iter() {
                RelayerFees::<T>::try_mutate(
                    address,
                    withdrawal_proof.source_proof.height.id.state_id,
                    |inner| {
                        *inner = Some(inner.clone().unwrap_or(U256::zero()) + fee);
                        Ok(())
                    },
                );
            }

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(2, 1))]
        pub fn withdraw_fees(
            origin: OriginFor<T>,
            withdrawal_data: WithdrawalInputData<T::Balance>,
        ) -> DispatchResult {
            ensure_none(origin)?;

            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
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
                            derive_map_key::<Host<T>>(
                                commitment.0.to_vec(),
                                REQUEST_COMMITMENTS_SLOT,
                            )
                            .0
                            .to_vec(),
                        );
                    },
                    _ =>
                        keys.push(pallet_ismp::RequestCommitments::<T>::hashed_key_for(commitment)),
                },
                Key::Response { response_commitment, .. } => {
                    match proof.source_proof.height.id.state_id {
                        StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc => {
                            keys.push(
                                derive_map_key::<Host<T>>(
                                    response_commitment.0.to_vec(),
                                    RESPONSE_COMMITMENTS_SLOT,
                                )
                                .0
                                .to_vec(),
                            );
                        },
                        _ => keys.push(pallet_ismp::ResponseCommitments::<T>::hashed_key_for(
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
                            derive_map_key::<Host<T>>(commitment.0.to_vec(), REQUEST_RECEIPTS_SLOT)
                                .0
                                .to_vec(),
                        );
                    },
                    _ =>
                        keys.push(pallet_ismp::RequestCommitments::<T>::hashed_key_for(commitment)),
                },
                Key::Response { request_commitment, .. } => {
                    match proof.dest_proof.height.id.state_id {
                        StateMachine::Ethereum(_) | StateMachine::Polygon | StateMachine::Bsc => {
                            keys.push(
                                derive_map_key::<Host<T>>(
                                    request_commitment.0.to_vec(),
                                    RESPONSE_RECEIPTS_SLOT,
                                )
                                .0
                                .to_vec(),
                            );
                        },
                        _ => keys.push(pallet_ismp::ResponseCommitments::<T>::hashed_key_for(
                            request_commitment,
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
    ) -> Result<BTreeMap<Vec<u8>, T::Balance>, Error<T>> {
        let mut result = BTreeMap::new();
        for ((key, source_key), dest_key) in
            proof.commitments.into_iter().zip(source_keys).zip(dest_keys)
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
                                let fee = FeeMetadata::decode(&mut &*encoded_metadata)
                                    .map_err(|_| Error::<T>::ProofValidationError)?
                                    .fee;
                                U256::from_big_endian(&fee.to_be_bytes()).low_u32().into()
                            },
                            _ => {
                                use codec::Decode;
                                pallet_ismp::dispatcher::FeeMetadata::<T>::decode(
                                    &mut &*encoded_metadata,
                                )
                                .map_err(|_| Error::<T>::ProofValidationError)?
                                .fee
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
                            _ => {
                                use codec::Decode;
                                <Vec<u8>>::decode(&mut &*encoded_receipt)
                                    .map_err(|_| Error::<T>::ProofValidationError)?
                            },
                        }
                    };
                    let entry = result.entry(address).or_insert(0u32.into());
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
                                let fee = FeeMetadata::decode(&mut &*encoded_metadata)
                                    .map_err(|_| Error::<T>::ProofValidationError)?
                                    .fee;
                                U256::from_big_endian(&fee.to_be_bytes()).low_u32().into()
                            },
                            _ => {
                                use codec::Decode;
                                pallet_ismp::dispatcher::FeeMetadata::<T>::decode(
                                    &mut &*encoded_metadata,
                                )
                                .map_err(|_| Error::<T>::ProofValidationError)?
                                .fee
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
                                let receipt = ResponseReceipt::decode(&mut &*encoded_receipt)
                                    .map_err(|_| Error::<T>::ProofValidationError)?;
                                (receipt.relayer.0.to_vec(), receipt.response_commitment.0)
                            },
                            _ => {
                                use codec::Decode;
                                let receipt =
                                    pallet_ismp::ResponseReciept::decode(&mut &*encoded_receipt)
                                        .map_err(|_| Error::<T>::ProofValidationError)?;
                                (receipt.relayer, receipt.response.0);
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
