use crate::abi::{arb_gas_info::ArbGasInfo, ovm_gas_price_oracle::OVM_gasPriceOracle};
use anyhow::{anyhow, Error};
use ethers::{
	prelude::{abigen, Bytes, Middleware, Provider},
	providers::Http,
	types::H160,
	utils::parse_units,
};
use geth_primitives::{new_u256, old_u256};
use hex_literal::hex;
use ismp::host::StateMachine;
use ismp_solidity_abi::evm_host::EvmHost;
use primitive_types::U256;
use reqwest::{header::HeaderMap, Client};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{de::DeserializeOwned, Deserialize};
use std::{fmt::Debug, sync::Arc, time::Duration};
use tesseract_primitives::Cost;

abigen!(
	IUniswapV2Router,
	r#"[
        function getAmountsIn(uint amountOut, address[] memory path) public view returns (uint[] memory amounts)
        function WETH() external pure returns (address)
    ]"#;
	IERC20,
	r#"[
        function decimals() external view returns (uint8)
    ]"#
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

pub fn read_op_registry() -> Result<Vec<superchain_registry::Chain>, anyhow::Error> {
	let chain_list = include_str!("../op-registry/chainList.json");
	let chains = serde_json::from_str::<Vec<superchain_registry::Chain>>(chain_list)?;
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

/// Function gets current gas price (for execution) in wei and return the equivalent in USD,
pub async fn get_current_gas_cost_in_usd(
	chain: StateMachine,
	ismp_host_address: H160,
	client: Arc<Provider<Http>>,
) -> Result<GasBreakdown, Error> {
	let mut gas_price_cost = U256::zero();
	let mut gas_price = U256::zero();
	let mut unit_wei = U256::zero();

	match chain {
		StateMachine::Evm(inner_evm) => {
			match inner_evm {
				chain_id if is_orbit_chain(chain_id) => {
					let node_gas_price = client.get_gas_price().await?;
					let arb_gas_info_contract = ArbGasInfo::new(ARB_GAS_INFO, client.clone());
					let (.., oracle_gas_price) = arb_gas_info_contract.get_prices_in_wei().await?;
					// needed because of ether-rs and polkadot-sdk incompatibility
					gas_price = new_u256(std::cmp::max(node_gas_price, oracle_gas_price)); // minimum gas price is
					                                                        // 0.1 Gwei
				},
				// op stack chains
				chain_id if is_op_stack(chain_id) => {
					let node_gas_price = client.get_gas_price().await?;
					let ovm_gas_price_oracle =
						OVM_gasPriceOracle::new(OP_GAS_ORACLE, client.clone());
					let ovm_gas_price = ovm_gas_price_oracle.gas_price().await?;
					// needed because of ether-rs and polkadot-sdk incompatibility
					gas_price = new_u256(std::cmp::max(node_gas_price, ovm_gas_price)); // minimum gas price is 0.1
					                                                     // Gwei
				},
				_ => {
					gas_price = new_u256(client.get_gas_price().await?);
				},
			}
		},
		chain => Err(anyhow!("Unknown chain: {chain:?}"))?,
	}
	let token_usd = get_price_from_uniswap_router(ismp_host_address, client)
		.await
		.unwrap_or(U256::zero());

	unit_wei = get_cost_of_one_wei(token_usd);
	gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;

	log::debug!(
		"Returned gas price for {chain:?}: {} Gwei",
		ethers::utils::format_units(old_u256(gas_price), "gwei").unwrap()
	);

	Ok(GasBreakdown { gas_price, gas_price_cost: gas_price_cost.into(), unit_wei_cost: unit_wei })
}

fn get_cost_of_one_wei(eth_usd: U256) -> U256 {
	let old: ethers::types::U256 =
		parse_units(1u64.to_string(), "ether").expect("Cannot overflow").into();
	// needed because of ether-rs and polkadot-sdk incompatibility
	let eth_to_wei: U256 = new_u256(old);
	eth_usd / eth_to_wei
}

async fn get_price_from_uniswap_router(
	ismp_host: H160,
	client: Arc<Provider<Http>>,
) -> Result<U256, Error> {
	let host = EvmHost::new(ismp_host, client.clone());
	let params = host.host_params().call().await?;

	// There are no uniswap pool on testnet, return 1 usd as native token value
	if params.hyperbridge.0.starts_with(b"KUSAMA") {
		return Ok(U256::from(10).pow(U256::from(27)));
	}

	let uniswap_v2 = params.uniswap_v2;
	let fee_token = params.fee_token;

	if uniswap_v2 == H160::zero() {
		return Err(anyhow!("Uniswap V2 Router not configured in Host Params"));
	}

	let router = IUniswapV2Router::new(uniswap_v2, client.clone());
	let native_token = router.weth().call().await?;

	let fee_token_contract = IERC20::new(fee_token, client.clone());
	let fee_token_decimals = fee_token_contract.decimals().call().await?;

	let native_token_contract = IERC20::new(native_token, client.clone());
	let native_decimals = native_token_contract.decimals().call().await?;

	let path = vec![fee_token, native_token];
	let amount_out = U256::from(10).pow(U256::from(native_decimals));

	let amounts = router.get_amounts_in(old_u256(amount_out), path).call().await?;

	if amounts.len() < 2 {
		return Err(anyhow!("Invalid amounts returned from Uniswap V2 Router"));
	}

	let amount_stable = new_u256(amounts[0]);

	let target_decimals = 27;
	Ok(amount_stable * U256::from(10).pow(U256::from(target_decimals - fee_token_decimals as u32)))
}

/// Returns the L2 data cost for a given transaction data in usd
pub async fn get_l2_data_cost(
	rlp_tx: Bytes,
	chain: StateMachine,
	client: Arc<Provider<Http>>,
	// Unit wei cost in 27 decimals
	unit_wei_cost: U256,
) -> Result<Cost, anyhow::Error> {
	let mut data_cost = U256::zero();
	match chain {
		StateMachine::Evm(inner_evm) => match inner_evm {
			id if is_op_stack(id) => {
				let ovm_gas_price_oracle = OVM_gasPriceOracle::new(OP_GAS_ORACLE, client);
				// needed because of ether-rs and polkadot-sdk incompatibility
				let data_cost_bytes: U256 =
					new_u256(ovm_gas_price_oracle.get_l1_fee(rlp_tx).await?); // this is in wei
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
	use crate::gas_oracle::{
		get_current_gas_cost_in_usd,
		get_l2_data_cost, ARBITRUM_SEPOLIA_CHAIN_ID, BSC_CHAIN_ID,
		ETHEREUM_CHAIN_ID, GNOSIS_CHAIN_ID, OPTIMISM_SEPOLIA_CHAIN_ID, POLYGON_CHAIN_ID,
		POLYGON_TESTNET_CHAIN_ID, SEPOLIA_CHAIN_ID,
	};
	use ethers::{prelude::Provider, providers::Http};
	use ismp::host::StateMachine;
	use std::sync::Arc;

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_ethereum_mainnet() {
		dotenv::dotenv().ok();
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		let ismp_host = std::env::var("ETHEREUM_ISMP_HOST")
			.expect("ETHEREUM_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(ETHEREUM_CHAIN_ID),
			ismp_host,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Eth mainnet: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_sepolia() {
		dotenv::dotenv().ok();
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		let ismp_host = std::env::var("SEPOLIA_ISMP_HOST")
			.expect("SEPOLIA_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(SEPOLIA_CHAIN_ID),
			ismp_host,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Sepolia: {:?}", ethereum_gas_cost_in_usd);
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
		let provider = Provider::<Http>::try_from(rpc_uri).unwrap();
		// Client is unused in this test
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(POLYGON_CHAIN_ID),
			ismp_host,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Polygon Mainnet: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_gnosis_testnet() {
		dotenv::dotenv().ok();
		let ethereum_rpc_uri = std::env::var("CHIADO_URL").expect("get url is not set in .env.");
		let ismp_host = std::env::var("CHIADO_ISMP_HOST")
			.expect("CHIADO_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(GNOSIS_CHAIN_ID),
			ismp_host,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Gnosis Mainnet: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_polygon_testnet() {
		dotenv::dotenv().ok();
		let ismp_host = std::env::var("POLYGON_TESTNET_ISMP_HOST")
			.expect("POLYGON_TESTNET_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(POLYGON_TESTNET_CHAIN_ID),
			ismp_host,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Polygon Testnet: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_bsc_mainnet() {
		dotenv::dotenv().ok();
		let rpc_uri = std::env::var("BSC_URL").expect("BSC_URL is not set in .env");
		let ismp_host = std::env::var("BSC_ISMP_HOST")
			.expect("BSC_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd =
			get_current_gas_cost_in_usd(StateMachine::Evm(BSC_CHAIN_ID), ismp_host, client.clone())
				.await
				.unwrap();

		println!("Ethereum Gas Cost Bsc: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_arbitrum_mainnet() {
		dotenv::dotenv().ok();
		let ethereum_rpc_uri = std::env::var("ARB_URL").expect("arb url is not set in .env.");
		let ismp_host = std::env::var("ARB_ISMP_HOST")
			.expect("ARB_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(ARBITRUM_SEPOLIA_CHAIN_ID),
			ismp_host,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Arbitrum: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_optimism_base_mainnet() {
		dotenv::dotenv().ok();
		let ismp_host = std::env::var("OP_ISMP_HOST")
			.expect("OP_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		let ethereum_rpc_uri = std::env::var("OP_URL").expect("op url is not set in .env.");
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(OPTIMISM_SEPOLIA_CHAIN_ID),
			ismp_host,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Optimism: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_l2_data_cost_optimism_base_mainnet() {
		dotenv::dotenv().ok();
		let ismp_host = std::env::var("OP_ISMP_HOST")
			.expect("OP_ISMP_HOST is not set in .env")
			.parse()
			.unwrap();
		let ethereum_rpc_uri = std::env::var("OP_URL").expect("op url is not set in .env.");
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());
		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(OPTIMISM_SEPOLIA_CHAIN_ID),
			ismp_host,
			client.clone(),
		)
		.await
		.unwrap();
		let data_cost = get_l2_data_cost(
			vec![1u8; 32].into(),
			StateMachine::Evm(OPTIMISM_SEPOLIA_CHAIN_ID),
			client.clone(),
			ethereum_gas_cost_in_usd.unit_wei_cost,
		)
		.await
		.unwrap();

		println!("Data Cost Optimism: {:?}", data_cost);
	}
}
