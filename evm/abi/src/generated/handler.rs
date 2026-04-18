//! Handler contract bindings generated with alloy sol! macro.
//!
//! See `beefy.rs` for why the std / no_std variants are distinct `sol!` invocations.

use alloy_sol_macro::sol;

#[cfg(feature = "std")]
sol!(
	#[allow(missing_docs)]
	#[sol(rpc, ignore_unlinked)]
	#[derive(Debug, PartialEq, Eq)]
	Handler,
	"../out/HandlerV1.sol/HandlerV1.json"
);

#[cfg(not(feature = "std"))]
sol!(
	#[allow(missing_docs)]
	#[sol(ignore_unlinked)]
	#[derive(Debug, PartialEq, Eq)]
	Handler,
	"../out/HandlerV1.sol/HandlerV1.json"
);

pub use Handler::*;
