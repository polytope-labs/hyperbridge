//! Handler contract bindings generated with alloy sol! macro

use alloy_sol_macro::sol;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc, ignore_unlinked)]
    #[derive(Debug, PartialEq, Eq)]
    Handler,
    "../out/HandlerV1.sol/HandlerV1.json"
);

pub use Handler::*;
