//! Aggregate public key BLS BEEFY contract bindings generated with alloy sol! macro.
//!
//! See [`crate::generated::ecdsa_beefy`] for why the two `sol!` invocations are picked between
//! with `#[cfg]` rather than `cfg_attr`.

use alloy_sol_macro::sol;

#[cfg(feature = "std")]
sol!(
	#[allow(missing_docs)]
	#[sol(rpc, ignore_unlinked)]
	#[derive(Debug, PartialEq, Eq)]
	BlsApkBeefy,
	"abi/BlsApkBeefy.json"
);

#[cfg(not(feature = "std"))]
sol!(
	#[allow(missing_docs)]
	#[sol(ignore_unlinked)]
	#[derive(Debug, PartialEq, Eq)]
	BlsApkBeefy,
	"abi/BlsApkBeefy.json"
);
