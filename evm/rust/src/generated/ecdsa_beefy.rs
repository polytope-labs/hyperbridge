//! Beefy contract bindings generated with alloy sol! macro.
//!
//! `#[sol(rpc)]` makes `sol!` also emit `ContractInstance` / provider-backed call
//! bindings alongside the plain ABI types. Those bindings depend on `alloy-contract`,
//! `alloy-provider`, `alloy-network` and `alloy-transport`, all of which are std-only.
//! `#[sol(...)]` is recognised by the `sol!` macro at expansion time, which runs after
//! `cfg_attr` is resolved — so we pick between two invocations with `#[cfg]` instead.

use alloy_sol_macro::sol;

#[cfg(feature = "std")]
sol!(
	#[allow(missing_docs)]
	#[sol(rpc, ignore_unlinked)]
	#[derive(Debug, PartialEq, Eq)]
	Beefy,
	"abi/EcdsaBeefy.json"
);

#[cfg(not(feature = "std"))]
sol!(
	#[allow(missing_docs)]
	#[sol(ignore_unlinked)]
	#[derive(Debug, PartialEq, Eq)]
	Beefy,
	"abi/EcdsaBeefy.json"
);

pub use Beefy::*;
