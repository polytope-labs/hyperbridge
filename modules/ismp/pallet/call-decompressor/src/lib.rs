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

use codec::{Decode, Encode};
use compression::prelude::{DecodeExt, ZlibDecoder};
use frame_support::{dispatch::DispatchResult, traits::IsSubType};
pub use pallet::*;
use sp_core::H256;
use sp_runtime::traits::ValidateUnsigned;
use sp_runtime::transaction_validity::{
    InvalidTransaction, TransactionSource, TransactionValidity, TransactionValidityError,
};
use sp_runtime::DispatchError;

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub enum CallIdentifier {
    IsmpHandleMessage,
    AccumulateRelayerFees,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct CompressedCall {
    /// The supported pallet call
    pub call_identifier: CallIdentifier,
    /// Compressed bytes representation of the call to decompress
    pub compressed_bytes: Vec<u8>,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::__private::sp_io;
    use frame_support::pallet_prelude::*;
    use frame_support::traits::IsSubType;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_ismp::Config + pallet_ismp_relayer::Config
    {
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Call Identifier not suported
        CallIdentifierNotSupported,
        /// Unsupported ISMP Call
        CallNotSupported,
        /// Invalid IsmpTransaction
        InvalidIsmpTransaction,
        /// Error executing Call
        ErrorExecutingCall,
        /// Error Decoding Call Identifier
        ErrorDecodingCallIdentifier,
        /// Compression Failed
        CompressionFailed,
        /// Error Decoding Call
        ErrorDecodingCall,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
        <T as frame_system::Config>::AccountId: From<[u8; 32]>,
        T::Balance: Into<u128>,
        <T as frame_system::Config>::RuntimeCall: IsSubType<pallet_ismp::Call<T>>,
        <T as frame_system::Config>::RuntimeCall: IsSubType<pallet_ismp_relayer::Call<T>>,
    {
        #[pallet::call_index(0)]
        #[pallet::weight({1_000_000})]
        pub fn decompress_call(origin: OriginFor<T>, compressed_bytes: Vec<u8>) -> DispatchResult {
            ensure_none(origin)?;
            let call_bytes = Self::decompress(compressed_bytes)?;
            Self::decode_and_execute(call_bytes)?;
            Ok(())
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
        <T as frame_system::Config>::AccountId: From<[u8; 32]>,
        T::Balance: Into<u128>,
        <T as frame_system::Config>::RuntimeCall: IsSubType<pallet_ismp::Call<T>>,
        <T as frame_system::Config>::RuntimeCall: IsSubType<pallet_ismp_relayer::Call<T>>,
    {
        type Call = Call<T>;

        // empty pre-dispatch so we don't modify storage
        fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
            Ok(())
        }

        fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match call {
                Call::decompress_call { compressed_bytes } => {
                    let decompressed = Self::decompress(compressed_bytes.clone())
                        .map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Call))?;

                    let runtime_call =
                        <T as frame_system::Config>::RuntimeCall::decode(&mut &decompressed[..])
                            .map_err(|_| {
                                TransactionValidityError::Invalid(InvalidTransaction::Call)
                            })?;

                    let ismp_call = Self::convert_to_ismp_call(runtime_call.clone());

                    if let Some(call) = ismp_call {
                        let _: Result<(), TransactionValidityError> = match call {
                            pallet_ismp::Call::handle { messages: _ } => Ok(()),
                            _ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
                        };

                        let _ = <pallet_ismp::Pallet<T> as ValidateUnsigned>::validate_unsigned(
                            source, &call,
                        )
                        .map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Call))?;
                    }

                    let ismp_relayer_call =
                        Self::convert_to_ismp_relayer_call(runtime_call.clone());

                    if let Some(call) = ismp_relayer_call {
                        let _: Result<(), TransactionValidityError> = match call.clone() {
                            pallet_ismp_relayer::Call::accumulate_fees { withdrawal_proof: _ } => {
                                Ok(())
                            },
                            _ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
                        };

                        let _ = <pallet_ismp_relayer::Pallet<T> as ValidateUnsigned>::validate_unsigned(
                            source,
                            &call
                        ).map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Call));
                    } else {
                        return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
                    }
                },
                _ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
            };

            let encoding = match call {
                Call::decompress_call { compressed_bytes } => compressed_bytes.encode(),
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
    <T as frame_system::Config>::RuntimeCall: IsSubType<pallet_ismp::Call<T>>,
    <T as frame_system::Config>::RuntimeCall: IsSubType<pallet_ismp_relayer::Call<T>>,
{
    pub fn decompress(compressed_bytes: Vec<u8>) -> Result<Vec<u8>, DispatchError> {
        let decompressed_call = compressed_bytes
            .iter()
            .cloned()
            .decode(&mut ZlibDecoder::new())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Error::<T>::CompressionFailed)?;

        Ok(decompressed_call)
    }

    pub fn decode_and_execute(call_bytes: Vec<u8>) -> DispatchResult {
        let runtime_call = <T as frame_system::Config>::RuntimeCall::decode(&mut &call_bytes[..])
            .map_err(|_| Error::<T>::ErrorDecodingCall)?;
        let ismp_call = Self::convert_to_ismp_call(runtime_call.clone());

        if let Some(call) = ismp_call {
            match call {
                pallet_ismp::Call::handle { messages } => {
                    <pallet_ismp::Pallet<T>>::handle(frame_system::RawOrigin::None.into(), messages)
                        .map_err(|_| Error::<T>::ErrorExecutingCall)?
                },
                _ => Err(Error::<T>::CallNotSupported)?,
            };
        }

        let ismp_relayer_call = Self::convert_to_ismp_relayer_call(runtime_call);
        if let Some(call) = ismp_relayer_call {
            match call {
                pallet_ismp_relayer::Call::accumulate_fees { withdrawal_proof } => {
                    <pallet_ismp_relayer::Pallet<T>>::accumulate_fees(
                        frame_system::RawOrigin::None.into(),
                        withdrawal_proof,
                    )?
                },
                _ => Err(Error::<T>::CallNotSupported)?,
            };
        } else {
            return Err(Error::<T>::CallNotSupported)?;
        }

        Ok(())
    }

    pub fn convert_to_ismp_call(runtime_call: T::RuntimeCall) -> Option<pallet_ismp::Call<T>> {
        IsSubType::<pallet_ismp::Call<T>>::is_sub_type(&runtime_call).cloned()
    }

    pub fn convert_to_ismp_relayer_call(
        runtime_call: T::RuntimeCall,
    ) -> Option<pallet_ismp_relayer::Call<T>> {
        IsSubType::<pallet_ismp_relayer::Call<T>>::is_sub_type(&runtime_call).cloned()
    }
}
