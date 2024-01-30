use ismp::{
	consensus::StateMachineHeight,
	events::{Event as IsmpEvent, StateMachineUpdated},
	host::StateMachine,
	messaging::{Message, Proof, RequestMessage, ResponseMessage},
	router::{Post, Request, RequestResponse, Response},
	util::{hash_request, hash_response, Keccak256},
};
use sp_core::{keccak_256, H256};
use tesseract_primitives::{config::RelayerConfig, IsmpHost, IsmpProvider, Query};

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

/// Parse events emitted from [`source`] into messages to be submitted to the counterparty
/// The [`state_machine_height`] parameter is the latest available height of [`source`] on
/// the counterparty chain
/// Returns a tuple where the first item are messages to be submitted to the sink
/// and the second items are messages to be submitted to the source
pub async fn parse_ismp_events<A, B>(
	source: &A,
	sink: &B,
	events: Vec<IsmpEvent>,
	state_machine_height: StateMachineHeight,
	config: RelayerConfig,
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

	for event in events.iter() {
		match event {
			IsmpEvent::PostRequest(post) => {
				// Skip timed out requests
				if post.timeout_timestamp != 0 &&
					post.timeout_timestamp <= counterparty_timestamp.as_secs()
				{
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

				if config.chain.state_machine() != sink.state_machine_id().state_id {
					request_messages.push(Message::Request(_msg));
				}
				post_requests.push(post.clone());
				post_request_queries.push(query);
			},
			IsmpEvent::PostResponse(post_response) => {
				// Skip timed out responses
				if post_response.timeout_timestamp != 0 &&
					post_response.timeout_timestamp <= counterparty_timestamp.as_secs()
				{
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

				if config.chain.state_machine() != sink.state_machine_id().state_id {
					response_messages.push(Message::Response(_msg));
				}
				post_responses.push(resp);
				response_queries.push(query);
			},
			_ => {},
		}
	}

	let (post_requests, post_request_queries, post_responses, response_queries) =
		if config.chain.state_machine() != sink.state_machine_id().state_id {
			let post_request_queries_to_push_with_option = return_successful_queries(
				source,
				sink,
				request_messages,
				post_request_queries,
				config.minimum_profit_percentage,
				true,
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
				source,
				sink,
				response_messages,
				response_queries,
				config.minimum_profit_percentage,
				false,
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
		} else {
			(post_requests, post_request_queries, post_responses, response_queries)
		};

	let mut messages = vec![];

	if !post_request_queries.is_empty() {
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
		IsmpEvent::PostRequest(post) => (post.dest == counterparty) || is_router,
		IsmpEvent::PostResponse(resp) => (resp.post.source == counterparty) || is_router,
		_ => false,
	}
}

fn chunk_size(state_machine: StateMachine) -> usize {
	match state_machine {
		StateMachine::Ethereum(_) => 100,
		_ => 200,
	}
}

pub struct Hasher;

impl Keccak256 for Hasher {
	fn keccak256(bytes: &[u8]) -> H256 {
		keccak_256(bytes).into()
	}
}

pub async fn return_successful_queries<A, B>(
	source: &A,
	sink: &B,
	messages: Vec<Message>,
	queries: Vec<Query>,
	minimum_profit_percentage: u32,
	request_batch: bool,
) -> Result<Vec<Option<Query>>, anyhow::Error>
where
	A: IsmpHost + IsmpProvider,
	B: IsmpHost + IsmpProvider,
{
	let mut queries_to_be_relayed = Vec::new();

	match sink.state_machine_id().state_id {
		StateMachine::Ethereum(_) => {
			let gas_estimates = sink.estimate_gas(messages).await?;
			for (index, estimate) in gas_estimates.into_iter().enumerate() {
				if estimate.successful_execution {
					let total_gas_to_be_expended_in_usd = estimate.execution_cost;
					let fee_metadata = if request_batch {
						source.get_message_request_fee_metadata(queries[index].commitment).await?
					} else {
						source
							.query_message_response_fee_metadata(queries[index].commitment)
							.await?
					};

					if fee_metadata <
						(total_gas_to_be_expended_in_usd * (minimum_profit_percentage + 100)) /
							100
					{
						log::debug!("not pushing this message, relay is not profitable");
						queries_to_be_relayed.push(None)
					} else {
						queries_to_be_relayed.push(Some(queries[index].clone()));
					}
				}
			}
		},
		_ => {},
	}

	Ok(queries_to_be_relayed)
}
