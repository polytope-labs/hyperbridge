#![allow(non_camel_case_types)]
use crate::{
	internals::{encode_request_call_data, encode_response_call_data},
	types::EventMetadata,
	HyperClient, MessageStatusWithMetadata,
};
use anyhow::anyhow;
use ismp::{
	host::{Ethereum, StateMachine},
	messaging::{hash_request, hash_response},
	router::{Request, Response},
};
use sp_core::{bytes::from_hex, H256};

use crate::Keccak256;

use gql_client::Client;
use serde::{Deserialize, Serialize};

static REQUEST_QUERY: &'static str = r#"
query RequestQuery($id: String!) {
	request(id: $id) {
	  status
	  statusMetadata {
		nodes {
			  id
			  status
			  chain
			  timestamp
			  blockNumber
			  transactionHash
			  blockHash
		}
	  }
	}
  }
"#;

static RESPONSE_QUERY: &'static str = r#"
query ResponseQuery($id: String!) {
	response(id: $id) {
	  status
	  statusMetadata {
		nodes {
			  id
			  status
			  chain
			  timestamp
			  blockNumber
			  transactionHash
			  blockHash
		}
	  }
	}
  }
"#;

static STATE_MACHINE_QUERY: &'static str = r#"
query StateMachineUpdatesQuery($stateMachineId: String!, $chain: SupportedChain!, $height: BigFloat!) {
	stateMachineUpdateEvents(
	  filter: {and: {stateMachineId: {equalTo: $stateMachineId }, chain: {equalTo: $chain}, height: {greaterThanOrEqualTo: $height}}}
	) {
	  nodes {
		blockHash
		blockNumber
		chain
		height
		id
		stateMachineId
		transactionHash
	  }
	}
  }
"#;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum SupportedChain {
	ETHE,
	BASE,
	OPTI,
	ARBI,
	BSC,
	POLY,
	HYPERBRIDGE,
	Other(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum Status {
	SOURCE,
	MESSAGE_RELAYED,
	DEST,
	TIMED_OUT,
	Other(String),
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StatusMetadata {
	pub nodes: Vec<StatusMetadataNode>,
}
#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StatusMetadataNode {
	pub id: String,
	pub status: Status,
	pub chain: SupportedChain,
	pub timestamp: BigInt,
	#[serde(rename = "blockNumber")]
	pub block_number: String,
	#[serde(rename = "transactionHash")]
	pub transaction_hash: String,
	#[serde(rename = "blockHash")]
	pub block_hash: String,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct RequestData {
	request: RequestNode,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ResponseData {
	response: ResponseNode,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct RequestNode {
	status: String,
	#[serde(rename = "statusMetadata")]
	status_metadata: StatusMetadata,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ResponseNode {
	status: String,
	#[serde(rename = "statusMetadata")]
	status_metadata: StatusMetadata,
}

#[derive(Serialize)]
pub struct RequestResponseVariables {
	id: String,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StateMachineResponseData {
	#[serde(rename = "stateMachineUpdateEvents")]
	pub state_machine_update_events: Option<StateMachineUpdatesQueryStateMachineUpdateEvents>,
}
#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StateMachineUpdatesQueryStateMachineUpdateEvents {
	pub nodes: Vec<StateMachineUpdatesQueryStateMachineUpdateEventsNodes>,
}
#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StateMachineUpdatesQueryStateMachineUpdateEventsNodes {
	#[serde(rename = "blockHash")]
	pub block_hash: String,
	#[serde(rename = "blockNumber")]
	pub block_number: BigInt,
	pub chain: SupportedChain,
	pub height: BigInt,
	pub id: String,
	#[serde(rename = "stateMachineId")]
	pub state_machine_id: String,
	#[serde(rename = "transactionHash")]
	pub transaction_hash: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StateMachineUpdateVariables {
	#[serde(rename = "stateMachineId")]
	pub state_machine_id: String,
	pub chain: SupportedChain,
	pub height: u64,
}

pub type BigInt = primitive_types::U256;

pub async fn query_request_status_from_indexer(
	request: Request,
	hyperclient: Option<&HyperClient>,
) -> Result<Option<MessageStatusWithMetadata>, anyhow::Error> {
	let commitment = hash_request::<Keccak256>(&request);

	let id = format!("{commitment:?}");
	let indexer_api = std::env::var("INDEXER_URL").unwrap_or("http://localhost:3000".to_string());

	let client = Client::new(indexer_api);
	let vars = RequestResponseVariables { id };
	let response_body = client
		.query_with_vars::<RequestData, RequestResponseVariables>(REQUEST_QUERY, vars)
		.await
		.map_err(|e| anyhow!("Failed to query request from indexer {e:?}"))?;

	let mut metadata = response_body
		.ok_or_else(|| anyhow!("Request not found in indexer db"))?
		.request
		.status_metadata
		.nodes
		.into_iter()
		.collect::<Vec<_>>();
	metadata.sort_by(|a, b| status_weight(&a.status).cmp(&status_weight(&b.status)));

	if let Some(latest_status) = metadata.last().cloned() {
		// transform to message status with metadata
		let StatusMetadataNode { status, transaction_hash, block_number, block_hash, .. } =
			latest_status;

		let status = match status {
			Status::SOURCE => {
				// Try and fetch state machine update for source chain on hyperbridge
				let vars = StateMachineUpdateVariables {
					chain: SupportedChain::HYPERBRIDGE,
					state_machine_id: request.source_chain().to_string(),
					height: block_number.parse::<u64>()?,
				};

				let response_body = client
					.query_with_vars::<StateMachineResponseData, _>(STATE_MACHINE_QUERY, vars)
					.await
					.map_err(|e| {
						anyhow!("Failed to query state machine update from indexer {e:?}")
					})?;

				let meta = if let Some(data) = response_body.and_then(|data| {
					data.state_machine_update_events.and_then(|update| update.nodes.get(0).cloned())
				}) {
					EventMetadata {
						block_hash: H256::from_slice(&from_hex(&data.block_hash)?),
						transaction_hash: H256::from_slice(&from_hex(&data.transaction_hash)?),
						block_number: data.block_number.low_u64(),
					}
				} else {
					Default::default()
				};

				MessageStatusWithMetadata::SourceFinalized {
					finalized_height: block_number.parse()?,
					meta,
				}
			},
			Status::MESSAGE_RELAYED => {
				// Try and fetch state machine update for hyperbridge on destination chain
				let vars = StateMachineUpdateVariables {
					chain: {
						match request.dest_chain() {
							StateMachine::Ethereum(Ethereum::ExecutionLayer) =>
								SupportedChain::ETHE,
							StateMachine::Ethereum(Ethereum::Base) => SupportedChain::BASE,
							StateMachine::Ethereum(Ethereum::Arbitrum) => SupportedChain::ARBI,
							StateMachine::Ethereum(Ethereum::Optimism) => SupportedChain::OPTI,
							StateMachine::Bsc => SupportedChain::BSC,
							StateMachine::Polygon => SupportedChain::POLY,
							_ => Err(anyhow!("Unsupported chain for indexer"))?,
						}
					},
					state_machine_id: request.dest_chain().to_string(),
					height: block_number.parse::<u64>()?,
				};
				let response_body = client
					.query_with_vars::<StateMachineResponseData, _>(STATE_MACHINE_QUERY, vars)
					.await
					.map_err(|e| {
						anyhow!("Failed to query state machine update from indexer {e:?}")
					})?;

				if let Some(data) = response_body.and_then(|data| {
					data.state_machine_update_events.and_then(|update| update.nodes.get(0).cloned())
				}) {
					let meta = EventMetadata {
						block_hash: H256::from_slice(&from_hex(&data.block_hash)?),
						transaction_hash: H256::from_slice(&from_hex(&data.transaction_hash)?),
						block_number: data.block_number.low_u64(),
					};

					let calldata = if let Some(hyperclient) = hyperclient {
						match request {
							Request::Post(post) => {
								let dest_client = hyperclient.dest.clone();
								let hyperbridge = hyperclient.hyperbridge.clone();
								encode_request_call_data(
									&hyperbridge,
									&dest_client,
									post,
									commitment,
									data.height.low_u64(),
								)
								.await?
							},
							_ => Default::default(),
						}
					} else {
						Default::default()
					};

					MessageStatusWithMetadata::HyperbridgeFinalized {
						finalized_height: data.height.low_u64(),
						meta,
						calldata,
					}
				} else {
					MessageStatusWithMetadata::HyperbridgeDelivered {
						meta: EventMetadata {
							block_hash: H256::from_slice(&from_hex(&block_hash)?),
							transaction_hash: H256::from_slice(&from_hex(&transaction_hash)?),
							block_number: block_number.parse()?,
						},
					}
				}
			},
			Status::DEST => MessageStatusWithMetadata::DestinationDelivered {
				meta: EventMetadata {
					block_hash: H256::from_slice(&from_hex(&block_hash)?),
					transaction_hash: H256::from_slice(&from_hex(&transaction_hash)?),
					block_number: block_number.parse()?,
				},
			},
			Status::TIMED_OUT => MessageStatusWithMetadata::Timeout,
			Status::Other(_) => MessageStatusWithMetadata::Pending,
		};
		return Ok(Some(status))
	}

	Ok(None)
}

fn status_weight(status: &Status) -> u8 {
	match status {
		Status::SOURCE => 1,
		Status::MESSAGE_RELAYED => 2,
		Status::DEST => 3,
		Status::TIMED_OUT => 4,
		Status::Other(_) => 4,
	}
}

pub async fn query_response_status_from_indexer(
	response: Response,
	hyperclient: Option<&HyperClient>,
) -> Result<Option<MessageStatusWithMetadata>, anyhow::Error> {
	let commitment = hash_response::<Keccak256>(&response);

	let id = format!("{commitment:?}");
	let indexer_api = std::env::var("INDEXER_URL").unwrap_or("http://localhost:3000".to_string());

	let client = Client::new(indexer_api);
	let vars = RequestResponseVariables { id };
	let response_body = client
		.query_with_vars::<ResponseData, RequestResponseVariables>(RESPONSE_QUERY, vars)
		.await
		.map_err(|e| anyhow!("Failed to query request from indexer {e:?}"))?;

	let mut metadata = response_body
		.ok_or_else(|| anyhow!("Request not found in indexer db"))?
		.response
		.status_metadata
		.nodes
		.into_iter()
		.collect::<Vec<_>>();
	metadata.sort_by(|a, b| status_weight(&a.status).cmp(&status_weight(&b.status)));

	if let Some(latest_status) = metadata.last().cloned() {
		// transform to message status with metadata
		let StatusMetadataNode { status, transaction_hash, block_number, block_hash, .. } =
			latest_status;

		let status = match status {
			Status::SOURCE => {
				// Try and fetch state machine update for source chain on hyperbridge
				let vars = StateMachineUpdateVariables {
					chain: SupportedChain::HYPERBRIDGE,
					state_machine_id: response.source_chain().to_string(),
					height: block_number.parse::<u64>()?,
				};

				let response_body = client
					.query_with_vars::<StateMachineResponseData, _>(STATE_MACHINE_QUERY, vars)
					.await
					.map_err(|e| {
						anyhow!("Failed to query state machine update from indexer {e:?}")
					})?;

				let meta = if let Some(data) = response_body.and_then(|data| {
					data.state_machine_update_events.and_then(|update| update.nodes.get(0).cloned())
				}) {
					EventMetadata {
						block_hash: H256::from_slice(&from_hex(&data.block_hash)?),
						transaction_hash: H256::from_slice(&from_hex(&data.transaction_hash)?),
						block_number: data.block_number.low_u64(),
					}
				} else {
					Default::default()
				};

				MessageStatusWithMetadata::SourceFinalized {
					finalized_height: block_number.parse()?,
					meta,
				}
			},
			Status::MESSAGE_RELAYED => {
				// Try and fetch state machine update for hyperbridge on destination chain
				let vars = StateMachineUpdateVariables {
					chain: {
						match response.dest_chain() {
							StateMachine::Ethereum(Ethereum::ExecutionLayer) =>
								SupportedChain::ETHE,
							StateMachine::Ethereum(Ethereum::Base) => SupportedChain::BASE,
							StateMachine::Ethereum(Ethereum::Arbitrum) => SupportedChain::ARBI,
							StateMachine::Ethereum(Ethereum::Optimism) => SupportedChain::OPTI,
							StateMachine::Bsc => SupportedChain::BSC,
							StateMachine::Polygon => SupportedChain::POLY,
							_ => Err(anyhow!("Unsupported chain for indexer"))?,
						}
					},
					state_machine_id: response.dest_chain().to_string(),
					height: block_number.parse::<u64>()?,
				};
				let response_body = client
					.query_with_vars::<StateMachineResponseData, _>(STATE_MACHINE_QUERY, vars)
					.await
					.map_err(|e| {
						anyhow!("Failed to query state machine update from indexer {e:?}")
					})?;

				if let Some(data) = response_body.and_then(|data| {
					data.state_machine_update_events.and_then(|update| update.nodes.get(0).cloned())
				}) {
					let meta = EventMetadata {
						block_hash: H256::from_slice(&from_hex(&data.block_hash)?),
						transaction_hash: H256::from_slice(&from_hex(&data.transaction_hash)?),
						block_number: data.block_number.low_u64(),
					};

					let calldata = if let Some(hyperclient) = hyperclient {
						match response {
							Response::Post(post) => {
								let dest_client = hyperclient.dest.clone();
								let hyperbridge = &hyperclient.hyperbridge;
								encode_response_call_data(
									hyperbridge,
									&dest_client,
									post,
									commitment,
									data.height.low_u64(),
								)
								.await?
							},
							_ => Default::default(),
						}
					} else {
						Default::default()
					};

					MessageStatusWithMetadata::HyperbridgeFinalized {
						finalized_height: data.height.low_u64(),
						meta,
						calldata,
					}
				} else {
					MessageStatusWithMetadata::HyperbridgeDelivered {
						meta: EventMetadata {
							block_hash: H256::from_slice(&from_hex(&block_hash)?),
							transaction_hash: H256::from_slice(&from_hex(&transaction_hash)?),
							block_number: block_number.parse()?,
						},
					}
				}
			},
			Status::DEST => MessageStatusWithMetadata::DestinationDelivered {
				meta: EventMetadata {
					block_hash: H256::from_slice(&from_hex(&block_hash)?),
					transaction_hash: H256::from_slice(&from_hex(&transaction_hash)?),
					block_number: block_number.parse()?,
				},
			},
			Status::TIMED_OUT => MessageStatusWithMetadata::Timeout,
			Status::Other(_) => MessageStatusWithMetadata::Pending,
		};
		return Ok(Some(status))
	}

	Ok(None)
}
