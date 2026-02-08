//! PingModule contract bindings generated with alloy sol! macro

use alloy_sol_macro::sol;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    PingModule,
    "../out/PingModule.sol/PingModule.json"
);

pub use PingModule::*;
