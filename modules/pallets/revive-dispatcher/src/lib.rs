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

//! Implementation of the ISMP dispatcher as a precompile for PolkaVM contracts.
extern crate alloc;

#[cfg(test)]
mod tests;
use polkadot_sdk::*;

use alloc::{
	format,
	string::{String, ToString},
	vec::Vec,
};
use core::{convert::From, default::Default, result::Result::*, str::FromStr};
use pallet_hyperbridge::VersionedHostParams;
use pallet_ismp::{FundMessageParams, MessageCommitment};

use frame_system::RawOrigin;
use sp_core::H256;
use sp_runtime::traits::Dispatchable;

use core::{marker::PhantomData, num::NonZero};
use frame_support::{traits::Get, weights::Weight};
use ismp::{
	dispatcher::{self, DispatchRequest, IsmpDispatcher},
	host::StateMachine,
	router,
};
use num_traits::{FromPrimitive, ToPrimitive};
use pallet_revive::precompiles::{
	alloy::{
		self,
		primitives::{Address, FixedBytes, Uint},
		sol_types::{Revert, SolValue},
	},
	AddressMapper, AddressMatcher, Error, Ext, Precompile,
};

alloy::sol!("src/IDispatcher.sol");
use IDispatcher::IDispatcherCalls;

/// Trait that provides weights for ISMP dispatcher operations
pub trait DispatcherWeightSchedule {
	/// Weight for getting the host state machine
	fn host() -> Weight;
	/// Weight for getting the hyperbridge address
	fn hyperbridge() -> Weight;
	/// Weight for getting the current nonce
	fn nonce() -> Weight;
	/// Weight for getting the fee token address
	fn fee_token() -> Weight;
	/// Weight for getting the per-byte fee for a destination
	fn per_byte_fee() -> Weight;
	/// Weight for dispatching a POST request
	fn dispatch_post() -> Weight;
	/// Weight for dispatching a GET request
	fn dispatch_get() -> Weight;
	/// Weight for dispatching a response
	fn dispatch_response() -> Weight;
	/// Weight for funding a request
	fn fund_request() -> Weight;
	/// Weight for funding a response
	fn fund_response() -> Weight;
}

/// [`pallet_revive::precompiles::Precompile`] implementation for [`ismp`] protocol dispatcher
pub struct ReviveDispatcher<Runtime, Dispatcher, FeeToken, WeightSchedule>(
	PhantomData<(Runtime, Dispatcher, FeeToken, WeightSchedule)>,
);

// Todo: Expose IsmpModule implementation
impl<Runtime, Dispatcher, FeeToken, WeightSchedule> Precompile
	for ReviveDispatcher<Runtime, Dispatcher, FeeToken, WeightSchedule>
where
	Runtime: pallet_ismp::Config + pallet_revive::Config + pallet_hyperbridge::Config,
	Runtime::AccountId: for<'a> TryFrom<&'a [u8]>,
	Runtime::Balance: Into<u128> + From<u128>,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_ismp::Call<Runtime>>,
	Dispatcher: IsmpDispatcher<Account = Runtime::AccountId, Balance = Runtime::Balance>,
	FeeToken: Get<Address>,
	WeightSchedule: DispatcherWeightSchedule,
{
	type T = Runtime;
	const MATCHER: AddressMatcher = AddressMatcher::Fixed(NonZero::new(3367).unwrap());
	const HAS_CONTRACT_INFO: bool = false;
	type Interface = IDispatcher::IDispatcherCalls;

	fn call(
		_address: &[u8; 20],
		input: &Self::Interface,
		env: &mut impl Ext<T = Self::T>,
	) -> Result<Vec<u8>, Error> {
		let caller = env.caller();
		let caller_account_id = caller
			.account_id()
			.map_err(|_| Error::Revert(Revert { reason: "Caller must be signed".into() }))?;
		let address = Runtime::AddressMapper::to_address(&caller_account_id);

		match input {
			IDispatcherCalls::host(IDispatcher::hostCall) => {
				env.charge(WeightSchedule::host())?;
				let host = Runtime::HostStateMachine::get();
				return Ok(host.to_string().as_bytes().to_vec().abi_encode());
			},
			IDispatcherCalls::hyperbridge(IDispatcher::hyperbridgeCall) => {
				env.charge(WeightSchedule::hyperbridge())?;
				let Some(hyperbridge) = Runtime::Coprocessor::get() else {
					Err(Error::Revert(Revert { reason: "Hyperbridge not defined".into() }))?
				};
				return Ok(hyperbridge.to_string().as_bytes().to_vec().abi_encode());
			},
			IDispatcherCalls::nonce(IDispatcher::nonceCall) => {
				env.charge(WeightSchedule::nonce())?;
				let nonce = pallet_ismp::Nonce::<Runtime>::get();
				return Ok(Uint::<256, 4>::from(nonce).abi_encode());
			},
			IDispatcherCalls::feeToken(IDispatcher::feeTokenCall) => {
				env.charge(WeightSchedule::fee_token())?;
				return Ok(FeeToken::get().abi_encode());
			},
			IDispatcherCalls::perByteFee(IDispatcher::perByteFeeCall { dest }) => {
				env.charge(WeightSchedule::per_byte_fee())?;
				let utf8 = String::from_utf8(dest.to_vec()).map_err(|_| {
					Error::Revert(Revert { reason: "Invalid state machine".into() })
				})?;
				let state_machine = StateMachine::from_str(&utf8).map_err(|_| {
					Error::Revert(Revert { reason: "Invalid state machine".into() })
				})?;
				let VersionedHostParams::<Runtime::Balance>::V1(host_params) =
					pallet_hyperbridge::HostParams::<Runtime>::get();
				let per_byte_fee = host_params
					.per_byte_fees
					.get(&state_machine)
					.unwrap_or(&host_params.default_per_byte_fee);

				let fee = Uint::<256, 4>::from_u128((*per_byte_fee).into())
					.expect("u128 will always fit in a u256");

				return Ok(fee.abi_encode());
			},
			IDispatcherCalls::dispatch_0(IDispatcher::dispatch_0Call { request }) => {
				env.charge(WeightSchedule::dispatch_post())?;
				let destination = String::from_utf8(request.dest.to_vec())
					.map_err(|_| Error::Revert(Revert { reason: "Invalid destination".into() }))?;
				let relayer_fee = request
					.fee
					.to_u128()
					.ok_or(Error::Revert(Revert { reason: "Invalid fee".into() }))?;
				let commitment = Dispatcher::default()
					.dispatch_request(
						DispatchRequest::Post(dispatcher::DispatchPost {
							body: request.body.to_vec(),
							from: address.0.to_vec(),
							to: request.to.to_vec(),
							timeout: request.timeout,
							dest: StateMachine::from_str(&destination).map_err(|_| {
								Error::Revert(Revert { reason: "Invalid state machine".into() })
							})?,
						}),
						dispatcher::FeeMetadata {
							payer: caller_account_id.clone(),
							fee: From::from(relayer_fee),
						},
					)
					.map_err(|err| {
						Error::Revert(Revert {
							reason: format!("Failed to dispatch request: {}", err),
						})
					})?;
				return Ok(FixedBytes::<32>::from(commitment.0).abi_encode());
			},
			IDispatcherCalls::dispatch_1(IDispatcher::dispatch_1Call { request }) => {
				env.charge(WeightSchedule::dispatch_get())?;
				let destination = String::from_utf8(request.dest.to_vec())
					.map_err(|_| Error::Revert(Revert { reason: "Invalid destination".into() }))?;
				let relayer_fee = request
					.fee
					.to_u128()
					.ok_or(Error::Revert(Revert { reason: "Invalid fee".into() }))?;
				let commitment = Dispatcher::default()
					.dispatch_request(
						DispatchRequest::Get(dispatcher::DispatchGet {
							context: request.context.to_vec(),
							height: request.height,
							from: address.0.to_vec(),
							keys: request.keys.iter().map(|key| key.to_vec()).collect(),
							timeout: request.timeout,
							dest: StateMachine::from_str(&destination).map_err(|_| {
								Error::Revert(Revert { reason: "Invalid state machine".into() })
							})?,
						}),
						dispatcher::FeeMetadata {
							payer: caller_account_id.clone(),
							fee: From::from(relayer_fee),
						},
					)
					.map_err(|err| {
						Error::Revert(Revert {
							reason: format!("Failed to dispatch request: {}", err),
						})
					})?;
				return Ok(FixedBytes::<32>::from(commitment.0).abi_encode());
			},
			IDispatcherCalls::dispatch_2(IDispatcher::dispatch_2Call { response }) => {
				env.charge(WeightSchedule::dispatch_response())?;
				let destination = String::from_utf8(response.request.dest.to_vec())
					.map_err(|_| Error::Revert(Revert { reason: "Invalid destination".into() }))?;
				let source = String::from_utf8(response.request.source.to_vec())
					.map_err(|_| Error::Revert(Revert { reason: "Invalid source".into() }))?;
				let relayer_fee = response
					.fee
					.to_u128()
					.ok_or(Error::Revert(Revert { reason: "Invalid fee".into() }))?;
				let commitment = Dispatcher::default()
					.dispatch_response(
						router::PostResponse {
							post: router::PostRequest {
								body: response.request.body.to_vec(),
								from: address.0.to_vec(),
								to: response.request.to.to_vec(),
								timeout_timestamp: response.request.timeoutTimestamp,
								dest: StateMachine::from_str(&destination).map_err(|_| {
									Error::Revert(Revert { reason: "Invalid state machine".into() })
								})?,
								nonce: response.request.nonce,
								source: StateMachine::from_str(&source).map_err(|_| {
									Error::Revert(Revert { reason: "Invalid state machine".into() })
								})?,
							},
							response: response.response.to_vec(),
							timeout_timestamp: response.timeout,
						},
						dispatcher::FeeMetadata {
							payer: caller_account_id.clone(),
							fee: From::from(relayer_fee),
						},
					)
					.map_err(|err| {
						Error::Revert(Revert {
							reason: format!("Failed to dispatch request: {}", err),
						})
					})?;
				return Ok(FixedBytes::<32>::from(commitment.0).abi_encode());
			},
			IDispatcherCalls::fundRequest(IDispatcher::fundRequestCall { commitment, amount }) => {
				env.charge(WeightSchedule::fund_request())?;
				let new_fee = amount
					.to_u128()
					.ok_or(Error::Revert(Revert { reason: "Invalid fee".into() }))?;
				let call: <Runtime as frame_system::Config>::RuntimeCall =
					pallet_ismp::Call::<Runtime>::fund_message {
						message: FundMessageParams {
							commitment: MessageCommitment::Request(H256(commitment.0)),
							amount: From::from(new_fee),
						},
					}
					.into();
				call.dispatch(RawOrigin::Signed(caller_account_id.clone()).into()).map_err(
					|_| Error::Revert(Revert { reason: "Failed to fund request".into() }),
				)?;
				return Ok(Default::default());
			},
			IDispatcherCalls::fundResponse(IDispatcher::fundResponseCall {
				commitment,
				amount,
			}) => {
				env.charge(WeightSchedule::fund_response())?;
				let new_fee = amount
					.to_u128()
					.ok_or(Error::Revert(Revert { reason: "Invalid fee".into() }))?;
				let call: <Runtime as frame_system::Config>::RuntimeCall =
					pallet_ismp::Call::<Runtime>::fund_message {
						message: FundMessageParams {
							commitment: MessageCommitment::Response(H256(commitment.0)),
							amount: From::from(new_fee),
						},
					}
					.into();
				call.dispatch(RawOrigin::Signed(caller_account_id.clone()).into()).map_err(
					|_| Error::Revert(Revert { reason: "Failed to fund request".into() }),
				)?;
				return Ok(Default::default());
			},
		}
	}
}
