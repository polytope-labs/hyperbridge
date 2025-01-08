# Pallet Hyperbridge

Pallet hyperbridge mediates the connection between hyperbridge and substrate-based chains. This pallet provides:

 - An [`IsmpDispatcher`] implementation which collects protocol fees and commits the reciepts for these fees to child storage. Hyperbridge only accepts messages that have been paid for using this module.
 - An [`IsmpModule`] which recieves and processes requests from hyperbridge. These requests are dispatched by hyperbridge governance and may adjust fees or request payouts for both relayers and protocol revenue.

This pallet contains no calls and dispatches no requests. Substrate based chains should use this to dispatch requests that should be processed by hyperbridge.

## Usage

This module must be configured as an [`IsmpModule`] in your [`IsmpRouter`] implementation so that it may receive
important messages from hyperbridge such as paramter updates or relayer fee withdrawals.

```rust,ignore
use ismp::Error;
use ismp::module::IsmpModule;
use ismp::router::IsmpRouter;
use pallet_hyperbridge::PALLET_HYPERBRIDGE_ID;

#[derive(Default)]
struct ModuleRouter;

impl IsmpRouter for ModuleRouter {
    fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, Error> {
        return match id.as_slice() {
            PALLET_HYPERBRIDGE_ID => Ok(Box::new(pallet_hyperbridge::Pallet::<Runtime>::default())),
            _ => Err(Error::ModuleNotFound(id)),
        };
    }
}
```

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2025 Polytope Labs.
