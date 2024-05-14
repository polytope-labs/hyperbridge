use crate::{types::EventMetadata, MessageStatusWithMetadata};
use anyhow::anyhow;
use graphql_client::{GraphQLQuery, Response};
use ismp::{messaging::hash_request, router::Request};
use sp_core::{bytes::from_hex, H256, U256};

use crate::Keccak256;

pub mod graphql;

use graphql::*;

use self::state_machine_updates_query::SupportedChain;

const INDEXER_API: &'static str =
	"https://api.subquery.network/sq/polytope-labs/hyperbridge-indexers";

pub type BigInt = primitive_types::U256;

pub async fn query_request_status(
	request: Request,
) -> Result<Option<MessageStatusWithMetadata>, anyhow::Error> {
	let commitment = hash_request::<Keccak256>(&request);

	let id = format!("{commitment:?}");

	let request_body = RequestQuery::build_query(request_query::Variables { id });

	let client = reqwest::Client::new();
	let res = client.post(INDEXER_API).json(&request_body).send().await?;
	let response_body: Response<request_query::ResponseData> = res.json().await?;

	let metadata = response_body
		.data
		.ok_or_else(|| anyhow!("Failed to query request status"))?
		.request
		.ok_or_else(|| anyhow!("Request does not exist in database"))?
		.status_metadata;

	if let Some(latest_status) = metadata.last().cloned().flatten() {
		// transform to message status with metadata
		let request_query::RequestQueryRequestStatusMetadata {
			status,
			chain,
			transaction_hash,
			block_number,
			..
		} = latest_status;

		let status = match status {
			request_query::RequestStatus::SOURCE => {
				// Try and fetch state machine update for chain on hyperbridge destination chain
				let request_body: graphql_client::QueryBody<request_query::Variables> =
					StateMachineUpdatesQuery::build_query(state_machine_updates_query::Variables {
						chain,
						height: U256::from(block_number.parse::<u64>()?),
					});

				let res = client.post(INDEXER_API).json(&request_body).send().await?;
				let response_body: Response<state_machine_updates_query::ResponseData> = res.json().await?;
				let meta = if let Some(data) = response_body.data.and_then(|data| data.state_machine_update_events.and_then(|update| update.nodes.and_then(|node| node.get(0).cloned().flatten()))) {
					EventMetadata {
        				block_hash: todo!(),
        				transaction_hash: todo!(),
        				block_number: todo!(),
    				}
				} else {
					Default::default()
				};
				MessageStatusWithMetadata::SourceFinalized {
					finalized_height: block_number.parse()?,
					meta: Default::default(),
				}
			},
			request_query::RequestStatus::MESSAGE_RELAYED =>
				MessageStatusWithMetadata::HyperbridgeDelivered {
					meta: EventMetadata {
						block_hash: H256::default(),
						transaction_hash: H256::from_slice(&from_hex(&transaction_hash)?),
						block_number: block_number.parse()?,
					},
				},
			request_query::RequestStatus::DEST => MessageStatusWithMetadata::DestinationDelivered {
				meta: EventMetadata {
					block_hash: H256::default(),
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
