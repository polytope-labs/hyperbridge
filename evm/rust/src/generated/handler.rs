//! Handler contract bindings generated with alloy sol! macro.
//!
//! See `beefy.rs` for why the std / no_std variants are distinct `sol!` invocations.

pub mod handler_v2 {
	use alloy_sol_macro::sol;

	#[cfg(feature = "std")]
	sol!(
		#[allow(missing_docs)]
		#[sol(rpc, ignore_unlinked)]
		#[derive(Debug, PartialEq, Eq)]
		HandlerV2,
		"abi/HandlerV2.json"
	);

	#[cfg(not(feature = "std"))]
	sol!(
		#[allow(missing_docs)]
		#[sol(ignore_unlinked)]
		#[derive(Debug, PartialEq, Eq)]
		HandlerV2,
		"abi/HandlerV2.json"
	);

	pub use HandlerV2::*;
	// `HandlerV2Instance` is only generated when `#[sol(rpc, ...)]` is enabled —
	// i.e. in `std` builds. no_std consumers only need the call-type selectors,
	// not the rpc helpers.
	#[cfg(feature = "std")]
	pub use HandlerV2::HandlerV2Instance;
}

// Re-export HandlerV2 types at the handler module level for backwards compatibility
pub use handler_v2::HandlerV2::*;
