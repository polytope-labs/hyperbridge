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

use crate::{offchain::LeafIndexAndPos, Config, Event, Pallet, RELAYER_FEE_ACCOUNT};
use alloc::{boxed::Box, format, vec::Vec};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use frame_support::{
	traits::{fungible::Mutate, tokens::Preservation, Get, UnixTime},
	weights::Weight,
};
use ismp::{
	dispatcher,
	dispatcher::{DispatchRequest, IsmpDispatcher},
	error::Error as IsmpError,
	host::IsmpHost,
	module::IsmpModule,
	router::{GetRequest, GetResponse, IsmpRouter, PostRequest, Request},
};
use sp_core::H256;
use sp_runtime::traits::{AccountIdConversion, Zero};

/// [`IsmpModule`] module identifier for incoming withdrawal requests from the
/// hyperbridge coprocessor. The router intercepts this id and routes the
/// payload to the built-in withdrawal handler — there is no separate pallet.
pub const HYPERBRIDGE_MODULE_ID: &[u8] = b"HYPR-FEE";

/// A request to withdraw some funds owed to a relayer by the protocol.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub struct WithdrawalRequest<Account, Amount> {
	/// The amount to be withdrawn
	pub amount: Amount,
	/// The withdrawal beneficiary
	pub account: Account,
}

/// Cross-chain messages this module accepts. Only messages from the configured
/// coprocessor are honoured. The SCALE encoding (including the `#[codec(index =
/// 2)]` discriminator) is preserved from the original standalone withdrawal
/// pallet so on-the-wire payloads continue to decode.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub enum Message<Account, Balance> {
	/// Withdraw the fees owed to a relayer
	#[codec(index = 0)]
	WithdrawRelayerFees(WithdrawalRequest<Account, Balance>),
}

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
}

/// Router installed by the pallet's [`IsmpHost`]. Intercepts the well known
/// protocol withdrawal id and routes everything else through
/// [`RefundingModule`] so that timed out requests return the escrowed fee to
/// the original payer.
pub(crate) struct RefundingRouter<T> {
	inner: Box<dyn IsmpRouter>,
	_phantom: PhantomData<T>,
}

impl<T: Config> RefundingRouter<T> {
	pub fn new(inner: Box<dyn IsmpRouter>) -> Self {
		Self { inner, _phantom: PhantomData }
	}
}

impl<T: Config> IsmpRouter for RefundingRouter<T> {
	fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
		// Intercept the well-known module id for protocol withdrawals. The
		// payload is decoded as [`Message::WithdrawRelayerFees`] and the
		// amount is transferred from `RELAYER_FEE_ACCOUNT` to the named
		// account. Only messages originating from the configured coprocessor
		// are accepted.
		if id.as_slice() == HYPERBRIDGE_MODULE_ID {
			return Ok(Box::new(HyperbridgeWithdrawalModule::<T>::default()));
		}

		let module = self.inner.module_for_id(id)?;
		Ok(Box::new(RefundingModule::<T>::new(module)))
	}
}

/// Built-in [`IsmpModule`] that performs relayer-fee withdrawals on behalf of
/// the hyperbridge coprocessor. Lives inside `pallet-ismp` so the protocol can
/// pay relayers without a dedicated companion pallet.
pub(crate) struct HyperbridgeWithdrawalModule<T>(PhantomData<T>);

impl<T> Default for HyperbridgeWithdrawalModule<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T: Config> IsmpModule for HyperbridgeWithdrawalModule<T> {
	fn on_accept(&self, request: PostRequest) -> Result<Weight, anyhow::Error> {
		// Only the configured coprocessor may instruct withdrawals.
		let source = request.source;
		if Some(source) != T::Coprocessor::get() {
			Err(IsmpError::Custom(format!("Invalid request source: {source}")))?
		}

		let message = Message::<T::AccountId, T::Balance>::decode(&mut &request.body[..])
			.map_err(|err| IsmpError::Custom(format!("Failed to decode message: {err:?}")))?;

		match message {
			Message::WithdrawRelayerFees(WithdrawalRequest { account, amount }) => {
				T::Currency::transfer(
					&RELAYER_FEE_ACCOUNT.into_account_truncating(),
					&account,
					amount,
					Preservation::Expendable,
				)
				.map_err(|err| {
					IsmpError::Custom(format!("Error withdrawing protocol fees: {err:?}"))
				})?;

				Pallet::<T>::deposit_event(Event::<T>::RelayerFeeWithdrawn { amount, account });
			},
		}

		Ok(<T as frame_system::Config>::DbWeight::get().reads_writes(0, 0))
	}

	fn on_response(&self, _response: GetResponse) -> Result<Weight, anyhow::Error> {
		Err(IsmpError::CannotHandleMessage.into())
	}

	fn on_timeout(&self, _request: Request, _meta: Option<&[u8]>) -> Result<Weight, anyhow::Error> {
		Err(IsmpError::CannotHandleMessage.into())
	}
}

/// Wraps a user module so that a successful timeout callback refunds the
/// escrowed relayer fee back to the original payer. The host has already
/// deleted the commitment by the time we run, so the fee metadata is read
/// from the `meta` argument that the framework threads through.
pub(crate) struct RefundingModule<T> {
	inner: Box<dyn IsmpModule>,
	_phantom: PhantomData<T>,
}

impl<T: Config> RefundingModule<T> {
	pub fn new(inner: Box<dyn IsmpModule>) -> Self {
		Self { inner, _phantom: PhantomData }
	}
}

impl<T: Config> IsmpModule for RefundingModule<T> {
	fn on_accept(&self, request: PostRequest) -> Result<Weight, anyhow::Error> {
		self.inner.on_accept(request)
	}

	fn on_response(&self, response: GetResponse) -> Result<Weight, anyhow::Error> {
		self.inner.on_response(response)
	}

	fn on_timeout(&self, request: Request, meta: Option<&[u8]>) -> Result<Weight, anyhow::Error> {
		let result = self.inner.on_timeout(request, meta);

		if result.is_ok() {
			if let Some(bytes) = meta {
				let decoded = RequestMetadata::<T>::decode(&mut &*bytes).map_err(|err| {
					IsmpError::Custom(format!("Failed to decode request metadata: {err:?}"))
				})?;
				let fee = decoded.fee;
				if fee.fee > Zero::zero() {
					T::Currency::transfer(
						&RELAYER_FEE_ACCOUNT.into_account_truncating(),
						&fee.payer,
						fee.fee,
						Preservation::Expendable,
					)
					.map_err(|err| {
						IsmpError::Custom(format!("Failed to refund relayer fee: {err:?}"))
					})?;
				}
			}
		}

		result
	}
}
