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

//! Implementation of the [`IIsmpHost`] trait as a precompile for PolkaVM contracts.

mod interface;
pub use interface::*;

use alloy_primitives::{Address, Uint};
use core::str::FromStr;
use pallet_hyperbridge::VersionedHostParams;
use pallet_ismp::{FundMessageParams, MessageCommitment};
use polkadot_sdk::*;

use frame_system::RawOrigin;
use sp_core::{H160, H256};
use sp_runtime::traits::Dispatchable;

use core::{marker::PhantomData, num::NonZero};
use frame_support::traits::Get;
use ismp::{
	dispatcher::{self, DispatchRequest, IsmpDispatcher},
	host::StateMachine,
	router,
};
use num_traits::{FromPrimitive, ToPrimitive};
use pallet_revive::precompiles::{
	alloy::{
		primitives::FixedBytes,
		sol_types::{Revert, SolValue},
	},
	AddressMapper, AddressMatcher, Error, Ext, Precompile,
};

use IIsmpHost::IIsmpHostCalls;

/// [`pallet_revive::precompiles::Precompile`] implementation for [`ismp`] protocol implementations
pub struct ReviveHost<Runtime, Dispatcher, FeeToken>(PhantomData<(Runtime, Dispatcher, FeeToken)>);

// Todos
// 1. figure out gas costs
// 2. clarify env.account_id()
impl<Runtime, Dispatcher, FeeToken> Precompile for ReviveHost<Runtime, Dispatcher, FeeToken>
where
	Runtime: pallet_ismp::Config + pallet_revive::Config + pallet_hyperbridge::Config,
	Runtime::AccountId: TryFrom<Vec<u8>>,
	Runtime::Balance: Into<u128> + From<u128>,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_ismp::Call<Runtime>>,
	Dispatcher: IsmpDispatcher<Account = Runtime::AccountId, Balance = Runtime::Balance>,
	FeeToken: Get<Address>,
{
	type T = Runtime;
	const MATCHER: AddressMatcher = AddressMatcher::Fixed(NonZero::new(999).unwrap());
	const HAS_CONTRACT_INFO: bool = false;
	type Interface = IIsmpHost::IIsmpHostCalls;

	fn call(
		address: &[u8; 20],
		input: &Self::Interface,
		_env: &mut impl Ext<T = Self::T>,
	) -> Result<Vec<u8>, Error> {
		match input {
			IIsmpHostCalls::responded(IIsmpHost::respondedCall { commitment }) => {
				let value = pallet_ismp::Responded::<Runtime>::get(H256(commitment.0.clone()));
				return Ok(value.abi_encode());
			},
			IIsmpHostCalls::requestReceipts(IIsmpHost::requestReceiptsCall { commitment }) => {
				if let Some(relayer) = pallet_ismp::child_trie::RequestReceipts::<Runtime>::get(
					H256(commitment.0.clone()),
				) {
					let account_id = Runtime::AccountId::try_from(relayer).map_err(|_| {
						Error::Revert(Revert { reason: "Invalid account ID".into() })
					})?;
					let address = Runtime::AddressMapper::to_address(&account_id);
					return Ok(FixedBytes::<20>::from(address.0).abi_encode());
				}

				return Ok(FixedBytes::<20>::default().abi_encode());
			},
			IIsmpHostCalls::responseReceipts(IIsmpHost::responseReceiptsCall { commitment }) => {
				if let Some(receipt) = pallet_ismp::child_trie::ResponseReceipts::<Runtime>::get(
					H256(commitment.0.clone()),
				) {
					let account_id =
						Runtime::AccountId::try_from(receipt.relayer).map_err(|_| {
							Error::Revert(Revert { reason: "Invalid account ID".into() })
						})?;
					let address = Runtime::AddressMapper::to_address(&account_id);
					let metadata = ResponseReceipt {
						responseCommitment: receipt.response.0.into(),
						relayer: address.0.into(),
					};
					return Ok(metadata.abi_encode());
				}

				return Ok(ResponseReceipt::default().abi_encode());
			},
			IIsmpHostCalls::responseCommitments(IIsmpHost::responseCommitmentsCall {
				commitment,
			}) => {
				if let Some(metadata) = pallet_ismp::child_trie::ResponseCommitments::<Runtime>::get(
					H256(commitment.0.clone()),
				) {
					let address = Runtime::AddressMapper::to_address(&metadata.fee.payer);
					let fee = Uint::<256, 4>::from_u128(metadata.fee.fee.into())
						.expect("u128 will always fit in a u256");
					return Ok(FeeMetadata { fee, sender: address.0.into() }.abi_encode());
				}

				return Ok(FeeMetadata::default().abi_encode());
			},
			IIsmpHostCalls::requestCommitments(IIsmpHost::requestCommitmentsCall {
				commitment,
			}) => {
				if let Some(metadata) = pallet_ismp::child_trie::RequestCommitments::<Runtime>::get(
					H256(commitment.0.clone()),
				) {
					let address = Runtime::AddressMapper::to_address(&metadata.fee.payer);
					let fee = Uint::<256, 4>::from_u128(metadata.fee.fee.into())
						.expect("u128 will always fit in a u256");
					return Ok(FeeMetadata { fee, sender: address.0.into() }.abi_encode());
				}

				return Ok(FeeMetadata::default().abi_encode());
			},
			IIsmpHostCalls::host(IIsmpHost::hostCall) => {
				let host = Runtime::HostStateMachine::get();
				return Ok(host.to_string().as_bytes().to_vec().abi_encode());
			},
			IIsmpHostCalls::hyperbridge(IIsmpHost::hyperbridgeCall) => {
				let Some(hyperbridge) = Runtime::Coprocessor::get() else {
					Err(Error::Revert(Revert { reason: "Hyperbridge not defined".into() }))?
				};
				return Ok(hyperbridge.to_string().as_bytes().to_vec().abi_encode());
			},
			IIsmpHostCalls::nonce(IIsmpHost::nonceCall) => {
				let nonce = pallet_ismp::Nonce::<Runtime>::get();
				return Ok(Uint::<256, 4>::from(nonce).abi_encode());
			},
			IIsmpHostCalls::feeToken(IIsmpHost::feeTokenCall) =>
				return Ok(FeeToken::get().abi_encode()),
			IIsmpHostCalls::perByteFee(IIsmpHost::perByteFeeCall { dest }) => {
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
			IIsmpHostCalls::dispatch_0(IIsmpHost::dispatch_0Call { request }) => {
				let contract_address = H160::from(address.clone());
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
							from: address.to_vec(),
							to: request.to.to_vec(),
							timeout: request.timeout,
							dest: StateMachine::from_str(&destination).map_err(|_| {
								Error::Revert(Revert { reason: "Invalid state machine".into() })
							})?,
						}),
						dispatcher::FeeMetadata {
							payer: Runtime::AddressMapper::to_account_id(&contract_address),
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
			IIsmpHostCalls::dispatch_1(IIsmpHost::dispatch_1Call { request }) => {
				let contract_address = H160::from(address.clone());
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
							from: address.to_vec(),
							keys: request.keys.iter().map(|key| key.to_vec()).collect(),
							timeout: request.timeout,
							dest: StateMachine::from_str(&destination).map_err(|_| {
								Error::Revert(Revert { reason: "Invalid state machine".into() })
							})?,
						}),
						dispatcher::FeeMetadata {
							payer: Runtime::AddressMapper::to_account_id(&contract_address),
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
			IIsmpHostCalls::dispatch_2(IIsmpHost::dispatch_2Call { response }) => {
				let contract_address = H160::from(address.clone());
				let destination = String::from_utf8(response.request.source.to_vec())
					.map_err(|_| Error::Revert(Revert { reason: "Invalid destination".into() }))?;
				let source = String::from_utf8(response.request.dest.to_vec())
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
								from: address.to_vec(),
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
							payer: Runtime::AddressMapper::to_account_id(&contract_address),
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
			IIsmpHostCalls::fundRequest(IIsmpHost::fundRequestCall { commitment, amount }) => {
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
				call.dispatch(
					RawOrigin::Signed(Runtime::AddressMapper::to_account_id(&H160(
						address.clone(),
					)))
					.into(),
				)
				.map_err(|_| Error::Revert(Revert { reason: "Failed to fund request".into() }))?;
				return Ok(Default::default());
			},
			IIsmpHostCalls::fundResponse(IIsmpHost::fundResponseCall { commitment, amount }) => {
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
				call.dispatch(
					RawOrigin::Signed(Runtime::AddressMapper::to_account_id(&H160(
						address.clone(),
					)))
					.into(),
				)
				.map_err(|_| Error::Revert(Revert { reason: "Failed to fund request".into() }))?;
				return Ok(Default::default());
			},
		}
	}
}
