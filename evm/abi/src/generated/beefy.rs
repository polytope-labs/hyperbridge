//! Beefy contract bindings generated with alloy sol! macro

use alloy_sol_macro::sol;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc, ignore_unlinked)]
    #[derive(Debug, PartialEq, Eq)]
    Beefy,
    "../out/BeefyV1.sol/BeefyV1.json"
);

pub use Beefy::*;
