use crate::{
	abi::{arb_gas_info::ArbGasInfoInstance, ovm_gas_price_oracle::OvmGasPriceOracleInstance},
	AlloyProvider,
};
use alloy::{
	primitives::{Address, Bytes as AlloyBytes, U256 as AlloyU256},
	providers::Provider,
};
use alloy_sol_macro::sol;
use anyhow::{anyhow, Error};
use hex_literal::hex;
use ismp::host::StateMachine;
use ismp_solidity_abi::evm_host::EvmHostInstance;
use primitive_types::U256;
use serde::Deserialize;
use sp_core::H160;
use std::{fmt::Debug, sync::Arc};
use tesseract_primitives::Cost;

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	#[derive(Debug, PartialEq, Eq)]
	interface IUniswapV2Router {
		function getAmountsIn(uint256 amountOut, address[] memory path) public view returns (uint256[] memory amounts);
		function WETH() external pure returns (address);
	}
);

sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	#[derive(Debug, PartialEq, Eq)]
	interface IERC20 {
		function decimals() external view returns (uint8);
	}
);

const ARB_GAS_INFO: [u8; 20] = hex!("000000000000000000000000000000000000006c");
const OP_GAS_ORACLE: [u8; 20] = hex!("420000000000000000000000000000000000000F");

// Supported EVM chains
// Mainnets
pub const ARBITRUM_CHAIN_ID: u32 = 42161;
pub const ETHEREUM_CHAIN_ID: u32 = 1;
pub const BSC_CHAIN_ID: u32 = 56;
pub const POLYGON_CHAIN_ID: u32 = 137;
pub const GNOSIS_CHAIN_ID: u32 = 100;
pub const CRONOS_CHAIN_ID: u32 = 25;
pub const SEI_CHAIN_ID: u32 = 1329;
pub const INJECTIVE_CHAIN_ID: u32 = 1440; // Not launched yet

// Testnets
pub const ARBITRUM_SEPOLIA_CHAIN_ID: u32 = 421614;
pub const OPTIMISM_SEPOLIA_CHAIN_ID: u32 = 11155420;
pub const BASE_SEPOLIA_CHAIN_ID: u32 = 84532;
pub const SEPOLIA_CHAIN_ID: u32 = 11155111;
pub const BSC_TESTNET_CHAIN_ID: u32 = 97;
pub const POLYGON_TESTNET_CHAIN_ID: u32 = 80002;
pub const CHIADO_CHAIN_ID: u32 = 10200;
pub const CRONOS_TESTNET_CHAIN_ID: u32 = 338;
pub const SEI_TESTNET_CHAIN_ID: u32 = 1328;
pub const INJECTIVE_TESTNET_CHAIN_ID: u32 = 1439;

pub fn is_orbit_chain(id: u32) -> bool {
	[ARBITRUM_CHAIN_ID, ARBITRUM_SEPOLIA_CHAIN_ID].contains(&id)
}

/// Minimal struct for deserializing OP Stack chain data from chainList.json
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpStackChain {
	pub chain_id: u64,
}

pub fn read_op_registry() -> Result<Vec<OpStackChain>, anyhow::Error> {
	let chain_list = include_str!("../op-registry/chainList.json");
	let chains = serde_json::from_str::<Vec<OpStackChain>>(chain_list)?;
	Ok(chains)
}

fn is_op_stack(id: u32) -> bool {
	let chain_ids = read_op_registry()
		.expect("Failed to read chain list")
		.iter()
		.map(|chain| chain.chain_id)
		.collect::<Vec<_>>();
	chain_ids.contains(&(id as u64))
}

#[derive(Debug)]
pub struct GasBreakdown {
	/// Gas price in wei
	pub gas_price: U256,
	/// Gas price cost
	pub gas_price_cost: Cost,
	/// Unit wei cost in 27 decimals
	pub unit_wei_cost: U256,
}

fn alloy_u256_to_primitive(val: AlloyU256) -> U256 {
	U256::from_little_endian(&val.to_le_bytes::<32>())
}

fn primitive_to_alloy_u256(val: U256) -> AlloyU256 {
	AlloyU256::from_limbs(val.0)
}

/// Function gets current gas price (for execution) in wei and return the equivalent in USD,
pub async fn get_current_gas_cost_in_usd(
	chain: StateMachine,
	ismp_host_address: H160,
	client: Arc<AlloyProvider>,
) -> Result<GasBreakdown, Error> {
	let mut gas_price = U256::zero();

	match chain {
		StateMachine::Evm(inner_evm) => {
			match inner_evm {
				chain_id if is_orbit_chain(chain_id) => {
					let node_gas_price = client.get_gas_price().await?;
					let arb_gas_info_contract =
						ArbGasInfoInstance::new(Address::from_slice(&ARB_GAS_INFO), client.clone());
					let prices = arb_gas_info_contract.getPricesInWei().call().await?;
					let oracle_gas_price = prices._5; // Last return value is L2 gas price
					gas_price = alloy_u256_to_primitive(std::cmp::max(
						AlloyU256::from(node_gas_price),
						oracle_gas_price,
					));
				},
				// op stack chains
				chain_id if is_op_stack(chain_id) => {
					let node_gas_price = client.get_gas_price().await?;
					let ovm_gas_price_oracle = OvmGasPriceOracleInstance::new(
						Address::from_slice(&OP_GAS_ORACLE),
						client.clone(),
					);
					let ovm_gas_price = ovm_gas_price_oracle.gasPrice().call().await?;
					gas_price = alloy_u256_to_primitive(std::cmp::max(
						AlloyU256::from(node_gas_price),
						ovm_gas_price,
					));
				},
				_ => {
					gas_price =
						alloy_u256_to_primitive(AlloyU256::from(client.get_gas_price().await?));
				},
			}
		},
		chain => Err(anyhow!("Unknown chain: {chain:?}"))?,
	}
	let token_usd = get_price_from_uniswap_router(ismp_host_address, client).await?;

	let unit_wei = get_cost_of_one_wei(token_usd);
	let gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;

	let gas_price_gwei = gas_price / U256::from(1_000_000_000u64);
	log::debug!("Returned gas price for {chain:?}: {} Gwei", gas_price_gwei);

	Ok(GasBreakdown { gas_price, gas_price_cost: gas_price_cost.into(), unit_wei_cost: unit_wei })
}

fn get_cost_of_one_wei(eth_usd: U256) -> U256 {
	// 1 ether = 10^18 wei
	let eth_to_wei: U256 = U256::from(10).pow(U256::from(18));
	eth_usd / eth_to_wei
}

async fn get_price_from_uniswap_router(
	ismp_host: H160,
	client: Arc<AlloyProvider>,
) -> Result<U256, Error> {
	let host = EvmHostInstance::new(Address::from_slice(&ismp_host.0), client.clone());
	let params = host.hostParams().call().await?;

	// There are no uniswap pool on testnet, return 1 usd as native token value
	if params.hyperbridge.0.starts_with(b"KUSAMA") {
		return Ok(U256::from(10).pow(U256::from(27)));
	}

	let uniswap_v2 = H160::from_slice(params.uniswapV2.as_slice());
	let fee_token = Address::from_slice(params.feeToken.as_slice());

	if uniswap_v2 == H160::zero() {
		return Err(anyhow!("Uniswap V2 Router not configured in Host Params"));
	}

	let router =
		IUniswapV2Router::IUniswapV2RouterInstance::new(params.uniswapV2, client.clone());
	let native_token = router.WETH().call().await?;

	let fee_token_contract = IERC20::IERC20Instance::new(fee_token, client.clone());
	let fee_token_decimals = fee_token_contract.decimals().call().await?;

	let native_token_contract = IERC20::IERC20Instance::new(native_token, client.clone());
	let native_decimals = native_token_contract.decimals().call().await?;

	let path = vec![fee_token, native_token];
	let amount_out = primitive_to_alloy_u256(U256::from(10).pow(U256::from(native_decimals)));

	let amounts = router.getAmountsIn(amount_out, path).call().await?;

	if amounts.is_empty() {
		return Err(anyhow!("Invalid amounts returned from Uniswap V2 Router"));
	}

	let amount_stable = alloy_u256_to_primitive(amounts[0]);

	let target_decimals = 27;
	Ok(amount_stable * U256::from(10).pow(U256::from(target_decimals - fee_token_decimals as u32)))
}

/// Returns the L2 data cost for a given transaction data in usd
pub async fn get_l2_data_cost(
	rlp_tx: AlloyBytes,
	chain: StateMachine,
	client: Arc<AlloyProvider>,
	// Unit wei cost in 27 decimals
	unit_wei_cost: U256,
) -> Result<Cost, anyhow::Error> {
	let mut data_cost = U256::zero();
	match chain {
		StateMachine::Evm(inner_evm) => match inner_evm {
			id if is_op_stack(id) => {
				let ovm_gas_price_oracle =
					OvmGasPriceOracleInstance::new(Address::from_slice(&OP_GAS_ORACLE), client);
				let data_cost_bytes =
					alloy_u256_to_primitive(ovm_gas_price_oracle.getL1Fee(rlp_tx).call().await?);
				data_cost = data_cost_bytes * unit_wei_cost
			},

			_ => {},
		},
		_ => Err(anyhow!("Unknown chain: {chain:?}"))?,
	}

	Ok(convert_27_decimals_to_18_decimals(data_cost)?.into())
}

/// Use this to convert a value in 27 decimals back to the standard erc20 18 decimals
pub fn convert_27_decimals_to_18_decimals(value: U256) -> Result<U256, Error> {
	let val_as_str = value.to_string();
	let characters = val_as_str.chars().collect::<Vec<_>>();
	// we are discarding the last 9 characters
	let significant_figs =
		characters[..characters.len().saturating_sub(9)].into_iter().collect::<String>();
	let parsed = U256::from_dec_str(&significant_figs)?;
	Ok(parsed)
}

#[cfg(test)]
mod test {
	use super::{
		get_current_gas_cost_in_usd, get_l2_data_cost, ARBITRUM_CHAIN_ID, BSC_CHAIN_ID,
		ETHEREUM_CHAIN_ID, GNOSIS_CHAIN_ID, POLYGON_CHAIN_ID,
	};
	use alloy::{primitives::Bytes, providers::RootProvider};
	use ismp::host::StateMachine;
	use std::sync::Arc;

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_ethereum_mainnet() {
		dotenv::dotenv().ok();
		let ethereum_rpc_uri =
			std::env::var("ETHEREUM_URL").expect("ethereum url is not set in .env.");
		let ismp_host = std::env::var("ETHEREUM_ISMP_HOST")
			.expect("ETHEREUM_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		let client = Arc::new(RootProvider::new_http(ethereum_rpc_uri.parse().unwrap()));

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(ETHEREUM_CHAIN_ID),
			ismp_host,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Gas Cost Eth mainnet: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_polygon_mainnet() {
		dotenv::dotenv().ok();
		let rpc_uri = std::env::var("POLYGON_URL").expect("POLYGON_URL is not set in .env");
		let ismp_host = std::env::var("POLYGON_ISMP_HOST")
			.expect("POLYGON_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		let client = Arc::new(RootProvider::new_http(rpc_uri.parse().unwrap()));

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(POLYGON_CHAIN_ID),
			ismp_host,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Gas Cost Polygon Mainnet: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_gnosis_mainnet() {
		dotenv::dotenv().ok();
		let ethereum_rpc_uri = std::env::var("GNOSIS_URL").expect("gnosis url is not set in .env.");
		let ismp_host = std::env::var("GNOSIS_ISMP_HOST")
			.expect("GNOSIS_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		let client = Arc::new(RootProvider::new_http(ethereum_rpc_uri.parse().unwrap()));

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(GNOSIS_CHAIN_ID),
			ismp_host,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Gas Cost Gnosis Mainnet: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_bsc_mainnet() {
		dotenv::dotenv().ok();
		let rpc_uri = std::env::var("BSC_MAINNET_URL").expect("BSC_MAINNET_URL is not set in .env");
		let ismp_host = std::env::var("BSC_ISMP_HOST")
			.expect("BSC_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		let client = Arc::new(RootProvider::new_http(rpc_uri.parse().unwrap()));

		let ethereum_gas_cost_in_usd =
			get_current_gas_cost_in_usd(StateMachine::Evm(BSC_CHAIN_ID), ismp_host, client.clone())
				.await
				.unwrap();

		println!("Gas Cost Bsc: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_arbitrum_mainnet() {
		dotenv::dotenv().ok();
		let ethereum_rpc_uri =
			std::env::var("ARBITRUM_MAINNET_URL").expect("arb url is not set in .env.");
		let ismp_host = std::env::var("ARB_ISMP_HOST")
			.expect("ARB_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		let client = Arc::new(RootProvider::new_http(ethereum_rpc_uri.parse().unwrap()));

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(ARBITRUM_CHAIN_ID),
			ismp_host,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Gas Cost Arbitrum: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_base_mainnet() {
		dotenv::dotenv().ok();
		let ismp_host = std::env::var("BASE_ISMP_HOST")
			.expect("BASE_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		let ethereum_rpc_uri =
			std::env::var("BASE_MAINNET_URL").expect("op url is not set in .env.");
		let client = Arc::new(RootProvider::new_http(ethereum_rpc_uri.parse().unwrap()));

		let ethereum_gas_cost_in_usd =
			get_current_gas_cost_in_usd(StateMachine::Evm(8453), ismp_host, client.clone())
				.await
				.unwrap();

		println!("Gas Cost Base: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_l2_data_cost_optimism_base_mainnet() {
		dotenv::dotenv().ok();
		let ismp_host = std::env::var("BASE_ISMP_HOST")
			.expect("BASE_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		let ethereum_rpc_uri =
			std::env::var("BASE_MAINNET_URL").expect("op url is not set in .env.");
		let client = Arc::new(RootProvider::new_http(ethereum_rpc_uri.parse().unwrap()));
		let ethereum_gas_cost_in_usd =
			get_current_gas_cost_in_usd(StateMachine::Evm(8453), ismp_host, client.clone())
				.await
				.unwrap();
		let data_cost = get_l2_data_cost(
			Bytes::from(vec![1u8; 32]),
			StateMachine::Evm(8453),
			client.clone(),
			ethereum_gas_cost_in_usd.unit_wei_cost,
		)
		.await
		.unwrap();

		println!("Data Cost Base: {:?}", data_cost);
	}
}
