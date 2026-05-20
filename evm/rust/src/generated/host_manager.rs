//! HostManager contract bindings generated with alloy sol! macro.
//!
//! See `beefy.rs` for why the std / no_std variants are distinct `sol!` invocations.

use alloy_sol_macro::sol;

#[cfg(feature = "std")]
sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	#[derive(Debug, PartialEq, Eq)]
	HostManager,
	"abi/HostManager.json"
);

#[cfg(not(feature = "std"))]
sol!(
	#[allow(missing_docs)]
	#[derive(Debug, PartialEq, Eq)]
	HostManager,
	"abi/HostManager.json"
);

pub use HostManager::*;
