# Pallet Token Gateway

This allows standalone chains or parachains make asset transfers to and from EVM token gateway deployments.


## Overview

The Pallet allows the [`AdminOrigin`](https://docs.rs/pallet-ismp/latest/pallet_ismp/pallet/trait.Config.html#associatedtype.AdminOrigin) configured in [`pallet-ismp`](https://docs.rs/pallet-ismp/latest/pallet_ismp) to dispatch calls for registering asset Ids.

## Adding to Runtime

The first step is to implement the pallet config for the runtime.

```rust,ignore
use frame_support::parameter_types;
use ismp::module::IsmpModule;
use ismp::router::IsmpRouter;

parameter_types! {
    // A constant that should represent the native asset id
    pub const NativeAssetId: u32 = 0; 
    // Set the correct decimals for the native currency
    pub const Decimals: u8 = 12;
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
    // A type that provides a function for creating unique asset ids
    // An implementation is required  for asset creation calls or messages
    type CreateAsset = ();
    // The decimals value of the native asset
    type Decimals = Decimals;
}

// Add the pallet to your ISMP router
#[derive(Default)]
struct Router;
impl IsmpRouter for Router {
    fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
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

1.  Register your native assets directly on `Hyperbridge` by dispatching  `create_erc6160_asset`.
3.  Set token gateway addresses for the EVM chains of interest by dispatching the `set_token_gateway_addresses` extrinsic.
    This allows us validate incoming requests.
    

## Dispatchable Functions

- `teleport` - This function is used to bridge assets through Hyperbridge.
- `set_token_gateway_addresses` - This call allows the `AdminOrigin` origin to set the token gateway address for EVM chains.
- `create_erc6160_asset` - This call dispatches a request to Hyperbridge to create multi chain native assets on token gateway deployments

## Asset creation
When creating assets, the metadata needs to be set, the account set as the asset owner is the pallet account, depending on the fungibles implementation, funding the pallet account might be required for the asset creation to succeed.

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2024 Polytope Labs.
