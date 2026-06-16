//! Bindings for `BeefyConsensusClientTest`, the Solidity test-helper contract that
//! exposes the otherwise-internal BEEFY codec functions (`EncodeCommitment`,
//! `EncodeLeaf`, `DecodeHeader`) as public entrypoints so Rust integration tests can
//! drive them directly. Gated behind the `test-helpers` feature so production builds
//! don't pull in test-only ABIs.

use alloy_sol_macro::sol;

#[cfg(feature = "std")]
sol!(
	#[allow(missing_docs)]
	#[sol(rpc, ignore_unlinked)]
	#[derive(Debug, PartialEq, Eq)]
	BeefyConsensusClientTest,
	"abi/BeefyConsensusClientTest.json"
);

#[cfg(not(feature = "std"))]
sol!(
	#[allow(missing_docs)]
	#[sol(ignore_unlinked)]
	#[derive(Debug, PartialEq, Eq)]
	BeefyConsensusClientTest,
	"abi/BeefyConsensusClientTest.json"
);

pub use BeefyConsensusClientTest::*;
