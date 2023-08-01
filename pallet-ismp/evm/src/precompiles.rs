//! IsmpDispatcher precompiles for pallet-evm

use pallet_ismp::{dispatcher::Dispatcher, weight_info::WeightInfo};

use crate::abi::{
    DispatchGet as SolDispatchGet, DispatchPost as SolDispatchPost, PostResponse as SolPostResponse,
};
use alloc::{format, str::FromStr, string::String};
use alloy_sol_types::SolType;
use core::marker::PhantomData;
use fp_evm::{
    ExitError, ExitSucceed, Precompile, PrecompileFailure, PrecompileHandle, PrecompileOutput,
    PrecompileResult,
};
use frame_support::traits::Get;
use hex_literal::hex;
use ismp_rs::{
    host::StateMachine,
    router::{DispatchGet, DispatchPost, DispatchRequest, IsmpDispatcher, Post, PostResponse},
};
use pallet_evm::GasWeightMapping;
use sp_core::{H160, H256};
use sp_std::prelude::*;

/// Ismp Request Dispatcher precompile for evm contracts
pub struct IsmpPostDispatcher<T> {
    _marker: PhantomData<T>,
}

/// Address for the post request precompile
pub const POST_REQUEST_DISPATCHER: H160 = H160(hex!("222a98a2832ae77e72a768bf5be1f82d8959f4ec"));
/// Address for the post response precompile
pub const POST_RESPONSE_DISPATCHER: H160 = H160(hex!("eb928e2de75cb5ab60abe75f539c5312aeb46f38"));
/// Address for the get request precompile
pub const GET_REQUEST_DISPATCHER: H160 = H160(hex!("f2d8dc5239ddc053ba5151302483fc48d7e24e60"));

impl<T> Precompile for IsmpPostDispatcher<T>
where
    T: pallet_ismp::Config + pallet_evm::Config,
    <T as frame_system::Config>::Hash: From<H256>,
    H256: From<<T as frame_system::Config>::Hash>,
{
    fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
        let input = handle.input();
        let context = handle.context();
        let weight = <T as pallet_ismp::Config>::WeightInfo::dispatch_post_request();

        // The cost of a dispatch is the weight of calling the dispatcher plus an extra storage read
        // and write
        let cost = <T as pallet_evm::Config>::GasWeightMapping::weight_to_gas(weight);

        let dispatcher = Dispatcher::<T>::default();
        let post_dispatch =
            SolDispatchPost::decode(input, true).map_err(|e| PrecompileFailure::Error {
                exit_status: ExitError::Other(format!("Failed to decode input: {:?}", e).into()),
            })?;

        let post_dispatch = DispatchPost {
            dest: parse_state_machine(post_dispatch.dest)?,
            from: context.caller.0.to_vec(),
            to: post_dispatch.to,
            timeout_timestamp: post_dispatch.timeoutTimestamp,
            data: post_dispatch.body,
            gas_limit: post_dispatch.gaslimit,
        };

        handle.record_cost(cost)?;
        match dispatcher.dispatch_request(DispatchRequest::Post(post_dispatch)) {
            Ok(_) => Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: vec![] }),
            Err(e) => Err(PrecompileFailure::Error {
                exit_status: ExitError::Other(format!("dispatch execution failed: {:?}", e).into()),
            }),
        }
    }
}

/// Ismp Get Request Dispatcher precompile for evm contracts
pub struct IsmpGetDispatcher<T> {
    _marker: PhantomData<T>,
}

impl<T> Precompile for IsmpGetDispatcher<T>
where
    T: pallet_ismp::Config + pallet_evm::Config,
    <T as frame_system::Config>::Hash: From<H256>,
    H256: From<<T as frame_system::Config>::Hash>,
{
    fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
        let input = handle.input();
        let context = handle.context();

        let weight = <T as pallet_ismp::Config>::WeightInfo::dispatch_get_request();

        // The cost of a dispatch is the weight of calling the dispatcher plus an extra storage read
        // and write
        let cost = <T as pallet_evm::Config>::GasWeightMapping::weight_to_gas(
            weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1)),
        );

        let dispatcher = Dispatcher::<T>::default();

        let get_dispatch =
            SolDispatchGet::decode(input, true).map_err(|e| PrecompileFailure::Error {
                exit_status: ExitError::Other(format!("Failed to decode input: {:?}", e).into()),
            })?;
        let get_dispatch = DispatchGet {
            dest: parse_state_machine(get_dispatch.dest)?,
            from: context.caller.0.to_vec(),
            keys: get_dispatch.keys,
            height: get_dispatch.height,
            timeout_timestamp: get_dispatch.timeoutTimestamp,
            gas_limit: get_dispatch.gaslimit,
        };

        handle.record_cost(cost)?;
        match dispatcher.dispatch_request(DispatchRequest::Get(get_dispatch)) {
            Ok(_) => Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: vec![] }),
            Err(e) => Err(PrecompileFailure::Error {
                exit_status: ExitError::Other(format!("dispatch execution failed: {:?}", e).into()),
            }),
        }
    }
}

/// Ismp Response Dispatcher precompile for evm contracts
pub struct IsmpResponseDispatcher<T> {
    _marker: PhantomData<T>,
}

impl<T> Precompile for IsmpResponseDispatcher<T>
where
    T: pallet_ismp::Config + pallet_evm::Config,
    <T as frame_system::Config>::Hash: From<H256>,
    H256: From<<T as frame_system::Config>::Hash>,
{
    fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
        let input = handle.input();

        let weight = <T as pallet_ismp::Config>::WeightInfo::dispatch_response();

        let cost = <T as pallet_evm::Config>::GasWeightMapping::weight_to_gas(weight);

        let dispatcher = Dispatcher::<T>::default();
        let response =
            SolPostResponse::decode(input, true).map_err(|e| PrecompileFailure::Error {
                exit_status: ExitError::Other(format!("Failed to decode input: {:?}", e).into()),
            })?;
        let post_response = PostResponse {
            post: Post {
                source: parse_state_machine(response.request.source)?,
                dest: parse_state_machine(response.request.dest)?,
                nonce: response.request.nonce,
                from: response.request.from,
                to: response.request.to,
                timeout_timestamp: response.request.timeoutTimestamp,
                data: response.request.body,
                gas_limit: response.request.gaslimit,
            },
            response: response.response,
        };
        handle.record_cost(cost)?;

        match dispatcher.dispatch_response(post_response) {
            Ok(_) => Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: vec![] }),
            Err(e) => Err(PrecompileFailure::Error {
                exit_status: ExitError::Other(format!("dispatch execution failed: {:?}", e).into()),
            }),
        }
    }
}

/// Parse state machine from utf8 bytes
fn parse_state_machine(bytes: Vec<u8>) -> Result<StateMachine, PrecompileFailure> {
    StateMachine::from_str(&String::from_utf8(bytes).unwrap_or_default()).map_err(|e| {
        PrecompileFailure::Error {
            exit_status: ExitError::Other(format!("Failed to destination chain: {:?}", e).into()),
        }
    })
}
