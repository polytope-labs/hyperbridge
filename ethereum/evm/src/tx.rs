use crate::{EthPriceResponse, EvmClient, GasResponse, GasResponseEthereum, GasResult};
use anyhow::{anyhow, Error};
use codec::Decode;
use ethers::{
	contract::FunctionCall,
	core::k256::ecdsa,
	middleware::SignerMiddleware,
	prelude::{Provider, Wallet, Ws},
	providers::PendingTransaction,
	types::Bytes,
};
use hex_literal::hex;
use ismp::{
	host::{Ethereum, StateMachine},
	messaging::{Message, ResponseMessage, TimeoutMessage},
	router::{Request, RequestResponse, Response},
};
use ismp_rpc::MmrProof;
use ismp_solidity_abi::{
	beefy::{GetRequest, StateMachineHeight},
	handler::{
		GetResponseMessage, GetTimeoutMessage, Handler as IsmpHandler, PostRequestLeaf,
		PostRequestMessage, PostRequestTimeoutMessage, PostResponseLeaf, PostResponseMessage,
		PostResponseTimeoutMessage, Proof,
	},
};
use merkle_mountain_range::mmr_position_to_k_index;
use pallet_ismp::{primitives::SubstrateStateProof, NodesUtils};
use primitive_types::{H160, H256, U256};
use std::sync::Arc;

use crate::abi::{arb_gas_info::ArbGasInfo, ovm_gas_price_oracle::OVM_gasPriceOracle};
use tesseract_primitives::IsmpHost;

/// Use this to initialize the transaction submit queue. This pipelines transaction submission
/// eliminating race conditions.
pub async fn submit_messages<I: IsmpHost>(
	client: &EvmClient<I>,
	messages: Vec<Message>,
) -> anyhow::Result<()> {
	let calls = generate_contract_calls(client, messages, false).await?;
	for call in calls {
		match call.send().await {
			Ok(progress) => wait_for_success(progress, None).await,
			Err(err) => {
				log::error!("Error broadcasting transaction for  {err:?}");
			},
		}
	}

	Ok(())
}

async fn wait_for_success<'a>(tx: PendingTransaction<'a, Ws>, confirmations: Option<usize>) {
	if let Err(err) = tx.confirmations(confirmations.unwrap_or(1)).await {
		log::error!("Error broadcasting transaction for  {err:?}");
	}
}

/// Function generates FunctionCall(s) from a batchs of messages
pub async fn generate_contract_calls<I: IsmpHost>(
	client: &EvmClient<I>,
	messages: Vec<Message>,
	debug: bool,
) -> anyhow::Result<
	Vec<
		FunctionCall<
			Arc<SignerMiddleware<Provider<Ws>, Wallet<ecdsa::SigningKey>>>,
			SignerMiddleware<Provider<Ws>, Wallet<ecdsa::SigningKey>>,
			(),
		>,
	>,
> {
	let contract = IsmpHandler::new(client.handler, client.signer.clone());
	let ismp_host = client.ismp_host;
	let mut calls = Vec::new();

	for message in messages {
		// If we are debugging we don't want to increase the client's nonce
		let nonce = if debug { client.read_nonce().await? } else { client.get_nonce().await? };

		match message {
			Message::Consensus(msg) => {
				let gas_limit = get_chain_gas_limit(client.state_machine);
				let call = contract
					.handle_consensus(ismp_host, msg.consensus_proof.into())
					.nonce(nonce)
					.gas(gas_limit);

				calls.push(call);
			},
			Message::Request(msg) => {
				let membership_proof =
					match MmrProof::<H256>::decode(&mut msg.proof.proof.as_slice()) {
						Ok(proof) => proof,
						_ => {
							log::error!("Failed to decode membership proof");
							continue
						},
					};
				let mmr_size = NodesUtils::new(membership_proof.leaf_count).size();
				let k_and_leaf_indices = membership_proof
					.leaf_positions_and_indices
					.into_iter()
					.map(|(pos, leaf_index)| {
						let k_index = mmr_position_to_k_index(vec![pos], mmr_size)[0].1;
						(k_index, leaf_index)
					})
					.collect::<Vec<_>>();

				let mut leaves = msg
					.requests
					.into_iter()
					.zip(k_and_leaf_indices)
					.map(|(post, (k_index, leaf_index))| PostRequestLeaf {
						request: post.into(),
						index: leaf_index.into(),
						k_index: k_index.into(),
					})
					.collect::<Vec<_>>();
				leaves.sort_by_key(|leaf| leaf.index);
				let gas_limit = get_chain_gas_limit(client.state_machine);
				let post_message = PostRequestMessage {
					proof: Proof {
						height: StateMachineHeight {
							state_machine_id: {
								match msg.proof.height.id.state_id {
									StateMachine::Polkadot(id) | StateMachine::Kusama(id) =>
										id.into(),
									_ => {
										panic!("Expected polkadot or kusama state machines");
									},
								}
							},
							height: msg.proof.height.height.into(),
						},
						multiproof: membership_proof.items.into_iter().map(|node| node.0).collect(),
						leaf_count: membership_proof.leaf_count.into(),
					},
					requests: leaves,
				};

				let call = contract
					.handle_post_requests(ismp_host, post_message)
					.nonce(nonce)
					.gas(gas_limit);

				calls.push(call);
			},
			Message::Response(ResponseMessage { datagram, proof, .. }) => {
				let membership_proof = match MmrProof::<H256>::decode(&mut proof.proof.as_slice()) {
					Ok(proof) => proof,
					_ => {
						log::error!("Failed to decode membership proof");
						continue
					},
				};
				let mmr_size = NodesUtils::new(membership_proof.leaf_count).size();
				let k_and_leaf_indices = membership_proof
					.leaf_positions_and_indices
					.into_iter()
					.map(|(pos, leaf_index)| {
						let k_index = mmr_position_to_k_index(vec![pos], mmr_size)[0].1;
						(k_index, leaf_index)
					})
					.collect::<Vec<_>>();

				let call = match datagram {
					RequestResponse::Response(responses) => {
						let mut leaves = responses
							.into_iter()
							.zip(k_and_leaf_indices)
							.filter_map(|(res, (k_index, leaf_index))| match res {
								Response::Post(res) => Some(PostResponseLeaf {
									response: res.into(),
									index: leaf_index.into(),
									k_index: k_index.into(),
								}),
								_ => None,
							})
							.collect::<Vec<_>>();
						leaves.sort_by_key(|leaf| leaf.index);
						let gas_limit = get_chain_gas_limit(client.state_machine);
						let message =
							PostResponseMessage {
								proof: Proof {
									height: StateMachineHeight {
										state_machine_id: {
											match proof.height.id.state_id {
												StateMachine::Polkadot(id) |
												StateMachine::Kusama(id) => id.into(),
												_ => {
													log::error!("Expected polkadot or kusama state machines");
													continue
												},
											}
										},
										height: proof.height.height.into(),
									},
									multiproof: membership_proof
										.items
										.into_iter()
										.map(|node| node.0)
										.collect(),
									leaf_count: membership_proof.leaf_count.into(),
								},
								responses: leaves,
							};

						contract
							.handle_post_responses(ismp_host, message)
							.nonce(nonce)
							.gas(gas_limit)
					},
					RequestResponse::Request(requests) => {
						let requests = match requests
							.into_iter()
							.map(|req| {
								let get = req
									.get_request()
									.map_err(|_| anyhow!("Expected get request"))?;
								Ok(GetRequest {
									source: get.source.to_string().as_bytes().to_vec().into(),
									dest: get.dest.to_string().as_bytes().to_vec().into(),
									nonce: get.nonce,
									from: get.from.into(),
									keys: get.keys.into_iter().map(|key| key.into()).collect(),
									timeout_timestamp: get.timeout_timestamp,
									gaslimit: get.gas_limit.into(),
									height: get.height.into(),
								})
							})
							.collect::<Result<Vec<_>, Error>>()
						{
							Ok(reqs) => reqs,
							Err(err) => {
								log::error!("Failed to error {err:?}");
								continue
							},
						};

						let gas_limit = get_chain_gas_limit(client.state_machine);

						let state_proof: SubstrateStateProof =
							match codec::Decode::decode(&mut proof.proof.as_slice()) {
								Ok(proof) => proof,
								_ => {
									log::error!("Failed to decode membership proof");
									continue
								},
							};
						let message = GetResponseMessage {
							proof: state_proof
								.storage_proof
								.into_iter()
								.map(|key| key.into())
								.collect(),
							height: StateMachineHeight {
								state_machine_id: {
									match proof.height.id.state_id {
										StateMachine::Polkadot(id) | StateMachine::Kusama(id) =>
											id.into(),
										_ => {
											log::error!(
												"Expected polkadot or kusama state machines"
											);
											continue
										},
									}
								},
								height: proof.height.height.into(),
							},
							requests,
						};

						contract
							.handle_get_responses(ismp_host, message)
							.nonce(nonce)
							.gas(gas_limit)
					},
				};

				calls.push(call);
			},
			Message::Timeout(TimeoutMessage::Post { timeout_proof, requests }) => {
				let post_requests = requests
					.into_iter()
					.filter_map(|req| match req {
						Request::Post(post) => Some(post.into()),
						Request::Get(_) => None,
					})
					.collect();

				let state_proof: SubstrateStateProof =
					match codec::Decode::decode(&mut timeout_proof.proof.as_slice()) {
						Ok(proof) => proof,
						_ => {
							log::error!("Failed to decode membership proof");
							continue
						},
					};
				let message = PostRequestTimeoutMessage {
					timeouts: post_requests,
					height: StateMachineHeight {
						state_machine_id: {
							match timeout_proof.height.id.state_id {
								StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.into(),
								_ => {
									log::error!("Expected polkadot or kusama state machines");
									continue
								},
							}
						},
						height: timeout_proof.height.height.into(),
					},
					proof: state_proof.storage_proof.into_iter().map(|key| key.into()).collect(),
				};
				let gas_limit = get_chain_gas_limit(client.state_machine);
				let call = contract
					.handle_post_request_timeouts(ismp_host, message)
					.nonce(nonce)
					.gas(gas_limit);

				calls.push(call);
			},
			Message::Timeout(TimeoutMessage::PostResponse { timeout_proof, responses }) => {
				let post_responses = responses.into_iter().map(|res| res.into()).collect();

				let state_proof: SubstrateStateProof =
					match codec::Decode::decode(&mut timeout_proof.proof.as_slice()) {
						Ok(proof) => proof,
						_ => {
							log::error!("Failed to decode membership proof");
							continue
						},
					};
				let message = PostResponseTimeoutMessage {
					timeouts: post_responses,
					height: StateMachineHeight {
						state_machine_id: {
							match timeout_proof.height.id.state_id {
								StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.into(),
								_ => {
									log::error!("Expected polkadot or kusama state machines");
									continue
								},
							}
						},
						height: timeout_proof.height.height.into(),
					},
					proof: state_proof.storage_proof.into_iter().map(|key| key.into()).collect(),
				};
				let gas_limit = get_chain_gas_limit(client.state_machine);
				let call = contract
					.handle_post_response_timeouts(ismp_host, message)
					.nonce(nonce)
					.gas(gas_limit);

				calls.push(call);
			},
			Message::Timeout(TimeoutMessage::Get { requests }) => {
				let get_requests = requests
					.into_iter()
					.filter_map(|req| match req {
						Request::Get(get) => Some(GetRequest {
							source: get.source.to_string().as_bytes().to_vec().into(),
							dest: get.dest.to_string().as_bytes().to_vec().into(),
							nonce: get.nonce,
							from: get.from.into(),
							keys: get.keys.into_iter().map(|key| key.into()).collect(),
							timeout_timestamp: get.timeout_timestamp,
							gaslimit: get.gas_limit.into(),
							height: get.height.into(),
						}),
						_ => None,
					})
					.collect();

				let message = GetTimeoutMessage { timeouts: get_requests };
				let gas_limit = get_chain_gas_limit(client.state_machine);
				let call = contract
					.handle_get_request_timeouts(ismp_host, message)
					.nonce(nonce)
					.gas(gas_limit);

				calls.push(call);
			},
			_ => {
				log::debug!(target: "tesseract", "Message handler not implemented in solidity abi")
			},
		}
	}

	Ok(calls)
}

const TESTNET_CHAIN_IDS: [u64; 6] = [11155111u64, 421614, 84532, 11155420, 80001, 97];
const ARB_GAS_INFO: [u8; 20] = hex!("000000000000000000000000000000000000006c");
const OP_GAS_ORACLE: [u8; 20] = hex!("420000000000000000000000000000000000000F");

/// Function gets current gas price (for execution) in Gwei and return the equivalent in USD,
/// returns (gas_cost_in_usd, native_in_usd)
pub async fn get_current_gas_cost_in_usd(
	chain_id: u64,
	chain: StateMachine,
	api_keys: &String,
	client: Arc<Provider<Ws>>,
) -> Result<(U256, U256), Error> {
	let mut gas_cost_in_usd = U256::zero();
	let mut native_in_usd = U256::zero();

	match chain {
		StateMachine::Ethereum(inner_evm) => {
			let (uri, eth_price_uri) = if TESTNET_CHAIN_IDS.contains(&chain_id) {
				let api = "https://api-sepolia.etherscan.com/api";
				// Sepolia chains
				let uri = format!("{api}?module=gastracker&action=gasoracle&apikey={api_keys}");
				let eth_price_uri = format!("{api}?module=stats&action=ethprice&apikey={api_keys}");
				(uri, eth_price_uri)
			} else {
				let api = "https://api.etherscan.com/api";
				// Mainnet
				let uri = format!("{api}?module=gastracker&action=gasoracle&apikey={api_keys}");
				let eth_price_uri = format!("{api}?module=stats&action=ethprice&apikey={api_keys}");
				(uri, eth_price_uri)
			};

			match inner_evm {
				Ethereum::Arbitrum => {
					let arb_gas_info_contract = ArbGasInfo::new(H160(ARB_GAS_INFO), client);
					let arb_gas_call = arb_gas_info_contract.get_prices_in_wei().await?;
					let gas_cost_for_execution = arb_gas_call.0 / U256::from(10u64.pow(18));
					let response_json = get_eth_to_usd_price(&eth_price_uri).await?;
					native_in_usd = to_18_decimals(response_json.result.ethusd.parse::<f64>()?);
					gas_cost_in_usd = gas_cost_for_execution * native_in_usd;
				},
				Ethereum::Optimism | Ethereum::Base => {
					let ovm_gas_price_oracle = OVM_gasPriceOracle::new(H160(OP_GAS_ORACLE), client);
					let gas_cost =
						ovm_gas_price_oracle.gas_price().await? / U256::from(10u64.pow(9));
					let response_json = get_eth_to_usd_price(&eth_price_uri).await?;
					native_in_usd = to_18_decimals(response_json.result.ethusd.parse::<f64>()?);
					gas_cost_in_usd = gas_cost * native_in_usd;
				},
				Ethereum::ExecutionLayer => {
					let response_json = get_eth_gas_and_price(&uri, &eth_price_uri).await?;
					let gas_price = to_18_decimals(
						response_json.result.safe_gas_price.parse::<f64>()? / 10f64.powf(9f64),
					);
					native_in_usd = to_18_decimals(response_json.result.usd_price.parse::<f64>()?);
					gas_cost_in_usd = gas_price * native_in_usd;
				},
			}
		},
		StateMachine::Polygon => {
			let uri = if TESTNET_CHAIN_IDS.contains(&chain_id) {
				// Mumbai
				format!(
					"https://api-testnet.polygonscan.com/api?module=gastracker&action=gasoracle&apikey={api_keys}"
				)
			} else {
				// Mainnet
				format!(
					"https://api.polygonscan.com/api?module=gastracker&action=gasoracle&apikey={api_keys}"
				)
			};
			let response = reqwest::get(&uri).await?;
			let response_json: GasResponse = response.json().await?;
			let gas_price = to_18_decimals(
				response_json.result.safe_gas_price.parse::<f64>()? / 10f64.powf(9f64),
			);
			native_in_usd = to_18_decimals(response_json.result.usd_price.parse::<f64>()?);
			gas_cost_in_usd = gas_price * native_in_usd;
		},
		StateMachine::Bsc => {
			let uri = if TESTNET_CHAIN_IDS.contains(&chain_id) {
				// Testnet
				format!(
					"https://api-testnet.bscscan.com/api?module=gastracker&action=gasoracle&apikey={api_keys}"
				)
			} else {
				// Mainnet
				format!(
					"https://api.bscscan.com/api?module=gastracker&action=gasoracle&apikey={api_keys}"
				)
			};
			let response = reqwest::get(&uri).await?;
			let response_json: GasResponse = response.json().await?;
			let gas_price = to_18_decimals(
				response_json.result.safe_gas_price.parse::<f64>()? / 10f64.powf(9f64),
			);
			native_in_usd = to_18_decimals(response_json.result.usd_price.parse::<f64>()?);
			gas_cost_in_usd = gas_price * native_in_usd;
		},
		_ => {},
	}

	Ok((gas_cost_in_usd, native_in_usd))
}

fn to_18_decimals(value: f64) -> U256 {
	((value * 10f64.powf(18f64)) as u64).into()
}

/// Returns the L2 data cost for a given transaction data in gwei
pub async fn get_l2_data_cost(
	rlp_tx: Bytes,
	chain: StateMachine,
	client: Arc<Provider<Ws>>,
) -> Result<U256, anyhow::Error> {
	let mut gas_cost_in_gwei = U256::zero();

	match chain {
		StateMachine::Ethereum(inner_evm) => {
			match inner_evm {
				Ethereum::Arbitrum => {
					let arb_gas_info_contract = ArbGasInfo::new(H160(ARB_GAS_INFO), client);
					let arb_gas_call = arb_gas_info_contract.get_prices_in_wei().await?;
					let data_cost_per_byte = arb_gas_call.1; // this is in wei
					let number_of_rlp_encoded_tx_bytes = rlp_tx.len();
					let data_cost_for_tx = data_cost_per_byte * number_of_rlp_encoded_tx_bytes;
					gas_cost_in_gwei = data_cost_for_tx / U256::from(1_000_000_000u64);
				},
				Ethereum::Optimism | Ethereum::Base => {
					let ovm_gas_price_oracle = OVM_gasPriceOracle::new(H160(OP_GAS_ORACLE), client);
					let data_cost = ovm_gas_price_oracle.get_l1_fee(rlp_tx).await?; // this is in Gwei
					gas_cost_in_gwei = data_cost
				},
				Ethereum::ExecutionLayer => {},
			}
		},
		_ => {
			log::debug!(target: "tesseract", "this chain is not a L2")
		},
	}

	Ok(gas_cost_in_gwei)
}

pub async fn get_eth_gas_and_price(
	uri: &String,
	uri_eth_price: &String,
) -> Result<GasResponse, Error> {
	let response = reqwest::get(uri).await?;

	let response_json: GasResponseEthereum = response.json().await?;
	let eth_to_usd_response = get_eth_to_usd_price(uri_eth_price).await?;

	Ok(GasResponse {
		result: GasResult {
			safe_gas_price: response_json.result.safe_gas_price,
			usd_price: eth_to_usd_response.result.ethusd,
		},
	})
}

pub async fn get_eth_to_usd_price(uri_eth_price: &String) -> Result<EthPriceResponse, Error> {
	let usd_response = reqwest::get(uri_eth_price).await?;
	Ok(usd_response.json::<EthPriceResponse>().await?)
}

pub fn get_chain_gas_limit(state_machine: StateMachine) -> u64 {
	match state_machine {
		StateMachine::Ethereum(Ethereum::ExecutionLayer) => 30_000_000,
		StateMachine::Ethereum(Ethereum::Arbitrum) => 1_000_000_000,
		StateMachine::Ethereum(Ethereum::Optimism) => 30_000_000,
		StateMachine::Ethereum(Ethereum::Base) => 25_000_000,
		StateMachine::Polygon => 30_000_000,
		StateMachine::Bsc => 140_000_000,
		_ => Default::default(),
	}
}

#[cfg(test)]
mod test {
	use crate::tx::get_current_gas_cost_in_usd;
	use ethers::prelude::{Provider, Ws};
	use ismp::host::{Ethereum, StateMachine};
	use std::sync::Arc;

	#[tokio::test]
	#[ignore]
	async fn test_get_current_gas_cost_in_usd_should_return_correct_value() {
		dotenv::dotenv().ok();
		let ethereum_etherscan_api_key = std::env::var("ETHERSCAN_ETHEREUM_KEY")
			.expect("Etherscan ethereum key is not set in .env.");
		let ethereum_rpc_uri =
			std::env::var("ETHEREUM_KEY_RPC").expect("Etherscan ethereum key is not set in .env.");
		let provider =
			Provider::<Ws>::connect_with_reconnects(ethereum_rpc_uri, 1000).await.unwrap();
		let client = Arc::new(provider.clone());

		let ethereum_gas_cost_in_usd = get_current_gas_cost_in_usd(
			11155111,
			StateMachine::Ethereum(Ethereum::ExecutionLayer),
			&ethereum_etherscan_api_key,
			client.clone(),
		)
		.await
		.unwrap();

		println!("Ethereum Gas Cost: {:?}", ethereum_gas_cost_in_usd);
	}
}
