# Pallet Hyperbridge

Pallet hyperbridge mediates the connection between hyperbridge and substrate-based chains. This pallet provides:

 - An [`IsmpDispatcher`] implementation which collects protocol fees and commits the reciepts for these fees to child storage. Hyperbridge only accepts messages that have been paid for using this module.
 - An [`IsmpModule`] which recieves and processes requests from hyperbridge. These requests are dispatched by hyperbridge governance and may adjust fees or request payouts for both relayers and protocol revenue.

This pallet contains no calls and dispatches no requests. Substrate based chains should use this to dispatch requests that should be processed by hyperbridge.

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2024 Polytope Labs.

