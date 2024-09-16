use crate::abi::{arb_gas_info::ArbGasInfo, ovm_gas_price_oracle::OVM_gasPriceOracle};
use anyhow::{anyhow, Error};
use ethers::{
	prelude::{Bytes, Middleware, Provider},
	providers::Http,
	utils::parse_units,
};
use frame_support::Deserialize;
use hex_literal::hex;
use ismp::host::StateMachine;
use primitive_types::{H160, U256};
use reqwest::{
	header::{HeaderMap, USER_AGENT},
	Client,
};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::de::DeserializeOwned;
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
pub const OPTIMISM_CHAIN_ID: u32 = 10;
pub const BASE_CHAIN_ID: u32 = 8453;
pub const ETHEREUM_CHAIN_ID: u32 = 1;
pub const BSC_CHAIN_ID: u32 = 56;
pub const POLYGON_CHAIN_ID: u32 = 137;
pub const GNOSIS_CHAIN_ID: u32 = 100;

// Testnets
pub const ARBITRUM_SEPOLIA_CHAIN_ID: u32 = 421614;
pub const OPTIMISM_SEPOLIA_CHAIN_ID: u32 = 11155420;
pub const BASE_SEPOLIA_CHAIN_ID: u32 = 84532;
pub const SEPOLIA_CHAIN_ID: u32 = 11155111;
pub const BSC_TESTNET_CHAIN_ID: u32 = 97;
pub const POLYGON_TESTNET_CHAIN_ID: u32 = 80002;
pub const CHIADO_CHAIN_ID: u32 = 10200;

pub fn is_orbit_chain(id: u32) -> bool {
	[ARBITRUM_CHAIN_ID, ARBITRUM_SEPOLIA_CHAIN_ID].contains(&id)
}

pub fn is_op_stack(id: u32) -> bool {
	[OPTIMISM_CHAIN_ID, OPTIMISM_SEPOLIA_CHAIN_ID, BASE_CHAIN_ID, BASE_SEPOLIA_CHAIN_ID]
		.contains(&id)
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
	gas_price_buffer: Option<u32>,
) -> Result<GasBreakdown, Error> {
	let mut gas_price_cost = U256::zero();
	let mut gas_price = U256::zero();
	let mut unit_wei = U256::zero();

	match chain {
		StateMachine::Evm(inner_evm) => {
			let api = "https://api.etherscan.io/api";
			let eth_price_uri = format!("{api}?module=stats&action=ethprice&apikey={api_keys}");

			match inner_evm {
				chain_id if is_orbit_chain(chain_id) => {
					let node_gas_price = client.get_gas_price().await?;
					let arb_gas_info_contract = ArbGasInfo::new(H160(ARB_GAS_INFO), client);
					let (.., oracle_gas_price) = arb_gas_info_contract.get_prices_in_wei().await?;
					gas_price = std::cmp::max(node_gas_price, oracle_gas_price); // minimum gas price is 0.1 Gwei
					let response_json = get_eth_to_usd_price(&eth_price_uri).await?;
					let eth_usd = parse_to_27_decimals(&response_json.result.ethusd)?;
					unit_wei = get_cost_of_one_wei(eth_usd);
					gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
				},
				SEPOLIA_CHAIN_ID | ETHEREUM_CHAIN_ID => {
					let uri = format!("{api}?module=gastracker&action=gasoracle&apikey={api_keys}");
					if inner_evm == SEPOLIA_CHAIN_ID {
						#[derive(Debug, Deserialize, Clone)]
						struct GasNow {
							standard: u128,
						}

						#[derive(Debug, Deserialize, Clone)]
						struct Response {
							data: GasNow,
						}

						// sepolia
						let data = make_request::<Response>(
							"https://sepolia.beaconcha.in/api/v1/execution/gasnow",
							Default::default(),
						)
						.await?
						.data
						.standard;
						let price = data as f64 * 1.25f64;
						let node_gas_price: U256 = client.get_gas_price().await?;
						let oracle_gas_price = U256::from(price as u128);
						gas_price = std::cmp::max(node_gas_price, oracle_gas_price);
						let response_json = get_eth_gas_and_price(&uri, &eth_price_uri).await?;
						let eth_usd = parse_to_27_decimals(&response_json.usd_price)?;
						unit_wei = get_cost_of_one_wei(eth_usd);
						gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
					} else {
						let node_gas_price: U256 = client.get_gas_price().await?;
						// Mainnet
						let response_json = get_eth_gas_and_price(&uri, &eth_price_uri).await?;
						let oracle_gas_price =
							parse_units(response_json.safe_gas_price.to_string(), "gwei")?.into();
						gas_price = std::cmp::max(node_gas_price, oracle_gas_price);
						let eth_usd = parse_to_27_decimals(&response_json.usd_price)?;
						unit_wei = get_cost_of_one_wei(eth_usd);
						gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
					};
				},
				CHIADO_CHAIN_ID | GNOSIS_CHAIN_ID => {
					let node_gas_price: U256 = client.get_gas_price().await?;
					#[derive(Debug, Deserialize, Clone)]
					struct BlockscoutResponse {
						average: f32,
					}
					if CHIADO_CHAIN_ID == inner_evm {
						let uri = "https://blockscout.chiadochain.net/api/v1/gas-price-oracle";
						let response_json =
							make_request::<BlockscoutResponse>(&uri, Default::default()).await?;
						let oracle_gas_price = parse_units(response_json.average, "gwei")?.into();
						gas_price = std::cmp::max(node_gas_price, oracle_gas_price);
					} else {
						let uri = "https://blockscout.com/xdai/mainnet/api/v1/gas-price-oracle";
						let response_json =
							make_request::<BlockscoutResponse>(&uri, Default::default()).await?;
						let oracle_gas_price = parse_units(response_json.average, "gwei")?.into();
						gas_price = std::cmp::max(node_gas_price, oracle_gas_price);
					}
					// Gnosis uses a stable coin for gas convert the gas which means the usd is
					// equivalent to the gas price
					gas_price_cost = gas_price
				},
				POLYGON_CHAIN_ID | POLYGON_TESTNET_CHAIN_ID => {
					let uri = format!(
						"https://api.polygonscan.com/api?module=gastracker&action=gasoracle&apikey={api_keys}"
					);
					if inner_evm == POLYGON_TESTNET_CHAIN_ID {
						const POLYGON_TESTNET: &'static str =
							"https://gasstation-testnet.polygon.technology/v2";

						#[derive(Debug, Deserialize, Clone)]
						#[serde(rename_all = "camelCase")]
						struct PriorityFee {
							max_priority_fee: f64,
						}

						#[derive(Debug, Deserialize, Clone)]
						#[serde(rename_all = "camelCase")]
						struct Response {
							standard: PriorityFee,
						}

						let mut header_map = HeaderMap::new();
						// Polygon gas API returns forbidden if the user agent is not set
						header_map.insert(USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse().unwrap());
						let response =
							make_request::<Response>(POLYGON_TESTNET, header_map).await?;
						let node_gas_price: U256 = client.get_gas_price().await?;
						let oracle_gas_price =
							parse_units(response.standard.max_priority_fee.to_string(), "gwei")?
								.into();
						gas_price = std::cmp::max(node_gas_price, oracle_gas_price);
						let response_json =
							make_request::<GasResponse>(&uri, Default::default()).await?;
						let eth_usd = parse_to_27_decimals(&response_json.result.usd_price)?;
						unit_wei = get_cost_of_one_wei(eth_usd);
						gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
					} else {
						// Mainnet
						let node_gas_price: U256 = client.get_gas_price().await?;
						let uri = format!(
							"https://api.polygonscan.com/api?module=gastracker&action=gasoracle&apikey={api_keys}"
						);
						let response_json =
							make_request::<GasResponse>(&uri, Default::default()).await?;
						let oracle_gas_price =
							parse_units(response_json.result.safe_gas_price.to_string(), "gwei")?
								.into();
						gas_price = std::cmp::max(node_gas_price, oracle_gas_price);
						let eth_usd = parse_to_27_decimals(&response_json.result.usd_price)?;
						unit_wei = get_cost_of_one_wei(eth_usd);
						gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
					};
				},

				BSC_CHAIN_ID | BSC_TESTNET_CHAIN_ID => {
					let uri = format!(
						"https://api.bscscan.com/api?module=gastracker&action=gasoracle&apikey={api_keys}"
					);

					if inner_evm == BSC_TESTNET_CHAIN_ID {
						gas_price = client.get_gas_price().await?;
						let response_json =
							make_request::<GasResponse>(&uri, Default::default()).await?;
						let eth_usd = parse_to_27_decimals(&response_json.result.usd_price)?;
						let unit_wei = get_cost_of_one_wei(eth_usd);
						gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
					} else {
						let node_gas_price: U256 = client.get_gas_price().await?;
						// Mainnet
						let response_json =
							make_request::<GasResponse>(&uri, Default::default()).await?;
						let oracle_gas_price =
							parse_units(response_json.result.safe_gas_price, "gwei")?.into();
						gas_price = std::cmp::max(node_gas_price, oracle_gas_price);
						let eth_usd = parse_to_27_decimals(&response_json.result.usd_price)?;
						unit_wei = get_cost_of_one_wei(eth_usd);
						gas_price_cost = convert_27_decimals_to_18_decimals(unit_wei * gas_price)?;
					};
				},
				// op stack chains
				chain_id if is_op_stack(chain_id) => {
					let node_gas_price: U256 = client.get_gas_price().await?;
					let ovm_gas_price_oracle = OVM_gasPriceOracle::new(H160(OP_GAS_ORACLE), client);
					let ovm_gas_price = ovm_gas_price_oracle.gas_price().await?;
					gas_price = std::cmp::max(ovm_gas_price, node_gas_price); // minimum gas price is 0.1 Gwei
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
		ethers::utils::format_units(gas_price, "gwei").unwrap()
	);

	let buffer = gas_price_buffer
		.map(|buffer| (U256::from(buffer) * gas_price) / U256::from(100u32))
		.unwrap_or_default();
	gas_price = gas_price + buffer;

	Ok(GasBreakdown { gas_price, gas_price_cost: gas_price_cost.into(), unit_wei_cost: unit_wei })
}

fn get_cost_of_one_wei(eth_usd: U256) -> U256 {
	let eth_to_wei: U256 = parse_units(1u64.to_string(), "ether").expect("Cannot overflow").into();
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
				let ovm_gas_price_oracle = OVM_gasPriceOracle::new(H160(OP_GAS_ORACLE), client);
				let data_cost_bytes = ovm_gas_price_oracle.get_l1_fee(rlp_tx).await?; // this is in wei
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
		get_l2_data_cost, parse_to_27_decimals, ARBITRUM_SEPOLIA_CHAIN_ID, BSC_TESTNET_CHAIN_ID,
		GNOSIS_CHAIN_ID, OPTIMISM_SEPOLIA_CHAIN_ID, POLYGON_TESTNET_CHAIN_ID, SEPOLIA_CHAIN_ID,
	};
	use ethers::{prelude::Provider, providers::Http, utils::parse_units};
	use ismp::host::StateMachine;
	use primitive_types::U256;
	use std::sync::Arc;
	use tesseract_primitives::Cost;

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_ethereum_mainnet() {
		dotenv::dotenv().ok();
		let ethereum_etherscan_api_key = std::env::var("ETHERSCAN_ETHEREUM_KEY")
			.expect("Etherscan ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(SEPOLIA_CHAIN_ID),
			&ethereum_etherscan_api_key,
			client.clone(),
			None,
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Eth mainnet: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_sepolia() {
		dotenv::dotenv().ok();
		let ethereum_etherscan_api_key = std::env::var("ETHERSCAN_ETHEREUM_KEY")
			.expect("Etherscan ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(SEPOLIA_CHAIN_ID),
			&ethereum_etherscan_api_key,
			client.clone(),
			None,
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Sepolia: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_polygon_mainnet() {
		dotenv::dotenv().ok();
		let ethereum_etherscan_api_key = std::env::var("ETHERSCAN_POLYGON_KEY")
			.expect("Polygon ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(POLYGON_TESTNET_CHAIN_ID),
			&ethereum_etherscan_api_key,
			client.clone(),
			None,
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

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(GNOSIS_CHAIN_ID),
			"",
			client.clone(),
			None,
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Gnosis Mainnet: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_polygon_testnet() {
		dotenv::dotenv().ok();
		let ethereum_etherscan_api_key = std::env::var("ETHERSCAN_POLYGON_KEY")
			.expect("Polygon ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(POLYGON_TESTNET_CHAIN_ID),
			&ethereum_etherscan_api_key,
			client.clone(),
			None,
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
			std::env::var("ETHERSCAN_BSC_KEY").expect("Polygon ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("GETH_URL").expect("get url is not set in .env.");
		// Client is unused in this test
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(BSC_TESTNET_CHAIN_ID),
			&ethereum_etherscan_api_key,
			client.clone(),
			None,
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Bsc: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_arbitrum_mainnet() {
		dotenv::dotenv().ok();
		let ethereum_etherscan_api_key = std::env::var("ETHERSCAN_ETHEREUM_KEY")
			.expect("Ethereum ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("ARB_URL").expect("arb url is not set in .env.");
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(ARBITRUM_SEPOLIA_CHAIN_ID),
			&ethereum_etherscan_api_key,
			client.clone(),
			None,
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Arbitrum: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_gas_price_optimism_base_mainnet() {
		dotenv::dotenv().ok();
		let ethereum_etherscan_api_key = std::env::var("ETHERSCAN_ETHEREUM_KEY")
			.expect("Ethereum ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("OP_URL").expect("op url is not set in .env.");
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(OPTIMISM_SEPOLIA_CHAIN_ID),
			&ethereum_etherscan_api_key,
			client.clone(),
			None,
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost Optimism: {:?}", ethereum_gas_cost_in_usd);
	}

	#[tokio::test]
	#[ignore]
	async fn get_l2_data_cost_optimism_base_mainnet() {
		dotenv::dotenv().ok();
		let ethereum_etherscan_api_key = std::env::var("ETHERSCAN_ETHEREUM_KEY")
			.expect("Ethereum ethereum key is not set in .env.");
		let ethereum_rpc_uri = std::env::var("OP_URL").expect("op url is not set in .env.");
		let provider = Provider::<Http>::try_from(ethereum_rpc_uri).unwrap();
		let client = Arc::new(provider.clone());
		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			StateMachine::Evm(OPTIMISM_SEPOLIA_CHAIN_ID),
			&ethereum_etherscan_api_key,
			client.clone(),
			None,
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
		let wei: U256 = parse_units(gas_price, "gwei").unwrap().into();
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
		let wei: U256 = parse_units(gas_price, "gwei").unwrap().into();
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
		let wei: U256 = parse_units(gas_price, "gwei").unwrap().into();
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
