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

use crate::withdrawal::{FeeMetadata, ResponseReceipt};
use alloc::{collections::BTreeMap, vec::Vec};
use alloy_primitives::Address;
use alloy_rlp::Decodable;
use ismp::{handlers::validate_state_machine, host::IsmpHost, messaging::Proof};
pub use pallet::*;
use pallet_ismp::host::Host;
use sp_runtime::DispatchError;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use ismp::host::StateMachine;

    use crate::withdrawal::{WithdrawalInputData, WithdrawalOutputData, WithdrawalProof};
    use codec::{Decode, Encode};
    use ismp::router::{DispatchPost, DispatchRequest, IsmpDispatcher};
    use pallet_ismp::dispatcher::Dispatcher;
    use sp_core::H256;
    use sp_runtime::traits::{IdentifyAccount, Verify};
    use sp_std::{prelude::*, vec};

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {
        /// A Signature can be verified with a specific `PublicKey`.
        /// The additional traits are boilerplate.
        type Signature: Verify<Signer = Self::PublicKey> + Encode + Decode + Parameter;

        /// A PublicKey can be converted into an `AccountId`. This is required by the
        /// `Signature` type.
        type PublicKey: IdentifyAccount<AccountId = Self::PublicKey> + Encode + Decode + Parameter;
    }

    /// double map of address to source chain, which holds the amount of the relayer address
    #[pallet::storage]
    #[pallet::getter(fn accumulating_fees)]
    pub type AccumulatingFees<T: Config> =
        StorageDoubleMap<_, Twox64Concat, Vec<u8>, Twox64Concat, StateMachine, u128, OptionQuery>;

    /// Latest nonce for each address when they withdraw
    #[pallet::storage]
    #[pallet::getter(fn nonce)]
    pub type Nonce<T: Config> = StorageMap<_, Identity, Vec<u8>, u64, OptionQuery>;

    #[pallet::error]
    pub enum Error<T> {
        /// State Proof Verifiction Failed
        StateProofVerificationFailed,
        /// Error Validating State Machine
        ErrorValidatingStateMachine,
        /// Error Fetching State Commitment
        ErrorFetchingStateMachineCommitment,
        /// Invalid Withdrawal Nonce
        InvalidWithdrawalNonce,
        /// Invalid Withdrawal Amount
        InvalidWithdrawalAmount,
        /// Signature Verification Failed
        SignatureVerificationFailed,
        /// Withdrawal Request Dispatch Failed
        WithdrawalRequestDispatchFailed,
        /// Cannot Decode Relayer Public Key
        CannotDecodeRelayerPublicKey,
        /// Cannot Decode Source Proof
        CannotDecodeSourceProof,
        /// Cannot Decode Fee Metadata
        CannotDecodeFeeMetadata,
        /// Cannot Verify Empty Commitments
        CannotVerifyEmptyCommitments,
        /// Invalid Request Commitment Proof
        InvalidRequestCommitmentProof,
        /// Invalid Response Commitment Proof
        InvalidResponseCommitmentProof,
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
            relayer_public_key: Vec<u8>,
            withdrawal_proof: WithdrawalProof,
            amount: u128,
        ) -> DispatchResult {
            ensure_none(origin)?;

            ensure!(
                withdrawal_proof.request_commitments.is_empty() &&
                    withdrawal_proof.response_commitments.is_empty(),
                Error::<T>::CannotVerifyEmptyCommitments
            );

            if !withdrawal_proof.request_commitments.is_empty() {
                let (address, fee_metadata) = Self::get_receipt_values(
                    withdrawal_proof.source_proof.clone(),
                    withdrawal_proof.dest_proof.clone(),
                    withdrawal_proof.to_request_commitments_bytes(),
                )?;

                if address.is_none() || fee_metadata.is_none() {
                    return Err(Error::<T>::InvalidRequestCommitmentProof.into());
                }
            }

            if !withdrawal_proof.response_commitments.is_empty() {
                let (address, response_receipt) = Self::get_response_values(
                    withdrawal_proof.source_proof.clone(),
                    withdrawal_proof.dest_proof.clone(),
                    withdrawal_proof.to_request_commitments_bytes(),
                )?;

                if address.is_none() || response_receipt.is_none() {
                    return Err(Error::<T>::InvalidResponseCommitmentProof.into());
                }
            }

            let state_machine = &withdrawal_proof.source_proof.height.id.state_id;

            let mut total_amount =
                AccumulatingFees::<T>::get(&relayer_public_key, state_machine).unwrap_or(0);
            total_amount = total_amount + amount;

            AccumulatingFees::<T>::insert(relayer_public_key, state_machine, total_amount);

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(2, 1))]
        pub fn withdraw_fees(
            origin: OriginFor<T>,
            withdrawal_data: WithdrawalInputData,
            signature_data: Vec<u8>,
            signature: T::Signature,
        ) -> DispatchResult {
            ensure_none(origin)?;

            let nonce = Nonce::<T>::get(&withdrawal_data.relayer_public_key).unwrap_or(0);

            if withdrawal_data.nonce < nonce {
                return Err(Error::<T>::InvalidWithdrawalNonce.into());
            }

            let amount = AccumulatingFees::<T>::get(
                &withdrawal_data.relayer_public_key,
                &withdrawal_data.source_chain,
            )
            .unwrap_or(0);
            if amount == 0 {
                return Err(Error::<T>::InvalidWithdrawalAmount.into());
            }

            if amount < withdrawal_data.amount {
                return Err(Error::<T>::InvalidWithdrawalAmount.into());
            }

            let signer = T::PublicKey::decode(&mut withdrawal_data.relayer_public_key.as_slice())
                .map_err(|_| Error::<T>::CannotDecodeRelayerPublicKey)?;
            if !signature.verify(signature_data.as_slice(), &signer) {
                return Err(Error::<T>::SignatureVerificationFailed.into());
            }

            let withdrawal_output_data = WithdrawalOutputData {
                beneficiary_address: withdrawal_data.beneficiary_address,
                amount: withdrawal_data.amount,
            };

            let post = DispatchPost {
                dest: withdrawal_data.source_chain,
                from: vec![],
                to: vec![],
                timeout_timestamp: 0,
                data: withdrawal_output_data.encode(),
                gas_limit: 0,
            };
            let dispatcher = Dispatcher::<T>::default();
            let dispatch_request = DispatchRequest::Post(post);
            dispatcher
                .dispatch_request(dispatch_request, [0u8; 32].into(), 0u32.into())
                .map_err(|_| Error::<T>::WithdrawalRequestDispatchFailed)?;

            Nonce::<T>::insert(&withdrawal_data.relayer_public_key, nonce + 1);

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
            .map_err(|_| Error::<T>::ErrorValidatingStateMachine)?;
        let state = ismp_host
            .state_machine_commitment(proof.height)
            .map_err(|_| Error::<T>::ErrorFetchingStateMachineCommitment)?;
        let result = state_machine
            .verify_state_proof(&ismp_host, keys, state, proof)
            .map_err(|_| Error::<T>::StateProofVerificationFailed)?;

        Ok(result)
    }

    pub fn get_receipt_values(
        source_proof: Proof,
        dest_proof: Proof,
        keys: Vec<Vec<u8>>,
    ) -> Result<(Option<Address>, Option<FeeMetadata>), DispatchError> {
        let mut fee_metadata = None;
        let source_proof_key_values = Self::verify_withdrawal_proof(&source_proof, keys.clone())?;

        for (_key, value) in source_proof_key_values.iter() {
            if let Some(metadata) =
                value.clone().and_then(|v| FeeMetadata::decode(&mut v.as_ref()).ok())
            {
                fee_metadata = Some(metadata);
                break;
            }
        }

        let mut request_receipt = None;
        let dest_proof_key_values = Self::verify_withdrawal_proof(&dest_proof, keys)?;
        for (_key, value) in dest_proof_key_values.iter() {
            if let Some(receipt) = value.clone().and_then(|v| Address::decode(&mut v.as_ref()).ok())
            {
                request_receipt = Some(receipt);
                break;
            }
        }

        Ok((request_receipt, fee_metadata))
    }

    pub fn get_response_values(
        source_proof: Proof,
        dest_proof: Proof,
        keys: Vec<Vec<u8>>,
    ) -> Result<(Option<Address>, Option<ResponseReceipt>), DispatchError> {
        let mut relayer_address = None;
        let source_proof_key_values = Self::verify_withdrawal_proof(&source_proof, keys.clone())?;

        for (_key, value) in source_proof_key_values.iter() {
            if let Some(address) = value.clone().and_then(|v| Address::decode(&mut v.as_ref()).ok())
            {
                relayer_address = Some(address);
                break;
            }
        }

        let mut response_receipt = None;
        let dest_proof_key_values = Self::verify_withdrawal_proof(&dest_proof, keys)?;
        for (_key, value) in dest_proof_key_values.iter() {
            if let Some(receipt) =
                value.clone().and_then(|v| ResponseReceipt::decode(&mut v.as_ref()).ok())
            {
                response_receipt = Some(receipt);
                break;
            }
        }

        Ok((relayer_address, response_receipt))
    }
}
