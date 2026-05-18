//! ABI types for BandwidthManager.sol.
//!
//! These structs are used internally by the contract (via `abi.encode`/`abi.decode`)
//! but don't appear in the contract's external function signatures, so they cannot
//! be generated from the compiled ABI JSON. Instead they are declared directly to
//! match the Solidity definitions.

use alloy_sol_macro::sol;

sol! {
	#![sol(all_derives)]

	/// Matches `BandwidthPurchaseMsg` in `evm/src/apps/BandwidthManager.sol`.
	struct BandwidthPurchaseMsg {
		bytes app;
		uint256 tier;
		uint256 months;
		bytes chain;
	}

	/// Matches `Tier` in `evm/src/apps/BandwidthManager.sol`.
	struct Tier {
		uint256 tier;
		uint256 price;
	}

	/// Matches `Withdrawal` in `evm/src/apps/BandwidthManager.sol`.
	struct Withdrawal {
		address token;
		address beneficiary;
		uint256 amount;
	}
}
