use crate::abi::{arb_gas_info::ArbGasInfo, ovm_gas_price_oracle::OVM_gasPriceOracle};
use anyhow::{anyhow, Error};
use ethers::{
	prelude::{Bytes, Middleware, Provider},
	providers::Http,
	utils::parse_units,
};
use geth_primitives::{new_u256, old_u256};
use hex_literal::hex;
use ismp::host::StateMachine;
use primitive_types::U256;
use reqwest::{header::HeaderMap, Client};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{de::DeserializeOwned, Deserialize};
use std::{fmt::Debug, sync::Arc, time::Duration};
use tesseract_primitives::Cost;

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct GasResult {
	pub safe_gas_price: String,
	#[allow(dead_code)]
	pub fast_gas_price: String,
	pub usd_price: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GasResultEthereum {
	pub safe_gas_price: String,
	pub fast_gas_price: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct EthPriceResult {
	pub ethusd: String,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct GasResponse {
	pub result: GasResult,
}

#[derive(Debug, Default, Deserialize)]
pub struct GasResponseEthereum {
	pub result: GasResultEthereum,
}

#[derive(Debug, Default, Deserialize)]
pub struct EthPriceResponse {
	pub result: EthPriceResult,
}

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
	api_keys: &str,
	client: Arc<Provider<Http>>,
) -> Result<GasBreakdown, Error> {
	let mut gas_price_cost = U256::zero();
	let mut gas_price = U256::zero();
	let mut unit_wei = U256::zero();

	match chain {
		StateMachine::Evm(inner_evm) => {
			let api = "https://api.etherscan.io/v2/api";
			// Use eth mainnet to fetch ETH price data for both testnet and mainnet
			let eth_price_uri = format!(
				"{api}?chainid={ETHEREUM_CHAIN_ID}&module=stats&action=ethprice&apikey={api_keys}"
			);

			match inner_evm {
				chain_id if is_orbit_chain(chain_id) => {
					let node_gas_price = client.get_gas_price().await?;
					let arb_gas_info_contract = ArbGasInfo::new(ARB_GAS_INFO, client);
					let (.., oracle_gas_price) = arb_gas_info_contract.get_prices_in_wei().await?;
					// needed because of ether-rs and polkadot-sdk incompatibility
					gas_price = new_u256(std::cmp::max(node_gas_price, oracle_gas_price)); // minimum gas price is 0.1 Gwei
					let response_json = get_eth_to_usd_price(&eth_price_uri).await?;
					let eth_usd = parse_to_27_decimals(&response_json.result.ethusd)?;
					unit_wei = get_cost_of_one_wei(eth_usd);
					gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
				},
				SEPOLIA_CHAIN_ID | ETHEREUM_CHAIN_ID => {
					let uri = format!("{api}?chainid={ETHEREUM_CHAIN_ID}&module=gastracker&action=gasoracle&apikey={api_keys}");
					if inner_evm == SEPOLIA_CHAIN_ID {
						gas_price = new_u256(client.get_gas_price().await?);
						let response_json = get_eth_gas_and_price(&uri, &eth_price_uri).await?;
						let eth_usd = parse_to_27_decimals(&response_json.usd_price)?;
						unit_wei = get_cost_of_one_wei(eth_usd);
						gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
					} else {
						let node_gas_price = client.get_gas_price().await?;
						// Mainnet
						let response_json = get_eth_gas_and_price(&uri, &eth_price_uri).await?;
						let oracle_gas_price =
							parse_units(response_json.safe_gas_price.to_string(), "gwei")?.into();
						// needed because of ether-rs and polkadot-sdk incompatibility
						gas_price = new_u256(std::cmp::max(node_gas_price, oracle_gas_price));
						let eth_usd = parse_to_27_decimals(&response_json.usd_price)?;
						unit_wei = get_cost_of_one_wei(eth_usd);
						gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
					};
				},
				CHIADO_CHAIN_ID | GNOSIS_CHAIN_ID => {
					let node_gas_price = client.get_gas_price().await?;
					#[derive(Debug, Deserialize, Clone, Default)]
					struct BlockscoutResponse {
						average: f32,
					}
					if CHIADO_CHAIN_ID == inner_evm {
						let uri = "https://blockscout.chiadochain.net/api/v1/gas-price-oracle";
						let response_json =
							make_request::<BlockscoutResponse>(&uri, Default::default())
								.await
								.unwrap_or_default();
						let oracle_gas_price = parse_units(response_json.average, "gwei")?.into();
						// needed because of ether-rs and polkadot-sdk incompatibility
						gas_price = new_u256(std::cmp::max(node_gas_price, oracle_gas_price));
					} else {
						let uri = "https://blockscout.com/xdai/mainnet/api/v1/gas-price-oracle";
						let response_json =
							make_request::<BlockscoutResponse>(&uri, Default::default())
								.await
								.unwrap_or_default();
						let oracle_gas_price = parse_units(response_json.average, "gwei")?.into();
						// needed because of ether-rs and polkadot-sdk incompatibility
						gas_price = new_u256(std::cmp::max(node_gas_price, oracle_gas_price));
					}
					// Gnosis uses a stable coin for gas token which means the usd is
					// equivalent to the gas price
					gas_price_cost = gas_price
				},
				INJECTIVE_CHAIN_ID | INJECTIVE_TESTNET_CHAIN_ID => {
					let node_gas_price = client.get_gas_price().await?;
					gas_price = new_u256(node_gas_price);
					let inj_usd_price = get_coingecko_price("injective-protocol").await?;
					let inj_usd = parse_to_27_decimals(&inj_usd_price)?;
					unit_wei = get_cost_of_one_wei(inj_usd);
					gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
				},
				POLYGON_CHAIN_ID | POLYGON_TESTNET_CHAIN_ID => {
					let uri = format!(
					"{api}?chainid={POLYGON_CHAIN_ID}&module=gastracker&action=gasoracle&apikey={api_keys}"
				);
					let node_gas_price = client.get_gas_price().await?;
					let response_json = make_request::<GasResponse>(&uri, Default::default())
						.await
						.unwrap_or_default();
					let oracle_gas_price =
						parse_units(response_json.result.safe_gas_price, "gwei")?.into();
					// needed because of ether-rs and polkadot-sdk incompatibility
					gas_price = if inner_evm == POLYGON_CHAIN_ID {
						new_u256(std::cmp::max(node_gas_price, oracle_gas_price))
					} else {
						new_u256(node_gas_price)
					};
					let eth_usd = parse_to_27_decimals(&response_json.result.usd_price)?;
					unit_wei = get_cost_of_one_wei(eth_usd);
					gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
				},
				BSC_CHAIN_ID | BSC_TESTNET_CHAIN_ID => {
					let uri = format!(
						"{api}?chainid={BSC_CHAIN_ID}&module=gastracker&action=gasoracle&apikey={api_keys}"
					);
					let node_gas_price = client.get_gas_price().await?;
					let response_json = make_request::<GasResponse>(&uri, Default::default())
						.await
						.unwrap_or_default();
					let oracle_gas_price =
						parse_units(response_json.result.safe_gas_price, "gwei")?.into();
					// needed because of ether-rs and polkadot-sdk incompatibility
					gas_price = if inner_evm == BSC_CHAIN_ID {
						new_u256(std::cmp::max(node_gas_price, oracle_gas_price))
					} else {
						new_u256(node_gas_price)
					};
					let eth_usd = parse_to_27_decimals(&response_json.result.usd_price)?;
					unit_wei = get_cost_of_one_wei(eth_usd);
					gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
				},
				CRONOS_CHAIN_ID | CRONOS_TESTNET_CHAIN_ID => {
					let uri = format!(
						"{api}?chainid={CRONOS_CHAIN_ID}&module=gastracker&action=gasoracle&apikey={api_keys}"
					);
					let node_gas_price = client.get_gas_price().await?;
					let response_json = make_request::<GasResponse>(&uri, Default::default())
						.await
						.unwrap_or_default();
					let oracle_gas_price =
						parse_units(response_json.result.safe_gas_price, "gwei")?.into();
					// needed because of ether-rs and polkadot-sdk incompatibility
					gas_price = if inner_evm == CRONOS_CHAIN_ID {
						new_u256(std::cmp::max(node_gas_price, oracle_gas_price))
					} else {
						new_u256(node_gas_price)
					};
					let cro_usd = parse_to_27_decimals(&response_json.result.usd_price)?;
					unit_wei = get_cost_of_one_wei(cro_usd);
					gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
				},
				SEI_CHAIN_ID | SEI_TESTNET_CHAIN_ID => {
					let uri = format!(
						"{api}?chainid={SEI_CHAIN_ID}&module=gastracker&action=gasoracle&apikey={api_keys}"
					);
					let node_gas_price = client.get_gas_price().await?;
					let response_json = make_request::<GasResponse>(&uri, Default::default())
						.await
						.unwrap_or_default();
					let oracle_gas_price =
						parse_units(response_json.result.safe_gas_price, "gwei")?.into();
					// needed because of ether-rs and polkadot-sdk incompatibility
					gas_price = if inner_evm == SEI_CHAIN_ID {
						new_u256(std::cmp::max(node_gas_price, oracle_gas_price))
					} else {
						new_u256(node_gas_price)
					};
					let sei_usd = parse_to_27_decimals(&response_json.result.usd_price)?;
					unit_wei = get_cost_of_one_wei(sei_usd);
					gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
				},
				// op stack chains
				chain_id if is_op_stack(chain_id) => {
					let node_gas_price = client.get_gas_price().await?;
					let ovm_gas_price_oracle = OVM_gasPriceOracle::new(OP_GAS_ORACLE, client);
					let ovm_gas_price = ovm_gas_price_oracle.gas_price().await?;
					// needed because of ether-rs and polkadot-sdk incompatibility
					gas_price = new_u256(std::cmp::max(node_gas_price, ovm_gas_price)); // minimum gas price is 0.1 Gwei
					let response_json = get_eth_to_usd_price(&eth_price_uri).await?;
					let eth_usd = parse_to_27_decimals(&response_json.result.ethusd)?;
					unit_wei = get_cost_of_one_wei(eth_usd);
					gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
				},
				_ => Err(anyhow!("Unknown chain: {chain:?}"))?,
			}
		},
		chain => Err(anyhow!("Unknown chain: {chain:?}"))?,
	}

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

async fn make_request<T: DeserializeOwned>(url: &str, header_map: HeaderMap) -> anyhow::Result<T> {
	// Retry a request twice in case the response does not deserialize correctly the first time
	for _ in 0..3 {
		// Retry up to 3 times with increasing intervals between attempts.
		let mut retry_policy = ExponentialBackoff::builder().build_with_max_retries(5);
		retry_policy.max_retry_interval = Duration::from_secs(3 * 60);
		let client = ClientBuilder::new(Client::new())
			.with(RetryTransientMiddleware::new_with_policy(retry_policy))
			.build();

		let res = client.get(url).headers(header_map.clone()).send().await?;
		if let Ok(response) = res.json().await {
			return Ok(response);
		}
	}

	Err(anyhow!("Failed to get response for request"))
}

pub async fn get_eth_gas_and_price(
	uri: &String,
	uri_eth_price: &String,
) -> Result<GasResult, Error> {
	let response_json = make_request::<GasResponseEthereum>(uri, Default::default()).await?;
	let eth_to_usd_response = get_eth_to_usd_price(uri_eth_price).await?;

	Ok(GasResult {
		safe_gas_price: response_json.result.safe_gas_price,
		fast_gas_price: response_json.result.fast_gas_price,
		usd_price: eth_to_usd_response.result.ethusd,
	})
}

pub async fn get_eth_to_usd_price(uri_eth_price: &String) -> Result<EthPriceResponse, Error> {
	let usd_response = make_request::<EthPriceResponse>(uri_eth_price, Default::default()).await?;
	Ok(usd_response)
}

#[derive(Debug, Deserialize)]
pub struct CoinGeckoResponse {
	#[serde(flatten)]
	pub prices: std::collections::HashMap<String, CoinGeckoPrice>,
}

#[derive(Debug, Deserialize)]
pub struct CoinGeckoPrice {
	pub usd: f64,
}

/// Fetches token price from CoinGecko API
pub async fn get_coingecko_price(coin_id: &str) -> Result<String, Error> {
	let uri =
		format!("https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd", coin_id);
	let response = make_request::<CoinGeckoResponse>(&uri, Default::default()).await?;
	let price = response
		.prices
		.get(coin_id)
		.ok_or_else(|| anyhow!("Price not found for {}", coin_id))?;
	Ok(price.usd.to_string())
}

/// 27 decimals helps preserve significant digits for small values of currency e.g 0.56756, 0.0078
pub fn parse_to_27_decimals(value: &str) -> Result<U256, Error> {
	// Split the string decimal point
	let split = value.split(".");
	let mut parts = split.into_iter().collect::<Vec<_>>();
	// Only floats or integers are valid
	if parts.len() < 1 || parts.len() > 2 {
		Err(anyhow!("Invalid value"))?
	}

	// Number of zeros to pad right
	let num_of_zeros = 27usize.saturating_sub(parts.get(1).unwrap_or(&"").len());
	let zeroes = (0..num_of_zeros).into_iter().map(|_| '0').collect::<String>();
	parts.push(zeroes.as_str());
	let mut formatted = String::new();
	parts.into_iter().for_each(|part| formatted.push_str(part));

	let parsed = U256::from_dec_str(&formatted)?;
	Ok(parsed)
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
		convert_27_decimals_to_18_decimals, get_cost_of_one_wei, get_current_gas_cost_in_usd,
		get_l2_data_cost, parse_to_27_decimals, ARBITRUM_SEPOLIA_CHAIN_ID, BSC_CHAIN_ID,
		ETHEREUM_CHAIN_ID, GNOSIS_CHAIN_ID, OPTIMISM_SEPOLIA_CHAIN_ID, POLYGON_CHAIN_ID,
		POLYGON_TESTNET_CHAIN_ID, SEPOLIA_CHAIN_ID,
	};
	use ethers::{prelude::Provider, providers::Http, utils::parse_units};
	use geth_primitives::new_u256;
	use ismp::host::StateMachine;
	use primitive_types::U256;
	use std::sync::Arc;
	use tesseract_primitives::Cost;

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_ethereum_mainnet() {
		dotenv::dotenv().ok();
		let ethereum_etherscan_api_key =
			std::env::var("ETHERSCAN_KEY").expect("Etherscan ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(ETHEREUM_CHAIN_ID),
			&ethereum_etherscan_api_key,
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
		let ethereum_etherscan_api_key =
			std::env::var("ETHERSCAN_KEY").expect("Etherscan ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(SEPOLIA_CHAIN_ID),
			&ethereum_etherscan_api_key,
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
		let ethereum_etherscan_api_key =
			std::env::var("ETHERSCAN_KEY").expect("Polygon ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(POLYGON_CHAIN_ID),
			&ethereum_etherscan_api_key,
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
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd =
			get_current_gas_cost_in_usd(StateMachine::Evm(GNOSIS_CHAIN_ID), "", client.clone())
				.await
				.unwrap();

		println!("Ethereum Gas Cost Gnosis Mainnet: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_polygon_testnet() {
		dotenv::dotenv().ok();
		let ethereum_etherscan_api_key =
			std::env::var("ETHERSCAN_KEY").expect("Polygon ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(POLYGON_TESTNET_CHAIN_ID),
			&ethereum_etherscan_api_key,
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
		let ethereum_etherscan_api_key =
			std::env::var("ETHERSCAN_KEY").expect("Polygon ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(BSC_CHAIN_ID),
			&ethereum_etherscan_api_key,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Bsc: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_arbitrum_mainnet() {
		dotenv::dotenv().ok();
		let ethereum_etherscan_api_key =
			std::env::var("ETHERSCAN_KEY").expect("Ethereum ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("ARB_URL").expect("arb url is not set in .env.");
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(ARBITRUM_SEPOLIA_CHAIN_ID),
			&ethereum_etherscan_api_key,
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
		let ethereum_etherscan_api_key =
			std::env::var("ETHERSCAN_KEY").expect("Ethereum ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("OP_URL").expect("op url is not set in .env.");
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(OPTIMISM_SEPOLIA_CHAIN_ID),
			&ethereum_etherscan_api_key,
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
		let ethereum_etherscan_api_key =
			std::env::var("ETHERSCAN_KEY").expect("Ethereum ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("OP_URL").expect("op url is not set in .env.");
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());
		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(OPTIMISM_SEPOLIA_CHAIN_ID),
			&ethereum_etherscan_api_key,
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

	#[test]
	fn test_currency_conversions() {
		// Gas price in gwei
		let gas_price = 17.0;
		let eth_usd = parse_to_27_decimals("2270.13").unwrap();
		dbg!(eth_usd);
		let wei: U256 = new_u256(parse_units(gas_price, "gwei").unwrap().into());
		let unit_wei = get_cost_of_one_wei(eth_usd);
		let gas_cost = unit_wei * wei;
		dbg!(gas_cost);

		// How much for 89k gas
		let gas_limit = U256::from(84904u64);
		let cost = convert_27_decimals_to_18_decimals(gas_limit * gas_cost).unwrap();
		dbg!(cost);
		dbg!(Cost(cost));

		let eth_usd = parse_to_27_decimals("2270.13").unwrap();
		let gas_price = 0.1;
		let wei: U256 = new_u256(parse_units(gas_price, "gwei").unwrap().into());
		let unit_wei = get_cost_of_one_wei(eth_usd);
		let gas_cost = unit_wei * wei;
		dbg!(gas_cost);

		// How much for 89k gas
		let gas_limit = U256::from(84904u64);
		let cost = convert_27_decimals_to_18_decimals(gas_limit * gas_cost).unwrap();
		dbg!(cost);
		dbg!(Cost(cost));

		// Test with even smaller usd values
		let eth_usd = parse_to_27_decimals("0.00781462").unwrap();
		dbg!(eth_usd);
		let gas_price = 74.0;
		let wei: U256 = new_u256(parse_units(gas_price, "gwei").unwrap().into());
		let unit_wei = get_cost_of_one_wei(eth_usd);
		let gas_cost = unit_wei * wei;
		dbg!(gas_cost);

		// How much for 89k gas
		let gas_limit = U256::from(84904u64);
		let cost = convert_27_decimals_to_18_decimals(gas_limit * gas_cost).unwrap();
		dbg!(cost);
		dbg!(Cost(cost));
		assert!(cost > U256::zero())
	}
}
