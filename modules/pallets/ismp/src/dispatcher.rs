// Copyright (c) 2025 Polytope Labs.
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

//! Implementation for the low-level ISMP Dispatcher
use polkadot_sdk::*;

use crate::{
	child_trie::{RequestCommitments, RequestReceipts, ResponseCommitments},
	offchain::LeafIndexAndPos,
	Config, Pallet, RELAYER_FEE_ACCOUNT,
};
use alloc::{boxed::Box, format, vec::Vec};
use core::marker::PhantomData;
use frame_support::{
	traits::{fungible::Mutate, tokens::Preservation, UnixTime},
	weights::Weight,
};
use ismp::{
	dispatcher,
	dispatcher::{DispatchRequest, IsmpDispatcher},
	error::Error as IsmpError,
	events::Meta,
	host::IsmpHost,
	messaging::{hash_post_response, hash_request},
	module::IsmpModule,
	router::{GetRequest, IsmpRouter, PostRequest, PostResponse, Request, Response, Timeout},
};
use sp_core::H256;
use sp_runtime::traits::{AccountIdConversion, Zero};

/// Metadata about an outgoing request
#[derive(codec::Encode, codec::Decode, scale_info::TypeInfo, Clone)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[scale_info(skip_type_params(T))]
pub struct RequestMetadata<T: Config> {
	/// Information about where it's stored in the offchain db
	pub offchain: LeafIndexAndPos,
	/// Other metadata about the request
	pub fee: FeeMetadata<T>,
	/// Has fee been claimed?
	pub claimed: bool,
}

/// This is used for tracking user fee payments for requests
pub type FeeMetadata<T> =
	dispatcher::FeeMetadata<<T as frame_system::Config>::AccountId, <T as Config>::Balance>;

/// The low-level dispatcher. This can be used to dispatch requests while locking up a fee to be
/// paid to relayers for request delivery and execution.
///
/// If the dispatched request times-out, then pallet-ismp's inner subsystems will refund the
/// fees to the sponsor of the request.
impl<T> IsmpDispatcher for Pallet<T>
where
	T: Config,
{
	type Account = T::AccountId;
	type Balance = T::Balance;

	fn dispatch_request(
		&self,
		request: DispatchRequest,
		fee: FeeMetadata<T>,
	) -> Result<H256, anyhow::Error> {
		// collect payment for the request
		if fee.fee != Zero::zero() {
			T::Currency::transfer(
				&fee.payer,
				&RELAYER_FEE_ACCOUNT.into_account_truncating(),
				fee.fee,
				Preservation::Expendable,
			)
			.map_err(|err| IsmpError::Custom(format!("Error withdrawing request fees: {err:?}")))?;
		}

		let request = match request {
			DispatchRequest::Get(dispatch_get) => {
				let get = GetRequest {
					source: self.host_state_machine(),
					dest: dispatch_get.dest,
					nonce: self.next_nonce(),
					from: dispatch_get.from,
					keys: dispatch_get.keys,
					height: dispatch_get.height,
					context: dispatch_get.context,
					timeout_timestamp: if dispatch_get.timeout == 0 {
						0
					} else {
						<T::TimestampProvider as UnixTime>::now()
							.as_secs()
							.saturating_add(dispatch_get.timeout)
					},
				};
				Request::Get(get)
			},
			DispatchRequest::Post(dispatch_post) => {
				let post = PostRequest {
					source: self.host_state_machine(),
					dest: dispatch_post.dest,
					nonce: self.next_nonce(),
					from: dispatch_post.from,
					to: dispatch_post.to,
					timeout_timestamp: if dispatch_post.timeout == 0 {
						0
					} else {
						<T::TimestampProvider as UnixTime>::now()
							.as_secs()
							.saturating_add(dispatch_post.timeout)
					},
					body: dispatch_post.body,
				};
				Request::Post(post)
			},
		};

		let commitment = Pallet::<T>::dispatch_request(request, fee)?;

		Ok(commitment)
	}

	fn dispatch_response(
		&self,
		response: PostResponse,
		fee: FeeMetadata<T>,
	) -> Result<H256, anyhow::Error> {
		// collect payment for the response
		if fee.fee != Zero::zero() {
			T::Currency::transfer(
				&fee.payer,
				&RELAYER_FEE_ACCOUNT.into_account_truncating(),
				fee.fee,
				Preservation::Expendable,
			)
			.map_err(|err| IsmpError::Custom(format!("Error withdrawing request fees: {err:?}")))?;
		}

		let req_commitment = hash_request::<Pallet<T>>(&response.request());
		if !RequestReceipts::<T>::contains_key(req_commitment) {
			Err(IsmpError::UnknownRequest {
				meta: Meta {
					source: response.request().source_chain(),
					dest: response.request().dest_chain(),
					nonce: response.request().nonce(),
				},
			})?
		}

		let response = Response::Post(response);
		let commitment = Pallet::<T>::dispatch_response(response, fee)?;

		Ok(commitment)
	}
}

/// An [`IsmpRouter`] implementation that delegates to an inner module which always refunds
/// relayer fees on_timeout.
pub(crate) struct RefundingRouter<T> {
	/// Inner [`IsmpModule`]
	inner: Box<dyn IsmpRouter>,
	/// Phantom type for pinning generics
	_phantom: PhantomData<T>,
}

impl<T: Config> RefundingRouter<T> {
	/// Create an instance of a refunding router
	pub fn new(inner: Box<dyn IsmpRouter>) -> Self {
		Self { inner, _phantom: PhantomData }
	}
}

impl<T: Config> IsmpRouter for RefundingRouter<T> {
	fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
		let module = self.inner.module_for_id(id)?;

		Ok(Box::new(RefundingModule::<T>::new(module)))
	}
}

/// An implementation of [`IsmpModule`] that wraps an inner implementation and refunds any relayer
/// fees on_timeout. This allows the ISMP framework refund the relayer fees when requests time-out.
pub(crate) struct RefundingModule<T> {
	/// Inner [`IsmpModule`]
	inner: Box<dyn IsmpModule>,
	/// Phantom type for pinning generics
	_phantom: PhantomData<T>,
}

impl<T: Config> RefundingModule<T> {
	/// Create an instance of a refunding module
	pub fn new(inner: Box<dyn IsmpModule>) -> Self {
		Self { inner, _phantom: PhantomData }
	}
}

impl<T: Config> IsmpModule for RefundingModule<T> {
	fn on_accept(&self, request: PostRequest) -> Result<Weight, anyhow::Error> {
		self.inner.on_accept(request)
	}

	fn on_response(&self, response: Response) -> Result<Weight, anyhow::Error> {
		self.inner.on_response(response)
	}

	fn on_timeout(&self, timeout: Timeout) -> Result<Weight, anyhow::Error> {
		let result = self.inner.on_timeout(timeout.clone());

		// only refund if module returns Ok(())
		if result.is_ok() {
			let fee_metadata = match timeout {
				Timeout::Request(request) => {
					let commitment = hash_request::<Pallet<T>>(&request);
					RequestCommitments::<T>::get(commitment).map(|meta| meta.fee)
				},
				Timeout::Response(response) => {
					let commitment = hash_post_response::<Pallet<T>>(&response);
					ResponseCommitments::<T>::get(commitment).map(|meta| meta.fee)
				},
			};

			if let Some(fee) = fee_metadata {
				if fee.fee > Zero::zero() {
					T::Currency::transfer(
						&RELAYER_FEE_ACCOUNT.into_account_truncating(),
						&fee.payer,
						fee.fee,
						Preservation::Expendable,
					)
					.map_err(|err| {
						IsmpError::Custom(format!("Error withdrawing request fees: {err:?}"))
					})?;
				}
			}
		}

		result
	}
}
