// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! ABI codec for the purchase message from `BandwidthManager.sol`.
//! Field layout must match the Solidity struct exactly.

use alloc::{format, string::ToString, vec::Vec};
use alloy_sol_macro::sol;
use alloy_sol_types::SolType;
use core::str::{self, FromStr};
use ismp::host::StateMachine;

sol! {
	#![sol(all_derives)]

	struct BandwidthPurchaseMsgAbi {
		bytes app;
		uint256 tier;
		uint256 months;
		bytes chain;
	}

	/// One row of a `SetTiers` governance batch — must match the Sol
	/// `Tier` struct in `BandwidthManager.sol`.
	struct TierAbi {
		uint256 tier;
		uint256 price;
	}

	/// `Withdraw` payload — must match the Sol `Withdrawal` struct in
	/// `BandwidthManager.sol`.
	struct WithdrawalAbi {
		address token;
		address beneficiary;
		uint256 amount;
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PurchaseMessage {
	pub app: Vec<u8>,
	pub tier: u32,
	pub months: u32,
	pub chain: StateMachine,
}

/// `chain` is the UTF-8 form of `StateMachine::Display` (e.g.
/// `"EVM-8453"`) so EVM dapps can build it with string concat.
impl TryFrom<&[u8]> for PurchaseMessage {
	type Error = anyhow::Error;

	fn try_from(body: &[u8]) -> Result<Self, Self::Error> {
		let abi = BandwidthPurchaseMsgAbi::abi_decode(body)
			.map_err(|err| anyhow::anyhow!(format!("invalid bandwidth purchase ABI: {err:?}")))?;

		let tier: u32 = abi.tier.try_into().map_err(|_| anyhow::anyhow!("tier exceeds u32"))?;
		let months: u32 =
			abi.months.try_into().map_err(|_| anyhow::anyhow!("months exceeds u32"))?;
		if months == 0 {
			return Err(anyhow::anyhow!("months must be >= 1"));
		}
		let chain_str = str::from_utf8(&abi.chain)
			.map_err(|err| anyhow::anyhow!(format!("chain is not utf-8: {err}")))?;
		let chain = StateMachine::from_str(chain_str)
			.map_err(|err| anyhow::anyhow!(format!("invalid chain {chain_str:?}: {err}")))?;

		Ok(PurchaseMessage { app: abi.app.into(), tier, months, chain })
	}
}

impl From<&PurchaseMessage> for Vec<u8> {
	fn from(msg: &PurchaseMessage) -> Vec<u8> {
		let abi = BandwidthPurchaseMsgAbi {
			app: alloy_primitives::Bytes::from(msg.app.clone()),
			tier: alloy_primitives::U256::from(msg.tier),
			months: alloy_primitives::U256::from(msg.months),
			chain: alloy_primitives::Bytes::from(msg.chain.to_string().into_bytes()),
		};
		BandwidthPurchaseMsgAbi::abi_encode(&abi)
	}
}
