use ismp::{
	consensus::StateMachineHeight,
	events::{Event as IsmpEvent, StateMachineUpdated},
	host::StateMachine,
	messaging::{Message, Proof, RequestMessage, ResponseMessage},
	router::{Post, Request, RequestResponse, Response},
	util::{hash_request, hash_response},
};
use sp_core::U256;
use std::collections::HashMap;
use tesseract_client::AnyClient;
use tesseract_primitives::{
	config::{Chain, RelayerConfig},
	Cost, Hasher, IsmpHost, IsmpProvider, Query,
};

/// Short description of a request/response event
#[derive(Debug)]
pub struct Meta {
	/// The source state machine of this request.
	pub source: StateMachine,
	/// The destination state machine of this request.
	pub dest: StateMachine,
	/// The nonce of this request on the source chain
	pub nonce: u64,
}

#[derive(Debug)]
pub enum Event {
	/// Emitted when a state machine is successfully updated to a new height after the challenge
	/// period has elapsed
	StateMachineUpdated(StateMachineUpdated),
	/// An event that is emitted when a post request is dispatched
	PostRequest(Meta),
	/// An event that is emitted when a post response is dispatched
	PostResponse(Meta),
	/// An event that is emitted when a get request is dispatched
	GetRequest(Meta),
}

impl From<IsmpEvent> for Event {
	fn from(value: IsmpEvent) -> Self {
		match value {
			IsmpEvent::StateMachineUpdated(e) => Event::StateMachineUpdated(e),
			IsmpEvent::PostRequest(e) =>
				Event::PostRequest(Meta { nonce: e.nonce, dest: e.dest, source: e.source }),
			IsmpEvent::PostResponse(e) => Event::PostResponse(Meta {
				nonce: e.post.nonce,
				dest: e.post.dest,
				source: e.post.source,
			}),
			IsmpEvent::GetRequest(e) =>
				Event::GetRequest(Meta { nonce: e.nonce, dest: e.dest, source: e.source }),
		}
	}
}

/// Translates events emitted from [`source`] into messages to be submitted to the counterparty
/// The [`state_machine_height`] parameter is the latest available height of [`source`] on
/// the counterparty chain
/// Returns a tuple where the first item are messages to be submitted to the sink
/// and the second items are messages to be submitted to the source
pub async fn translate_events_to_messages<A, B>(
	source: &A,
	sink: &B,
	events: Vec<IsmpEvent>,
	state_machine_height: StateMachineHeight,
	config: RelayerConfig,
	coprocessor: Chain,
	client_map: &HashMap<StateMachine, AnyClient>,
) -> Result<Vec<Message>, anyhow::Error>
where
	A: IsmpHost + IsmpProvider,
	B: IsmpHost + IsmpProvider,
{
	let mut post_request_queries = vec![];
	let mut response_queries = vec![];

	let mut post_requests = vec![];
	let mut post_responses = vec![];

	let mut request_messages = vec![];
	let mut response_messages = vec![];

	let counterparty_timestamp = sink.query_timestamp().await?;
	let is_allowed_module = |module: &Vec<u8>| match config.module_filter {
		Some(ref filters) =>
			if !filters.is_empty() {
				filters.iter().find(|filter| **filter == *module).is_some()
			} else {
				true
			},
		// if no filter is provided, allow all modules
		None => true,
	};

	for event in events.iter() {
		match event {
			IsmpEvent::PostRequest(post) => {
				// Skip timed out requests
				if post.timeout_timestamp != 0 &&
					post.timeout_timestamp <= counterparty_timestamp.as_secs()
				{
					tracing::trace!(
						"Found timed out request, request: {}, counterparty: {}",
						post.timeout_timestamp,
						counterparty_timestamp.as_secs()
					);
					continue
				}

				if !is_allowed_module(&post.from) {
					tracing::trace!(
						"Request from module {}, filtered by module filter",
						hex::encode(&post.from),
					);
					continue
				}

				let req = Request::Post(post.clone());
				let hash = hash_request::<Hasher>(&req);

				let query = Query {
					source_chain: req.source_chain(),
					dest_chain: req.dest_chain(),
					nonce: req.nonce(),
					commitment: hash,
				};

				let proof =
					source.query_requests_proof(state_machine_height.height, vec![query]).await?;

				let _msg = RequestMessage {
					requests: vec![post.clone()],
					proof: Proof { height: state_machine_height, proof },
					signer: sink.address(),
				};
				request_messages.push(Message::Request(_msg));
				post_requests.push(post.clone());
				post_request_queries.push(query);
			},
			IsmpEvent::PostResponse(post_response) => {
				// Skip timed out responses
				if post_response.timeout_timestamp != 0 &&
					post_response.timeout_timestamp <= counterparty_timestamp.as_secs()
				{
					tracing::trace!(
						"Found timed out request, request: {}, counterparty: {}",
						post_response.timeout_timestamp,
						counterparty_timestamp.as_secs()
					);
					continue
				}

				if !is_allowed_module(&post_response.source_module()) {
					tracing::trace!(
						"Request from module {}, filtered by module filter",
						hex::encode(&post_response.source_module()),
					);
					continue
				}

				let resp = Response::Post(post_response.clone());
				let hash = hash_response::<Hasher>(&resp);

				let query = Query {
					source_chain: resp.source_chain(),
					dest_chain: resp.dest_chain(),
					nonce: resp.nonce(),
					commitment: hash,
				};

				let proof =
					source.query_responses_proof(state_machine_height.height, vec![query]).await?;

				let _msg = ResponseMessage {
					datagram: RequestResponse::Response(vec![resp.clone()]),
					proof: Proof { height: state_machine_height, proof },
					signer: sink.address(),
				};
				response_messages.push(Message::Response(_msg));
				post_responses.push(resp);
				response_queries.push(query);
			},
			_ => {},
		}
	}

	let (post_requests, post_request_queries, post_responses, response_queries) = {
		tracing::trace!(
			"Tracing transactions to {:?}, from: {:?}",
			sink.state_machine_id().state_id,
			source.state_machine_id().state_id
		);
		let post_request_queries_to_push_with_option = return_successful_queries(
			sink,
			request_messages,
			post_request_queries,
			config.minimum_profit_percentage,
			coprocessor,
			&client_map,
		)
		.await?;

		let post_request_to_push: Vec<Post> = post_requests
			.into_iter()
			.zip(post_request_queries_to_push_with_option.iter())
			.filter_map(
				|(current_post, current_query)| {
					if current_query.is_some() {
						Some(current_post)
					} else {
						None
					}
				},
			)
			.collect();

		let post_request_queries_to_push: Vec<Query> = post_request_queries_to_push_with_option
			.into_iter()
			.filter_map(|query| query)
			.collect();

		let post_response_successful_query = return_successful_queries(
			sink,
			response_messages,
			response_queries,
			config.minimum_profit_percentage,
			coprocessor,
			&client_map,
		)
		.await?;

		let post_response_to_push: Vec<Response> = post_responses
			.into_iter()
			.zip(post_response_successful_query.iter())
			.filter_map(
				|(current_post, current_query)| {
					if current_query.is_some() {
						Some(current_post)
					} else {
						None
					}
				},
			)
			.collect();

		let post_response_queries_to_push: Vec<Query> =
			post_response_successful_query.into_iter().filter_map(|query| query).collect();
		(
			post_request_to_push,
			post_request_queries_to_push,
			post_response_to_push,
			post_response_queries_to_push,
		)
	};

	let mut messages = vec![];

	if !post_request_queries.is_empty() {
		tracing::trace!("Querying request proof for batch length {}", post_request_queries.len());
		let chunks = chunk_size(sink.state_machine_id().state_id);
		let query_chunks = post_request_queries.chunks(chunks);
		let post_request_chunks = post_requests.chunks(chunks);
		for (queries, post_requests) in query_chunks.into_iter().zip(post_request_chunks) {
			let requests_proof = source
				.query_requests_proof(state_machine_height.height, queries.to_vec())
				.await?;
			let msg = RequestMessage {
				requests: post_requests.to_vec(),
				proof: Proof { height: state_machine_height, proof: requests_proof },
				signer: sink.address(),
			};
			messages.push(Message::Request(msg));
		}
	}

	if !response_queries.is_empty() {
		tracing::trace!("Querying response proof for batch length {}", response_queries.len());
		let chunks = chunk_size(sink.state_machine_id().state_id);
		let query_chunks = response_queries.chunks(chunks);
		let post_request_chunks = post_responses.chunks(chunks);
		for (queries, post_responses) in query_chunks.into_iter().zip(post_request_chunks) {
			let responses_proof = source
				.query_responses_proof(state_machine_height.height, queries.to_vec())
				.await?;
			let msg = ResponseMessage {
				datagram: RequestResponse::Response(post_responses.to_vec()),
				proof: Proof { height: state_machine_height, proof: responses_proof },
				signer: sink.address(),
			};
			messages.push(Message::Response(msg));
		}
	}

	Ok(messages)
}

/// Return true for Request and Response events designated for the counterparty
pub fn filter_events(router_id: StateMachine, counterparty: StateMachine, ev: &IsmpEvent) -> bool {
	// Is the counterparty the routing chain?
	let is_router = router_id == counterparty;

	match ev {
		// We filter out events whose origin is the coprocessor
		IsmpEvent::PostRequest(post) =>
			(post.dest == counterparty && post.source != router_id) || is_router,
		IsmpEvent::PostResponse(resp) =>
			(resp.dest_chain() == counterparty && resp.source_chain() != router_id) || is_router,
		_ => false,
	}
}

fn chunk_size(state_machine: StateMachine) -> usize {
	match state_machine {
		StateMachine::Ethereum(_) | StateMachine::Bsc => 100,
		_ => 200,
	}
}

pub async fn return_successful_queries<A>(
	sink: &A,
	messages: Vec<Message>,
	queries: Vec<Query>,
	minimum_profit_percentage: u32,
	coprocessor: Chain,
	client_map: &HashMap<StateMachine, AnyClient>,
) -> Result<Vec<Option<Query>>, anyhow::Error>
where
	A: IsmpHost + IsmpProvider,
{
	let mut queries_to_be_relayed = Vec::new();
	let gas_estimates = sink.estimate_gas(messages.clone()).await?;
	for (index, estimate) in gas_estimates.into_iter().enumerate() {
		if estimate.successful_execution &&
			coprocessor.state_machine() != sink.state_machine_id().state_id
		{
			let total_gas_to_be_expended_in_usd = estimate.execution_cost;
			// what kind of message is this?
			let og_source = if let Some(client) = client_map.get(&queries[index].source_chain) {
				client
			} else {
				tracing::info!("Skipping tx because fee metadata cannot be queried, client for {:?} was not provided", queries[index].source_chain);
				queries_to_be_relayed.push(None);
				continue
			};
			let mut fee_metadata: Cost = if matches!(messages[index], Message::Request(_)) {
				og_source.query_request_fee_metadata(queries[index].commitment).await?.into()
			} else {
				og_source.query_response_fee_metadata(queries[index].commitment).await?.into()
			};

			let profit = (U256::from(minimum_profit_percentage) *
				total_gas_to_be_expended_in_usd.0) /
				U256::from(100);
			// 0 profit percentage means we want to relay all requests for free
			let fee_with_profit: Cost = total_gas_to_be_expended_in_usd + profit;

			if minimum_profit_percentage == 0 {
				fee_metadata = U256::MAX.into()
			};
			if fee_metadata < fee_with_profit {
				tracing::info!("Skipping unprofitable tx. Expected ${fee_with_profit}, user provided ${fee_metadata}");
				queries_to_be_relayed.push(None)
			} else {
				tracing::trace!(
					"Pushing tx to {:?} with cost ${fee_with_profit}",
					sink.state_machine_id().state_id
				);
				queries_to_be_relayed.push(Some(queries[index].clone()));
			}
		} else if estimate.successful_execution &&
			coprocessor.state_machine() == sink.state_machine_id().state_id
		// We only deliver sucessful messages to hyperbridge
		{
			tracing::trace!("Pushing tx to {:?}", sink.state_machine_id().state_id);
			queries_to_be_relayed.push(Some(queries[index].clone()));
		} else {
			tracing::info!("Skipping Failed tx");
			queries_to_be_relayed.push(None)
		}
	}

	Ok(queries_to_be_relayed)
}
