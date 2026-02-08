//! HostManager contract bindings generated with alloy sol! macro

use alloy_sol_macro::sol;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    HostManager,
    "../out/HostManager.sol/HostManager.json"
);

pub use HostManager::*;
