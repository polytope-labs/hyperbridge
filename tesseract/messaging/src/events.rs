#![allow(dead_code)]
use anyhow::anyhow;
use futures::stream::FuturesOrdered;
use ismp::{
	consensus::StateMachineHeight,
	events::{
		Event as IsmpEvent, Meta, RequestResponseHandled, StateMachineUpdated, TimeoutHandled,
	},
	host::StateMachine,
	messaging::{hash_request, hash_response, Message, Proof, RequestMessage, ResponseMessage},
	router::{PostRequest, Request, RequestResponse, Response},
};
use sp_core::{H160, U256};
use std::{collections::HashMap, sync::Arc};
use tesseract_primitives::{config::RelayerConfig, Cost, Hasher, IsmpProvider, Query};
use tokio_stream::StreamExt;

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
	/// An event that is emitted when a get response is dispatched
	GetResponse(Meta),
	/// Post request handled
	PostRequestHandled(RequestResponseHandled),
	/// Emitted when a post response is handled
	PostResponseHandled(RequestResponseHandled),
	/// Emitted when a post request timeout is handled
	PostRequestTimeoutHandled(TimeoutHandled),
	/// Emitted when a post response timeout is handled
	PostResponseTimeoutHandled(TimeoutHandled),
	/// Emitted when a get request is handled
	GetRequestHandled(RequestResponseHandled),
	/// Emitted when a get request timeout is handled
	GetRequestTimeoutHandled(TimeoutHandled),
	/// State Commitment Vetoed
	StateCommitmentVetoed,
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
			IsmpEvent::GetResponse(e) => Event::GetResponse(Meta {
				nonce: e.get.nonce,
				dest: e.get.dest,
				source: e.get.source,
			}),
			IsmpEvent::PostRequestHandled(ev) => Event::PostRequestHandled(ev),
			IsmpEvent::PostResponseHandled(handled) => Event::PostResponseHandled(handled),
			IsmpEvent::PostRequestTimeoutHandled(handled) =>
				Event::PostRequestTimeoutHandled(handled),
			IsmpEvent::PostResponseTimeoutHandled(handled) =>
				Event::PostResponseTimeoutHandled(handled),
			IsmpEvent::GetRequestHandled(handled) => Event::GetRequestHandled(handled),
			IsmpEvent::GetRequestTimeoutHandled(handled) =>
				Event::GetRequestTimeoutHandled(handled),
			IsmpEvent::StateCommitmentVetoed(_) => Event::StateCommitmentVetoed,
		}
	}
}

/// Translates events emitted from [`source`] into messages to be submitted to the counterparty
/// The [`state_machine_height`] parameter is the latest available height of [`source`] on
/// the counterparty chain
/// Returns a tuple where the first item are messages to be submitted to the sink
/// and the second tuple are currently unprofitable messages
pub async fn translate_events_to_messages(
	source: Arc<dyn IsmpProvider>,
	sink: Arc<dyn IsmpProvider>,
	events: Vec<IsmpEvent>,
	state_machine_height: StateMachineHeight,
	config: RelayerConfig,
	coprocessor: StateMachine,
	client_map: &HashMap<StateMachine, Arc<dyn IsmpProvider>>,
) -> Result<(Vec<Message>, Vec<Message>), anyhow::Error> {
	let mut post_request_queries = vec![];
	let mut response_queries = vec![];

	let mut post_requests = vec![];
	let mut post_responses = vec![];

	let mut request_messages = vec![];
	let mut response_messages = vec![];

	let counterparty_timestamp = sink.query_timestamp().await?;

	// Fetch message proofs for estimating gas concurrently
	let batch_size = source.max_concurrent_queries();
	for chunk in events.chunks(batch_size) {
		let processes = chunk
			.into_iter()
			.map(|event| {
				let source = source.clone();
				let event = event.clone();
				let sink = sink.clone();
				let config = config.clone();
				async move {
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
								return Ok::<_, anyhow::Error>(None);
							}

							if !is_allowed_module(&config, &post.from) {
								tracing::trace!(
									"Request from module {}, filtered by module filter",
									hex::encode(&post.from),
								);
								return Ok(None);
							}

							let req = Request::Post(post.clone());
							let hash = hash_request::<Hasher>(&req);

							let query = Query {
								source_chain: req.source_chain(),
								dest_chain: req.dest_chain(),
								nonce: req.nonce(),
								commitment: hash,
							};

							let proof = source
								.query_requests_proof(
									state_machine_height.height,
									vec![query],
									sink.state_machine_id().state_id,
								)
								.await?;

							let _msg = RequestMessage {
								requests: vec![post.clone()],
								proof: Proof { height: state_machine_height, proof },
								signer: sink.address(),
							};
							Ok(Some((Message::Request(_msg), query)))
						},
						IsmpEvent::PostResponse(post_response) => {
							// Skip timed out responses
							if post_response.timeout_timestamp != 0 &&
								post_response.timeout_timestamp <=
									counterparty_timestamp.as_secs()
							{
								tracing::trace!(
									"Found timed out request, request: {}, counterparty: {}",
									post_response.timeout_timestamp,
									counterparty_timestamp.as_secs()
								);
								return Ok(None);
							}

							if !is_allowed_module(&config, &post_response.source_module()) {
								tracing::trace!(
									"Request from module {}, filtered by module filter",
									hex::encode(&post_response.source_module()),
								);
								return Ok(None);
							}

							let resp = Response::Post(post_response.clone());
							let hash = hash_response::<Hasher>(&resp);

							let query = Query {
								source_chain: resp.source_chain(),
								dest_chain: resp.dest_chain(),
								nonce: resp.nonce(),
								commitment: hash,
							};

							let proof = source
								.query_responses_proof(
									state_machine_height.height,
									vec![query],
									sink.state_machine_id().state_id,
								)
								.await?;

							let _msg = ResponseMessage {
								datagram: RequestResponse::Response(vec![resp.clone()]),
								proof: Proof { height: state_machine_height, proof },
								signer: sink.address(),
							};
							Ok(Some((Message::Response(_msg), query)))
						},
						_ => Ok(None),
					}
				}
			})
			.collect::<FuturesOrdered<_>>();

		let mut results = processes.collect::<Result<Vec<_>, _>>().await?.into_iter().flatten();

		while let Some((msg, query)) = results.next() {
			match msg {
				Message::Request(req_msg) => {
					post_request_queries.push(query);
					post_requests.push(
						req_msg
							.requests
							.get(0)
							.cloned()
							.ok_or_else(|| anyhow!("Expected a post to be present"))?,
					);
					request_messages.push(Message::Request(req_msg))
				},
				Message::Response(resp_msg) => {
					response_queries.push(query);
					let response = match resp_msg.datagram {
						RequestResponse::Response(ref resps) => resps
							.get(0)
							.cloned()
							.ok_or_else(|| anyhow!("Expected a response to be present"))?,
						_ => Err(anyhow!("Expected Response found posts"))?,
					};
					post_responses.push(response);
					response_messages.push(Message::Response(resp_msg))
				},
				_ => Err(anyhow!("Unexpected message: {msg:?}"))?,
			}
		}
	}

	let mut unprofitable = vec![];

	let (post_requests, post_request_queries, post_responses, response_queries) = {
		if !request_messages.is_empty() || !response_messages.is_empty() {
			tracing::trace!(
				"Tracing transactions to {:?}, from: {:?}",
				sink.state_machine_id().state_id,
				source.state_machine_id().state_id
			);
		}

		let post_request_queries_to_push_with_option = return_successful_queries(
			sink.clone(),
			request_messages,
			post_request_queries,
			config.minimum_profit_percentage,
			coprocessor,
			&client_map,
			config.deliver_failed.unwrap_or_default(),
		)
		.await?;

		unprofitable.extend(post_request_queries_to_push_with_option.retriable_messages);

		let post_request_to_push: Vec<PostRequest> = post_requests
			.into_iter()
			.zip(post_request_queries_to_push_with_option.queries.iter())
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
			.queries
			.into_iter()
			.filter_map(|query| query)
			.collect();

		let post_response_successful_query = return_successful_queries(
			sink.clone(),
			response_messages,
			response_queries,
			config.minimum_profit_percentage,
			coprocessor,
			&client_map,
			config.deliver_failed.unwrap_or_default(),
		)
		.await?;

		unprofitable.extend(post_response_successful_query.retriable_messages);

		let post_response_to_push: Vec<Response> = post_responses
			.into_iter()
			.zip(post_response_successful_query.queries.iter())
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

		let post_response_queries_to_push: Vec<Query> = post_response_successful_query
			.queries
			.into_iter()
			.filter_map(|query| query)
			.collect();
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
				.query_requests_proof(
					state_machine_height.height,
					queries.to_vec(),
					sink.state_machine_id().state_id,
				)
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
				.query_responses_proof(
					state_machine_height.height,
					queries.to_vec(),
					sink.state_machine_id().state_id,
				)
				.await?;
			let msg = ResponseMessage {
				datagram: RequestResponse::Response(post_responses.to_vec()),
				proof: Proof { height: state_machine_height, proof: responses_proof },
				signer: sink.address(),
			};
			messages.push(Message::Response(msg));
		}
	}

	Ok((messages, unprofitable))
}

/// Return true for Request and Response events designated for the counterparty
pub fn filter_events(
	config: &RelayerConfig,
	router_id: StateMachine,
	counterparty: StateMachine,
	ev: &IsmpEvent,
) -> bool {
	// Is the counterparty the routing chain?
	let is_router = router_id == counterparty;

	let allow_module = |module: &[u8]| {
		config.module_filter.as_ref().is_some_and(|inner| !inner.is_empty()) &&
			is_allowed_module(config, module)
	};
	match ev {
		// We filter out events whose origin is the coprocessor unless the source module is
		// explicitly allowed in the module filter
		IsmpEvent::PostRequest(post) =>
			(post.dest == counterparty &&
				(post.source != router_id ||
					(post.source == router_id && allow_module(&post.from)))) ||
				is_router,
		IsmpEvent::PostResponse(resp) =>
			(resp.dest_chain() == counterparty &&
				(resp.source_chain() != router_id ||
					(resp.source_chain() == router_id &&
						allow_module(&resp.source_module())))) ||
				is_router,
		_ => false,
	}
}

pub fn chunk_size(state_machine: StateMachine) -> usize {
	match state_machine {
		StateMachine::Evm(_) => 100,
		_ => 200,
	}
}

#[derive(Default)]
pub struct ProfitabilityResult {
	pub queries: Vec<Option<Query>>,
	pub retriable_messages: Vec<Message>,
}

pub async fn return_successful_queries(
	sink: Arc<dyn IsmpProvider>,
	messages: Vec<Message>,
	queries: Vec<Query>,
	minimum_profit_percentage: u32,
	coprocessor: StateMachine,
	client_map: &HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	deliver_failed: bool,
) -> Result<ProfitabilityResult, anyhow::Error> {
	if messages.is_empty() {
		return Ok(Default::default());
	}

	let mut queries_to_be_relayed = Vec::new();
	let mut retriable_messages = Vec::new();
	let gas_estimates = sink.estimate_gas(messages.clone()).await?;

	// We'll be querying from possibly multiple chains, Let's use the lowest tracing batch size
	// from all clients(except the coprocessor) and use that as the max concurrency
	let batch_size = client_map
		.values()
		.into_iter()
		.map(|client| client.max_concurrent_queries())
		.min()
		.unwrap_or(1);

	for chunk in gas_estimates
		.into_iter()
		.zip(messages.into_iter())
		.zip(queries.into_iter())
		.collect::<Vec<_>>()
		.chunks(batch_size)
	{
		let processes = chunk
			.into_iter().cloned()
			.map(|((est, msg), query)| {
				let sink = sink.clone();
				let client_map = client_map.clone();
				async move {
					if !est.successful_execution && !deliver_failed {
						tracing::info!("Skipping Failed tx");
						// if msg has not been delivered return the message as retriable
						let relayer = match &msg {
							Message::Request(_) => {
								sink.query_request_receipt(query.commitment).await?
							}
							Message::Response(_) => {
								sink.query_response_receipt(query.commitment).await?
							}
							_ => unreachable!("Relayer should only ever debug trace request or response messages")
						};

						if relayer == H160::zero().0.to_vec() && coprocessor != sink.state_machine_id().state_id {
							return Ok((None, Some(msg)))
						} else {
							return Ok((None, None))
						}
					}

					let value = if coprocessor != sink.state_machine_id().state_id {
						let total_gas_to_be_expended_in_usd = est.execution_cost;
						// what kind of message is this?
						let Some(og_source)  = client_map.get(&query.source_chain) else {
							tracing::info!("Skipping tx because fee metadata cannot be queried, client for {:?} was not provided", query.source_chain);
							return Ok((None, None))
						};

						let fee_metadata = match msg {
							Message::Request(_) => og_source.query_request_fee_metadata(query.commitment).await?,
							Message::Response(_) => og_source.query_response_fee_metadata(query.commitment).await?,
							_ => Err(anyhow!("Unexpected message: {msg:?}"))?
						};

						// normalize fee_metadata to 18 decimals since gas cost is calculated in 18 decimals
						let fee_token_decimal = og_source.fee_token_decimals().await?;
						let mut fee_metadata: Cost = (fee_metadata * U256::from(10u128.pow(18u32.saturating_sub(fee_token_decimal.into()) as u32))).into();

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
							(None, Some(msg))
						} else {
							tracing::trace!(
								"Pushing tx to {:?} with cost ${fee_with_profit} and profit: ${}",
									sink.state_machine_id().state_id, Cost(profit)
							);
							(Some(query), None)
						}

					} else {
						// We only deliver sucessful messages to hyperbridge
						tracing::trace!("Pushing tx to {:?}", sink.state_machine_id().state_id);
						(Some(query), None)
					};

					return Ok::<_, anyhow::Error>(value)
				}
			})
			.collect::<FuturesOrdered<_>>();

		let results = processes.collect::<Result<Vec<_>, _>>().await?;

		for (query, unprofitable_msg) in results {
			queries_to_be_relayed.push(query);
			if let Some(msg) = unprofitable_msg {
				retriable_messages.push(msg);
			}
		}
	}

	Ok(ProfitabilityResult { queries: queries_to_be_relayed, retriable_messages })
}

fn is_allowed_module(config: &RelayerConfig, module: &[u8]) -> bool {
	match config.module_filter {
		Some(ref filters) =>
			if !filters.is_empty() {
				return filters
					.iter()
					.find(|filter| {
						hex::decode(filter.replace("0x", ""))
							.expect("Module identifier should be valid hex") ==
							module
					})
					.is_some();
			},
		// if no filter is provided, allow all modules
		_ => {},
	};

	true
}
