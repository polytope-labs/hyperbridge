//! EvmHost contract bindings generated with alloy sol! macro

use alloy_sol_macro::sol;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    EvmHost,
    "../out/EvmHost.sol/EvmHost.json"
);

pub use EvmHost::*;
