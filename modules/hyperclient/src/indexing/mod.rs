use crate::{types::EventMetadata, MessageStatusWithMetadata};
use anyhow::anyhow;
use graphql_client::{GraphQLQuery, Response};
use ismp::{
	host::{Ethereum, StateMachine},
	messaging::hash_request,
	router::Request,
};
use sp_core::{bytes::from_hex, H256, U256};

use crate::Keccak256;

pub mod graphql;

use graphql::*;

use self::{request_query::RequestStatus, state_machine_updates_query::SupportedChain};

const INDEXER_API: &'static str =
	"https://api.subquery.network/sq/polytope-labs/hyperbridge-indexers";

pub type BigInt = primitive_types::U256;

pub async fn query_request_status_from_indexer(
	request: Request,
) -> Result<Option<MessageStatusWithMetadata>, anyhow::Error> {
	let commitment = hash_request::<Keccak256>(&request);

	let id = format!("{commitment:?}");

	let request_body = RequestQuery::build_query(request_query::Variables { id });

	let client = reqwest::Client::new();
	let res = client.post(INDEXER_API).json(&request_body).send().await?;
	let response_body: Response<request_query::ResponseData> = res.json().await?;

	let mut metadata = response_body
		.data
		.ok_or_else(|| anyhow!("Failed to query request status"))?
		.request
		.ok_or_else(|| anyhow!("Request does not exist in database"))?
		.status_metadata
		.into_iter()
		.filter_map(|item| item)
		.collect::<Vec<_>>();
	metadata
		.sort_by(|a, b| request_status_weight(&a.status).cmp(&request_status_weight(&b.status)));

	if let Some(latest_status) = metadata.last().cloned() {
		// transform to message status with metadata
		let request_query::RequestQueryRequestStatusMetadata {
			status,
			transaction_hash,
			block_number,
			block_hash,
			..
		} = latest_status;

		let status = match status {
			request_query::RequestStatus::SOURCE => {
				// Try and fetch state machine update for source chain on hyperbridge
				let request_body: graphql_client::QueryBody<
					state_machine_updates_query::Variables,
				> = StateMachineUpdatesQuery::build_query(state_machine_updates_query::Variables {
					chain: SupportedChain::HYPERBRIDGE_GARGANTUA,
					state_machine_id: request.source_chain().to_string(),
					height: U256::from(block_number.parse::<u64>()?),
				});

				let res = client.post(INDEXER_API).json(&request_body).send().await?;
				let response_body: Response<state_machine_updates_query::ResponseData> =
					res.json().await?;
				let meta = if let Some(data) = response_body.data.and_then(|data| {
					data.state_machine_update_events.and_then(|update| {
						update.nodes.and_then(|node| node.get(0).cloned().flatten())
					})
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
			request_query::RequestStatus::MESSAGE_RELAYED => {
				// Try and fetch state machine update for hyperbridge on destination chain
				let request_body: graphql_client::QueryBody<
					state_machine_updates_query::Variables,
				> = StateMachineUpdatesQuery::build_query(state_machine_updates_query::Variables {
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
					height: U256::from(block_number.parse::<u64>()?),
				});

				let res = client.post(INDEXER_API).json(&request_body).send().await?;
				let response_body: Response<state_machine_updates_query::ResponseData> =
					res.json().await?;
				if let Some(data) = response_body.data.and_then(|data| {
					data.state_machine_update_events.and_then(|update| {
						update.nodes.and_then(|node| node.get(0).cloned().flatten())
					})
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
			request_query::RequestStatus::DEST => MessageStatusWithMetadata::DestinationDelivered {
				meta: EventMetadata {
					block_hash: H256::from_slice(&from_hex(&block_hash)?),
					transaction_hash: H256::from_slice(&from_hex(&transaction_hash)?),
					block_number: block_number.parse()?,
				},
			},
			request_query::RequestStatus::TIMED_OUT => MessageStatusWithMetadata::Timeout,
			request_query::RequestStatus::Other(_) => MessageStatusWithMetadata::Pending,
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
