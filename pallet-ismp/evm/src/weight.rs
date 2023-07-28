//! Weight info utilities for evm contracts
use core::marker::PhantomData;
use frame_support::dispatch::Weight;
use ismp_rs::router::{Post, Request, Response};
use pallet_evm::GasWeightMapping;
use pallet_ismp::{weight_info::IsmpModuleWeight, Config};

/// An implementation of IsmpModuleWeight for evm contract callbacks
pub struct EvmWeightCalculator<T: Config + pallet_evm::Config>(PhantomData<T>);

impl<T: Config + pallet_evm::Config> Default for EvmWeightCalculator<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Config + pallet_evm::Config> IsmpModuleWeight for EvmWeightCalculator<T> {
    fn on_accept(&self, request: &Post) -> Weight {
        <T as pallet_evm::Config>::GasWeightMapping::gas_to_weight(request.gas_limit, true)
    }

    fn on_timeout(&self, request: &Request) -> Weight {
        match request {
            Request::Post(post) => {
                <T as pallet_evm::Config>::GasWeightMapping::gas_to_weight(post.gas_limit, true)
            }
            Request::Get(get) => {
                <T as pallet_evm::Config>::GasWeightMapping::gas_to_weight(get.gas_limit, true)
            }
        }
    }

    fn on_response(&self, response: &Response) -> Weight {
        match response {
            Response::Post(response) => <T as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
                response.post.gas_limit,
                true,
            ),
            Response::Get(response) => <T as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
                response.get.gas_limit,
                true,
            ),
        }
    }
}
