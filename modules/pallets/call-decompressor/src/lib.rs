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
use alloc::{vec, vec::Vec};
use codec::DecodeLimit;
use frame_support::{
	dispatch::DispatchResult,
	traits::{Get, IsSubType},
};
pub use pallet::*;
use polkadot_sdk::*;
#[cfg(feature = "std")]
use ruzstd::io::Read;
#[cfg(not(feature = "std"))]
use ruzstd::io_nostd::Read;
use ruzstd::StreamingDecoder;
use sp_core::H256;
use sp_runtime::{
	traits::ValidateUnsigned,
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity, TransactionValidityError,
	},
	DispatchError,
};

const ONE_MB: u32 = 1_000_000;
/// This is the maximum nesting level required to decode
/// the supported ismp messages and pallet_ismp_relayer calls
/// All suported call types require a recursion depth of 2 except calls containing Ismp Get requests
/// Ismp Get requests have a nested vector of keys requiring an extra recursion depth
const MAX_EXTRINSIC_DECODE_DEPTH_LIMIT: u32 = 4;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec;
	use frame_support::{pallet_prelude::*, traits::IsSubType};
	use frame_system::pallet_prelude::*;
	use sp_core::H256;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config + pallet_ismp::Config + pallet_ismp_relayer::Config
	{
		/// Represents the maximum call size in megabytes(MB)
		type MaxCallSize: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Unsupported Call
		CallNotSupported,
		/// Error executing Call
		ErrorExecutingCall,
		/// Decompression Failed
		DecompressionFailed,
		/// Error Decoding Call
		ErrorDecodingCall,
		/// Call Size Out Of Bound
		CallSizeOutOfBound,
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
		/// This is for decompressing and executing compressed encoded runtime calls
		///
		///  The dispatch origin for this call must be an unsigned one.
		///
		/// - `compressed`: the compressed encoded runtime call represented in bytes.
		/// - `encoded_call_size`: this is the size of the not compressed(decompressed) encoded call
		/// in bytes.
		#[pallet::call_index(0)]
		#[pallet::weight({1_000_000})]
		pub fn decompress_call(
			origin: OriginFor<T>,
			compressed: Vec<u8>,
			encoded_call_size: u32,
		) -> DispatchResult {
			ensure_none(origin)?;
			ensure!(
				encoded_call_size < T::MaxCallSize::get() * ONE_MB,
				Error::<T>::CallSizeOutOfBound
			);
			let call_bytes = Self::decompress(compressed, encoded_call_size)?;
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
			let Call::decompress_call { compressed, encoded_call_size } = call else {
				return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
			};

			let decompressed = Self::decompress(compressed.clone(), encoded_call_size.clone())
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Call))?;

			let runtime_call = T::RuntimeCall::decode_with_depth_limit(
				MAX_EXTRINSIC_DECODE_DEPTH_LIMIT,
				&mut &decompressed[..],
			)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Call))?;

			let provides = if let Some(call) =
				IsSubType::<pallet_ismp::Call<T>>::is_sub_type(&runtime_call).cloned()
			{
				let _: Result<(), TransactionValidityError> = match call {
					pallet_ismp::Call::handle_unsigned { messages: _ } => Ok(()),
					_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
				};

				let ValidTransaction { provides, .. } =
					<pallet_ismp::Pallet<T> as ValidateUnsigned>::validate_unsigned(source, &call)
						.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Call))?;

				provides
			} else if let Some(call) =
				IsSubType::<pallet_ismp_relayer::Call<T>>::is_sub_type(&runtime_call).cloned()
			{
				let _: Result<(), TransactionValidityError> = match call.clone() {
					pallet_ismp_relayer::Call::accumulate_fees { withdrawal_proof: _ } => Ok(()),
					_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
				};

				let ValidTransaction { provides, .. } =
					<pallet_ismp_relayer::Pallet<T> as ValidateUnsigned>::validate_unsigned(
						source, &call,
					)
					.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Call))?;

				provides
			} else {
				return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
			};

			Ok(ValidTransaction {
				priority: 100,
				requires: vec![],
				provides,
				longevity: 25,
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
	/// This decompresses the encoded runtime call
	///
	/// - `compressed_bytes`: the compressed encoded runtime call represented in bytes.
	/// - `encoded_call_size`: this is the size of the not compressed(decompressed) encoded call
	/// in bytes.
	pub fn decompress(
		compressed_bytes: Vec<u8>,
		encoded_call_size: u32,
	) -> Result<Vec<u8>, DispatchError> {
		let mut decoder = StreamingDecoder::new(compressed_bytes.as_slice())
			.map_err(|_| Error::<T>::DecompressionFailed)?;

		let mut result = vec![0u8; encoded_call_size as usize];
		let _ = decoder.read(&mut result);
		Ok(result)
	}

	/// This decoded and executes the encoded runtime call which is represented in  bytes
	/// - `call_bytes`: the uncompressed encoded runtime call.
	pub fn decode_and_execute(call_bytes: Vec<u8>) -> DispatchResult {
		let runtime_call = <T as frame_system::Config>::RuntimeCall::decode_with_depth_limit(
			MAX_EXTRINSIC_DECODE_DEPTH_LIMIT,
			&mut &call_bytes[..],
		)
		.map_err(|_| Error::<T>::ErrorDecodingCall)?;

		if let Some(call) = IsSubType::<pallet_ismp::Call<T>>::is_sub_type(&runtime_call).cloned() {
			match call {
				pallet_ismp::Call::handle_unsigned { messages } =>
					<pallet_ismp::Pallet<T>>::execute(messages)
						.map_err(|_| Error::<T>::ErrorExecutingCall)?,
				_ => Err(Error::<T>::CallNotSupported)?,
			};
		} else if let Some(call) =
			IsSubType::<pallet_ismp_relayer::Call<T>>::is_sub_type(&runtime_call).cloned()
		{
			match call {
				pallet_ismp_relayer::Call::accumulate_fees { withdrawal_proof } =>
					<pallet_ismp_relayer::Pallet<T>>::accumulate_fees(
						frame_system::RawOrigin::None.into(),
						withdrawal_proof,
					)?,
				_ => Err(Error::<T>::CallNotSupported)?,
			};
		} else {
			return Err(Error::<T>::CallNotSupported)?;
		}

		Ok(())
	}
}
