#![allow(non_camel_case_types)]
use crate::{types::EventMetadata, MessageStatusWithMetadata};
use anyhow::anyhow;
use ismp::{
	host::{Ethereum, StateMachine},
	messaging::hash_request,
	router::Request,
};
use sp_core::{bytes::from_hex, H256};

use crate::Keccak256;

const INDEXER_API: &'static str =
	"https://api.subquery.network/sq/polytope-labs/hyperbridge-indexers";

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

static _RESPONSE_QUERY: &'static str = r#"
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
	ETHEREUM_SEPOLIA,
	BASE_SEPOLIA,
	OPTIMISM_SEPOLIA,
	ARBITRUM_SEPOLIA,
	BSC_CHAPEL,
	HYPERBRIDGE_GARGANTUA,
	Other(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum RequestStatus {
	SOURCE,
	MESSAGE_RELAYED,
	DEST,
	TIMED_OUT,
	Other(String),
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct RequestQueryRequestStatusMetadata {
	pub nodes: Vec<RequestQueryRequestStatusMetadataNodes>,
}
#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct RequestQueryRequestStatusMetadataNodes {
	pub id: String,
	pub status: RequestStatus,
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
pub struct RequestNode {
	status: String,
	#[serde(rename = "statusMetadata")]
	status_metadata: RequestQueryRequestStatusMetadata,
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
) -> Result<Option<MessageStatusWithMetadata>, anyhow::Error> {
	let commitment = hash_request::<Keccak256>(&request);

	let id = format!("{commitment:?}");

	let client = Client::new(INDEXER_API);
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
	metadata
		.sort_by(|a, b| request_status_weight(&a.status).cmp(&request_status_weight(&b.status)));

	if let Some(latest_status) = metadata.last().cloned() {
		// transform to message status with metadata
		let RequestQueryRequestStatusMetadataNodes {
			status,
			transaction_hash,
			block_number,
			block_hash,
			..
		} = latest_status;

		let status = match status {
			RequestStatus::SOURCE => {
				// Try and fetch state machine update for source chain on hyperbridge
				let vars = StateMachineUpdateVariables {
					chain: SupportedChain::HYPERBRIDGE_GARGANTUA,
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
			RequestStatus::MESSAGE_RELAYED => {
				// Try and fetch state machine update for hyperbridge on destination chain
				let vars = StateMachineUpdateVariables {
					chain: {
						match request.dest_chain() {
							StateMachine::Ethereum(Ethereum::ExecutionLayer) =>
								SupportedChain::ETHEREUM_SEPOLIA,
							StateMachine::Ethereum(Ethereum::Base) => SupportedChain::BASE_SEPOLIA,
							StateMachine::Ethereum(Ethereum::Arbitrum) =>
								SupportedChain::ARBITRUM_SEPOLIA,
							StateMachine::Ethereum(Ethereum::Optimism) =>
								SupportedChain::OPTIMISM_SEPOLIA,
							StateMachine::Polkadot(3367) | StateMachine::Kusama(4009) =>
								SupportedChain::HYPERBRIDGE_GARGANTUA,
							StateMachine::Bsc => SupportedChain::BSC_CHAPEL,
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

					MessageStatusWithMetadata::HyperbridgeFinalized {
						finalized_height: data.height.low_u64(),
						meta,
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
			RequestStatus::DEST => MessageStatusWithMetadata::DestinationDelivered {
				meta: EventMetadata {
					block_hash: H256::from_slice(&from_hex(&block_hash)?),
					transaction_hash: H256::from_slice(&from_hex(&transaction_hash)?),
					block_number: block_number.parse()?,
				},
			},
			RequestStatus::TIMED_OUT => MessageStatusWithMetadata::Timeout,
			RequestStatus::Other(_) => MessageStatusWithMetadata::Pending,
		};
		return Ok(Some(status))
	}

	Ok(None)
}

fn request_status_weight(status: &RequestStatus) -> u8 {
	match status {
		RequestStatus::SOURCE => 0,
		RequestStatus::MESSAGE_RELAYED => 1,
		RequestStatus::DEST => 2,
		RequestStatus::TIMED_OUT => 3,
		RequestStatus::Other(_) => 4,
	}
}
