#![allow(clippy::all, ambiguous_glob_reexports)]
#![allow(non_snake_case)]

use alloy_sol_macro::sol;

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	#[derive(Debug, PartialEq, Eq)]
	ArbGasInfo,
	"abis/ArbGasInfo.json"
);

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	#[derive(Debug, PartialEq, Eq)]
	OvmGasPriceOracle,
	"abis/OVM_gasPriceOracle.json"
);

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	#[derive(Debug, PartialEq, Eq)]
	Erc20,
	"abis/ERC20.json"
);

pub mod arb_gas_info {
	pub use super::ArbGasInfo::*;
}

pub mod ovm_gas_price_oracle {
	pub use super::OvmGasPriceOracle::*;
}

pub mod erc_20 {
	pub use super::Erc20::*;
}

pub use ismp_solidity_abi::{beefy::*, evm_host::*, handler::*, ping_module::*};
