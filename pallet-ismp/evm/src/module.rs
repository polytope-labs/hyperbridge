//! Module Handler for EVM contracts
use crate::abi::{
    onAcceptCall, onGetResponseCall, onGetTimeoutCall, onPostResponseCall, onPostTimeoutCall,
    GetRequest as SolGetRequest, GetResponse as SolGetResponse, PostRequest,
    PostResponse as SolPostResponse, StorageValue as SolStorageValue,
};
use alloc::{format, string::ToString};
use alloy_sol_types::SolCall;
use core::marker::PhantomData;
use fp_evm::{ExitReason, FeeCalculator};
use hex_literal::hex;
use ismp_rs::{
    error::Error,
    module::IsmpModule,
    router::{Post, Request, Response},
};
use pallet_evm::GasWeightMapping;
use pallet_ismp::{primitives::ModuleId, WeightConsumed};
use sp_core::H160;
use sp_std::prelude::*;

/// Handler host address
/// Contracts should only allow ismp module callbacks to be executed by this address
pub const EVM_HOST_ADDRESS: [u8; 20] = hex!("843b131bd76419934dae248f6e5a195c0a3c324d");

/// [`IsmpModule`] implementation that routes requests & responses to EVM contracts.
pub struct EvmIsmpModule<T: pallet_ismp::Config + pallet_evm::Config>(PhantomData<T>);

impl<T: pallet_ismp::Config + pallet_evm::Config> Default for EvmIsmpModule<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: pallet_ismp::Config + pallet_evm::Config> IsmpModule for EvmIsmpModule<T> {
    fn on_accept(&self, request: Post) -> Result<(), Error> {
        let target_contract = parse_contract_id(&request.to)?;
        let gaslimit = request.gas_limit;
        let post = PostRequest {
            source: request.source.to_string().as_bytes().to_vec(),
            dest: request.dest.to_string().as_bytes().to_vec(),
            nonce: request.nonce,
            timeoutTimestamp: request.timeout_timestamp,
            from: request.from,
            to: request.to,
            body: request.data,
            gaslimit,
        };
        let call_data = onAcceptCall { request: post }.encode();
        execute_call::<T>(target_contract, call_data, gaslimit)
    }

    fn on_response(&self, response: Response) -> Result<(), Error> {
        let target_contract = parse_contract_id(&response.destination_module())?;

        let (call_data, gas_limit) = match response {
            Response::Post(response) => {
                // we set the gas limit for executing the contract to be the same as used in the
                // request. we assume the request was dispatched with a gas limit
                // that accounts for execution of the response on this source chain
                let gaslimit = response.post.gas_limit;
                let post_response = SolPostResponse {
                    request: PostRequest {
                        source: response.post.source.to_string().as_bytes().to_vec(),
                        dest: response.post.dest.to_string().as_bytes().to_vec(),
                        nonce: response.post.nonce,
                        timeoutTimestamp: response.post.timeout_timestamp,
                        from: response.post.from,
                        to: response.post.to,
                        body: response.post.data,
                        gaslimit,
                    },
                    response: response.response,
                };
                (onPostResponseCall { response: post_response }.encode(), gaslimit)
            }
            Response::Get(response) => {
                let gaslimit = response.get.gas_limit;
                let get_response = SolGetResponse {
                    request: SolGetRequest {
                        source: response.get.source.to_string().as_bytes().to_vec(),
                        dest: response.get.dest.to_string().as_bytes().to_vec(),
                        nonce: response.get.nonce,
                        height: response.get.height,
                        timeoutTimestamp: response.get.timeout_timestamp,
                        from: response.get.from,
                        keys: response.get.keys,
                        gaslimit,
                    },
                    values: response
                        .values
                        .into_iter()
                        .map(|(key, value)| SolStorageValue {
                            key,
                            value: value.unwrap_or_default(),
                        })
                        .collect(),
                };
                (onGetResponseCall { response: get_response }.encode(), gaslimit)
            }
        };

        execute_call::<T>(target_contract, call_data, gas_limit)
    }

    fn on_timeout(&self, request: Request) -> Result<(), Error> {
        let target_contract = parse_contract_id(&request.source_module())?;
        let (call_data, gas_limit) = match request {
            Request::Post(post) => {
                let gaslimit = post.gas_limit;
                let request = PostRequest {
                    source: post.source.to_string().as_bytes().to_vec(),
                    dest: post.dest.to_string().as_bytes().to_vec(),
                    nonce: post.nonce,
                    timeoutTimestamp: post.timeout_timestamp,
                    from: post.from,
                    to: post.to,
                    body: post.data,
                    gaslimit,
                };
                (onPostTimeoutCall { request }.encode(), gaslimit)
            }
            Request::Get(get) => {
                let gaslimit = get.gas_limit;
                let request = SolGetRequest {
                    source: get.source.to_string().as_bytes().to_vec(),
                    dest: get.dest.to_string().as_bytes().to_vec(),
                    nonce: get.nonce,
                    height: get.height,
                    timeoutTimestamp: get.timeout_timestamp,
                    from: get.from,
                    keys: get.keys,
                    gaslimit,
                };
                (onGetTimeoutCall { request }.encode(), gaslimit)
            }
        };
        execute_call::<T>(target_contract, call_data, gas_limit)
    }
}

/// Parse contract id from raw bytes
pub fn parse_contract_id(bytes: &[u8]) -> Result<H160, Error> {
    let module_id =
        ModuleId::from_bytes(bytes).map_err(|e| Error::ImplementationSpecific(e.to_string()))?;
    match module_id {
        ModuleId::Evm(id) => Ok(id),
        _ => Err(Error::ImplementationSpecific("Expected Evm contract id".to_string())),
    }
}

/// Call execute call data
fn execute_call<T: pallet_ismp::Config + pallet_evm::Config>(
    target: H160,
    call_data: Vec<u8>,
    gas_limit: u64,
) -> Result<(), Error> {
    let (weight_used, result) =
        match <<T as pallet_evm::Config>::Runner as pallet_evm::Runner<T>>::call(
            H160::from(EVM_HOST_ADDRESS),
            target,
            call_data,
            Default::default(),
            gas_limit,
            Some(<<T as pallet_evm::Config>::FeeCalculator as FeeCalculator>::min_gas_price().0),
            Some(<<T as pallet_evm::Config>::FeeCalculator as FeeCalculator>::min_gas_price().0),
            None,
            Default::default(),
            true,
            true,
            None,
            None,
            <T as pallet_evm::Config>::config(),
        ) {
            Ok(info) => {
                let weight =
                    T::GasWeightMapping::gas_to_weight(info.used_gas.standard.low_u64(), true);
                let result = match info.exit_reason {
                    ExitReason::Succeed(_) => Ok(()),
                    _ => Err(Error::ImplementationSpecific(
                        "Contract call did not successfully execute".to_string(),
                    )),
                };
                (weight, result)
            }
            Err(error) => {
                let dispatch_error: sp_runtime::DispatchError = error.error.into();
                (
                    error.weight,
                    Err(Error::ImplementationSpecific(format!(
                        "Contract call failed with error {:?}",
                        dispatch_error
                    ))),
                )
            }
        };
    let mut total_weight_used = WeightConsumed::<T>::get();
    let weight_limit = T::GasWeightMapping::gas_to_weight(gas_limit, true);
    total_weight_used.weight_used = total_weight_used.weight_used + weight_used;
    total_weight_used.weight_limit = total_weight_used.weight_limit + weight_limit;
    WeightConsumed::<T>::put(total_weight_used);
    result
}
