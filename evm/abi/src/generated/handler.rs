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

// HandlerV2 lives in its own module to avoid the shared `MerklePatricia`
// type (and other library symbols) being generated twice in the same scope
// by two independent `sol!` invocations.
pub mod handler_v2 {
	use alloy_sol_macro::sol;

	#[cfg(feature = "std")]
	sol!(
		#[allow(missing_docs)]
		#[sol(rpc, ignore_unlinked)]
		#[derive(Debug, PartialEq, Eq)]
		HandlerV2,
		"../out/HandlerV2.sol/HandlerV2.json"
	);

	#[cfg(not(feature = "std"))]
	sol!(
		#[allow(missing_docs)]
		#[sol(ignore_unlinked)]
		#[derive(Debug, PartialEq, Eq)]
		HandlerV2,
		"../out/HandlerV2.sol/HandlerV2.json"
	);

	pub use HandlerV2::{batchCallCall, HandlerV2Instance};
}
