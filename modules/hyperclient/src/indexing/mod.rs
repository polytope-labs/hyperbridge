use crate::MessageStatusWithMetadata;
use anyhow::anyhow;
use graphql_client::{GraphQLQuery, Response};
use ismp::{messaging::hash_request, router::Request};

use crate::Keccak256;

pub mod graphql;

use graphql::*;

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

	if let Some(latest_status) = metadata.last().cloned() {
		// transform to message status with metadata
		// let request_query::RequestQueryRequestStatusMetadata {status, chain, transaction_hash, ..
		// } = latest_status;
	}

	Ok(None)
}
