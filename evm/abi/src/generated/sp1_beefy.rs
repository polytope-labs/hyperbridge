//! SP1Beefy contract bindings generated with alloy sol! macro

use alloy_sol_macro::sol;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc, ignore_unlinked)]
    #[derive(Debug, PartialEq, Eq)]
    SP1Beefy,
    "../out/SP1Beefy.sol/SP1Beefy.json"
);

pub use SP1Beefy::*;
