//! ERC20 contract bindings generated with alloy sol! macro

use alloy_sol_macro::sol;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, PartialEq, Eq)]
    ERC20,
    "../out/ERC20.sol/ERC20.json"
);

pub use ERC20::*;
