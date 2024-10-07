# Pallet Token Gateway

This allows standalone chains or parachains make asset transfers to and from EVM token gateway deployments.


## Overview

The Pallet allows the [`AdminOrigin`](https://docs.rs/pallet-ismp/latest/pallet_ismp/pallet/trait.Config.html#associatedtype.AdminOrigin) configured in [`pallet-ismp`](https://docs.rs/pallet-ismp/latest/pallet_ismp) to dispatch calls for registering asset Ids
and also requesting the token gateway addresses from Hyperbridge.

## Adding to Runtime

The first step is to implement the pallet config for the runtime.

```rust,ignore
use frame_support::parameter_types;
use ismp::Error;
use ismp::host::StateMachine;
use ismp::module::IsmpModule;
use ismp::router::{IsmpRouter, Post, Response, Timeout};

parameter_types! {
    // The Native asset Id for the native currency, for parachains this would be the XCM location for the parachain
    // For standalone chains, any constant of your choosing 
    pub const NativeAssetId: StateMachine = Location::here(); 
}

impl pallet_ismp::Config for Runtime {
    // configure the runtime event
    type RuntimeEvent = RuntimeEvent;
    // Pallet Ismp 
    type Dispatcher = Ismp;
    // Pallet Assets
	type Assets = Assets;
    // Pallet balances
	type Currency = Balances;
    // The Native asset Id
	type NativeAssetId = NativeAssetId;
}

#[derive(Default)]
struct Router;
impl IsmpRouter for Router {
    fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, Error> {
        let module = match id.as_slice() {
            id if TokenGateway::is_token_gateway(&id) => Box::new(TokenGateway::default()),
            _ => Err(Error::ModuleNotFound(id))?
        };
        Ok(module)
    }
}
``` 

## Setting up

The pallet requires some setting up before the teleport function is available for use in the runtime.

1.  Register your native asset directly on `Hyperbridge` by dispatching  `TokenGovernor::create_erc6160_asset`.
2.  Register a map of local asset Ids to their token gateway equivalents by dispatching `register_assets` extrinsic.
    Note: This registration must be done for your native asset also.
3.  Request token gateway addresses for the EVM chains of interest by dispatching the `request_token_gateway_address` extrinsic.
    

## Dispatchable Functions

- `teleport` - This function is used to bridge assets to EVM chains through Hyperbridge.
- `request_token_gateway_address` - This call allows the `AdminOrigin` origin to request the token gateway addresses from Hyperbridge.
- `register_assets` - This call allows the configured `AdminOrigin` to register a map of local asset ids to their equivalent asset ids on token gateway.

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2024 Polytope Labs.
