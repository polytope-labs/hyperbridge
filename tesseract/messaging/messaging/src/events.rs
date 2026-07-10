#![allow(dead_code)]
use anyhow::anyhow;
use codec::Encode;
use futures::stream::FuturesOrdered;
use ismp::{
	consensus::StateMachineHeight,
	events::{
		Event as IsmpEvent, Meta, RequestResponseHandled, StateMachineUpdated, TimeoutHandled,
	},
	host::StateMachine,
	messaging::{hash_request, hash_response, Message, Proof, RequestMessage, ResponseMessage},
	router::{GetResponse, PostRequest, Request},
};
use sp_core::{H160, U256};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tesseract_primitives::{config::RelayerConfig, Cost, Hasher, IsmpProvider, Query};
use tokio_stream::StreamExt;

#[derive(Debug)]
pub enum Event {
	/// Emitted when a state machine is successfully updated to a new height after the challenge
	/// period has elapsed
	StateMachineUpdated(StateMachineUpdated),
	/// An event that is emitted when a post request is dispatched
	PostRequest(Meta),
	/// An event that is emitted when a get request is dispatched
	GetRequest(Meta),
	/// An event that is emitted when a get response is dispatched
	GetResponse(Meta),
	/// Post request handled
	PostRequestHandled(RequestResponseHandled),
	/// Emitted when a post request timeout is handled
	PostRequestTimeoutHandled(TimeoutHandled),
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
			IsmpEvent::GetRequest(e) =>
				Event::GetRequest(Meta { nonce: e.nonce, dest: e.dest, source: e.source }),
			IsmpEvent::GetResponse(e) => Event::GetResponse(Meta {
				nonce: e.get.nonce,
				dest: e.get.dest,
				source: e.get.source,
			}),
			IsmpEvent::PostRequestHandled(ev) => Event::PostRequestHandled(ev),
			IsmpEvent::PostRequestTimeoutHandled(handled) =>
				Event::PostRequestTimeoutHandled(handled),
			IsmpEvent::GetRequestHandled(handled) => Event::GetRequestHandled(handled),
			IsmpEvent::GetRequestTimeoutHandled(handled) =>
				Event::GetRequestTimeoutHandled(handled),
			IsmpEvent::StateCommitmentVetoed(_) => Event::StateCommitmentVetoed,
		}
	}
}

/// Translates events emitted from [`source`] into messages to be submitted to the counterparty
/// The [`state_machine_height`] parameter is the latest available height of [`source`] on
/// the counterparty chain.
///
/// `consensus_prelude` is the source-side consensus update that will land in
/// the same batch as these messages (the outbound fan-out passes the
/// `Message::Consensus` it built from the current `ProofAccepted` event).
/// When present, it's passed through to gas estimation so each per-message
/// estimate reflects the post-update state — without it, EVM sinks would
/// simulate messages against the pre-update state commitment and either
/// misestimate or fail the success check. Callers that don't submit a
/// consensus message alongside (inbound pipeline) pass `None`.
///
/// Returns a tuple where the first item are messages to be submitted to the sink
/// and the second tuple are currently unprofitable messages.
pub async fn translate_events_to_messages(
	source: Arc<dyn IsmpProvider>,
	sink: Arc<dyn IsmpProvider>,
	events: Vec<IsmpEvent>,
	state_machine_height: StateMachineHeight,
	config: RelayerConfig,
	coprocessor: StateMachine,
	client_map: &HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	consensus_prelude: Option<Message>,
) -> Result<(Vec<Message>, Vec<Message>), anyhow::Error> {
	let mut post_request_queries = vec![];

	let mut post_requests = vec![];

	let mut request_messages = vec![];

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
									target: crate::LOG_TARGET, "Found timed out request, request: {}, counterparty: {}",
									post.timeout_timestamp,
									counterparty_timestamp.as_secs()
								);
								return Ok::<_, anyhow::Error>(None);
							}

							if !is_allowed_module(&config, &post.from) {
								tracing::trace!(
									target: crate::LOG_TARGET, "Request from module {}, filtered by module filter",
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
				_ => Err(anyhow!("Unexpected message: {msg:?}"))?,
			}
		}
	}

	let mut unprofitable = vec![];

	let (post_requests, post_request_queries) = {
		if !request_messages.is_empty() {
			tracing::trace!(
				target: crate::LOG_TARGET, "Tracing transactions to {:?}, from: {:?}",
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
			consensus_prelude.clone(),
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

		(post_request_to_push, post_request_queries_to_push)
	};

	let mut messages = vec![];

	if !post_request_queries.is_empty() {
		tracing::trace!(target: crate::LOG_TARGET, "Querying request proof for batch length {}", post_request_queries.len());
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

	// GetResponses only ever originate on the coprocessor and come back to the
	// chain that made the request. Their values already ride in the events, so
	// we gate each one for profitability and then batch the survivors into
	// chunked `handleGetResponses` calls the same way post requests are batched.
	if source.state_machine_id().state_id == coprocessor {
		let (response_messages, response_queries, responses) = build_get_response_candidates(
			&source,
			&sink,
			&events,
			&config,
			counterparty_timestamp,
			state_machine_height,
		)
		.await?;

		if !response_messages.is_empty() {
			let profitability = return_successful_queries(
				sink.clone(),
				response_messages,
				response_queries,
				config.minimum_profit_percentage,
				coprocessor,
				&client_map,
				config.deliver_failed.unwrap_or_default(),
				consensus_prelude,
			)
			.await?;

			unprofitable.extend(profitability.retriable_messages);

			let profitable = responses
				.into_iter()
				.zip(profitability.queries)
				.filter_map(|(res, query)| if query.is_some() { Some(res) } else { None })
				.collect::<Vec<_>>();

			for chunk in profitable.chunks(chunk_size(sink.state_machine_id().state_id)) {
				let commitments =
					chunk.iter().map(|res| hash_response::<Hasher>(res)).collect::<Vec<_>>();
				let proof = source
					.query_responses_proof(
						state_machine_height.height,
						commitments,
						sink.state_machine_id().state_id,
					)
					.await?;
				messages.push(Message::Response(ResponseMessage {
					requests: chunk.iter().map(|res| res.get.clone()).collect(),
					proof: Proof {
						height: state_machine_height,
						proof: (proof, chunk.to_vec()).encode(),
					},
					signer: sink.address(),
				}));
			}
		}
	}

	Ok((messages, unprofitable))
}

/// Build one delivery candidate per GetResponse in `events` whose reader is
/// `sink`, used to gate profitability before the survivors are batched. The
/// response values ride in the events, so we only fetch the membership proof.
/// Substrate sinks can't verify the mmr proof, so they are skipped.
///
/// Proofs are fetched concurrently in `source.max_concurrent_queries()`-sized
/// chunks, the same way the post request candidates above are built — a block
/// carrying N responses would otherwise cost N sequential round-trips to the
/// coprocessor on the critical path of every outbound update.
async fn build_get_response_candidates(
	source: &Arc<dyn IsmpProvider>,
	sink: &Arc<dyn IsmpProvider>,
	events: &[IsmpEvent],
	config: &RelayerConfig,
	counterparty_timestamp: Duration,
	state_machine_height: StateMachineHeight,
) -> Result<(Vec<Message>, Vec<Query>, Vec<GetResponse>), anyhow::Error> {
	let sink_state_machine = sink.state_machine_id().state_id;
	if !sink_state_machine.is_evm() {
		return Ok(Default::default());
	}

	let candidates = events
		.iter()
		.filter_map(|event| {
			let IsmpEvent::GetResponse(res) = event else { return None };
			if res.get.source != sink_state_machine {
				return None;
			}

			if res.get.timeout_timestamp != 0 &&
				res.get.timeout_timestamp <= counterparty_timestamp.as_secs()
			{
				tracing::trace!(target: crate::LOG_TARGET, "Skipping timed out get response with nonce {}", res.get.nonce);
				return None;
			}

			if !is_allowed_module(config, &res.get.from) {
				tracing::trace!(
					target: crate::LOG_TARGET, "Get response for module {}, filtered by module filter",
					hex::encode(&res.get.from),
				);
				return None;
			}

			Some(res)
		})
		.collect::<Vec<_>>();

	let mut messages = vec![];
	let mut queries = vec![];
	let mut responses = vec![];

	for chunk in candidates.chunks(source.max_concurrent_queries()) {
		let proofs = chunk
			.iter()
			.map(|res| {
				let source = source.clone();
				let response_commitment = hash_response::<Hasher>(res);
				async move {
					source
						.query_responses_proof(
							state_machine_height.height,
							vec![response_commitment],
							sink_state_machine,
						)
						.await
				}
			})
			.collect::<FuturesOrdered<_>>()
			.collect::<Result<Vec<_>, _>>()
			.await?;

		for (res, mmr_proof) in chunk.iter().zip(proofs) {
			// The `Query` describes the origin GetRequest, not the response: its commitment
			// is the request commitment, which is what the EVM host keys both the fee
			// metadata and the response receipt on.
			queries.push(Query {
				source_chain: res.get.source,
				dest_chain: res.get.dest,
				nonce: res.get.nonce,
				commitment: hash_request::<Hasher>(&Request::Get(res.get.clone())),
			});
			messages.push(Message::Response(ResponseMessage {
				requests: vec![res.get.clone()],
				proof: Proof {
					height: state_machine_height,
					proof: (mmr_proof, vec![(*res).clone()]).encode(),
				},
				signer: sink.address(),
			}));
			responses.push((*res).clone());
		}
	}

	Ok((messages, queries, responses))
}

/// Return true for Request and GetResponse events designated for the counterparty.
///
/// Events are gated by `module_filter` via `is_allowed_module`, so operators
/// can scope which modules they deliver. With no `module_filter` configured,
/// `is_allowed_module` permits every module. The on-chain reward allowlist
/// (`pallet_ismp_relayer::OutboundRequestDeliveryReward`) is applied
/// separately by the outbound task.
///
/// When the counterparty is the coprocessor, every post request flows through
/// regardless of its final destination or the module filter — the coprocessor
/// needs to ingest all requests so it can process and forward them.
pub fn filter_events(
	config: &RelayerConfig,
	counterparty: StateMachine,
	coprocessor: StateMachine,
	ev: &IsmpEvent,
) -> bool {
	match ev {
		IsmpEvent::PostRequest(post) => {
			if counterparty == coprocessor {
				return true;
			}
			post.dest == counterparty && is_allowed_module(config, &post.from)
		},
		// GetResponses only originate on the coprocessor and are delivered back to the
		// chain that made the request, so `get.source` is their destination.
		IsmpEvent::GetResponse(res) =>
			res.get.source == counterparty && is_allowed_module(config, &res.get.from),
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
	consensus_prelude: Option<Message>,
) -> Result<ProfitabilityResult, anyhow::Error> {
	if messages.is_empty() {
		return Ok(Default::default());
	}

	let mut queries_to_be_relayed = Vec::new();
	let mut retriable_messages = Vec::new();
	// Estimate each message together with the consensus update it rides in
	// with; EVM sinks use `batchCall([prelude, msg])` so the simulation sees
	// the post-update state commitment.
	let gas_estimates = sink.estimate_gas_batched(consensus_prelude, messages.clone()).await?;

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
						tracing::info!(target: crate::LOG_TARGET, "Skipping Failed tx");
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
							tracing::info!(target: crate::LOG_TARGET, "Skipping tx because fee metadata cannot be queried, client for {:?} was not provided", query.source_chain);
							return Ok((None, None))
						};

						let fee_metadata = match msg {
							// A GetResponse is gated on the fee attached to its origin GetRequest,
							// which the EVM host pays the relayer when it dispatches the response.
							// `query.commitment` is the request commitment for both message kinds.
							Message::Request(_) | Message::Response(_) =>
								og_source.query_request_fee_metadata(query.commitment).await?,
							_ => Err(anyhow!("Unexpected message: {msg:?}"))?
						};

						// normalize fee_metadata to 18 decimals since gas cost is calculated in 18 decimals
						let fee_token_decimal = og_source.fee_token_decimals().await?;
						let mut fee_metadata: Cost = (fee_metadata * U256::from(10u128.pow(18u32.saturating_sub(fee_token_decimal.into()) as u32))).into();

						let profit = (U256::from(minimum_profit_percentage) *
							total_gas_to_be_expended_in_usd.0) /
							U256::from(10000);
						// 0 profit percentage means we want to relay all requests for free
						let fee_with_profit: Cost = total_gas_to_be_expended_in_usd + profit;
						if minimum_profit_percentage == 0 {
							fee_metadata = U256::MAX.into()
						};

						if fee_metadata < fee_with_profit {
							tracing::info!(target: crate::LOG_TARGET, "Skipping unprofitable tx. Expected ${fee_with_profit}, user provided ${fee_metadata}");
							(None, Some(msg))
						} else {
							tracing::trace!(
								target: crate::LOG_TARGET, "Pushing tx to {:?} with cost ${fee_with_profit} and profit: ${}",
									sink.state_machine_id().state_id, Cost(profit)
							);
							(Some(query), None)
						}

					} else {
						// We only deliver sucessful messages to hyperbridge
						tracing::trace!(target: crate::LOG_TARGET, "Pushing tx to {:?}", sink.state_machine_id().state_id);
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
