///! This module contains the internal implementation of HyperClient.
use crate::{
	any_client::AnyClient,
	indexing::query_response_status_from_indexer,
	providers::{
		interface::{wait_for_challenge_period, Client, Query, RequestOrResponse},
		substrate::SubstrateClient,
	},
	types::{BoxStream, MessageStatusWithMetadata, PostStreamState, TimeoutStatus},
	HyperClient, Keccak256,
};
use anyhow::anyhow;
use ethers::prelude::H160;
use futures::{stream, StreamExt};
use ismp::{
	consensus::StateMachineHeight,
	messaging::{hash_request, Message, Proof, RequestMessage, ResponseMessage, TimeoutMessage},
	router::{Post, PostResponse, Request, Response},
};
use sp_core::H256;
use subxt_utils::Hyperbridge;

use crate::indexing::query_request_status_from_indexer;
use ismp::events::Event;
use std::time::Duration;

/// `query_request_status_internal` is an internal function that
/// checks the status of a message
pub async fn query_request_status_internal(
	client: &HyperClient,
	post: Post,
) -> Result<MessageStatusWithMetadata, anyhow::Error> {
	let destination_current_timestamp = client.dest.query_timestamp().await?;
	let req = Request::Post(post.clone());
	let hash = hash_request::<Keccak256>(&req);
	let relayer_address = client.dest.query_request_receipt(hash).await?;
	if let Some(ref status) = query_request_status_from_indexer(Request::Post(post.clone()), client)
		.await
		.ok()
		.flatten()
	{
		return Ok(status.clone());
	}
	if relayer_address != H160::zero() {
		// This means the message has gotten the destination chain
		return Ok(MessageStatusWithMetadata::DestinationDelivered { meta: Default::default() });
	}

	// Checking to see if the messaging has timed-out
	if destination_current_timestamp.as_secs() >= post.timeout_timestamp {
		// request timed out before reaching the destination chain
		return Ok(MessageStatusWithMetadata::Timeout);
	}

	let hyperbridge_current_timestamp = client.hyperbridge.query_timestamp().await?;
	let relayer = client.hyperbridge.query_request_receipt(hash).await?;

	if relayer != H160::zero() {
		return Ok(MessageStatusWithMetadata::HyperbridgeDelivered { meta: Default::default() });
	}

	if hyperbridge_current_timestamp.as_secs() > post.timeout_timestamp {
		// the request timed out before getting to hyper bridge
		return Ok(MessageStatusWithMetadata::Timeout);
	}

	Ok(MessageStatusWithMetadata::Pending)
}

/// `query_response_status_internal` function returns the status of a response
pub async fn query_response_status_internal(
	hyperclient: &HyperClient,
	post_response: PostResponse,
) -> Result<MessageStatusWithMetadata, anyhow::Error> {
	let response_destination_timeout = hyperclient.dest.query_timestamp().await?;
	let res = Response::Post(post_response.clone());
	let req_hash = hash_request::<Keccak256>(&res.request());
	let response_receipt_relayer = hyperclient.dest.query_response_receipt(req_hash).await?;
	if let Some(ref status) =
		query_response_status_from_indexer(Response::Post(post_response.clone()), hyperclient)
			.await
			.ok()
			.flatten()
	{
		return Ok(status.clone());
	}
	if response_receipt_relayer != H160::zero() {
		return Ok(MessageStatusWithMetadata::DestinationDelivered { meta: Default::default() });
	}

	if response_destination_timeout.as_secs() > post_response.timeout_timestamp {
		// response timed out before reaching the destination chain
		return Ok(MessageStatusWithMetadata::Timeout);
	}

	let relayer = hyperclient.hyperbridge.query_response_receipt(req_hash).await?;

	if relayer != H160::zero() {
		return Ok(MessageStatusWithMetadata::HyperbridgeDelivered { meta: Default::default() });
	}

	let hyperbridge_current_timestamp = hyperclient.hyperbridge.latest_timestamp().await?;

	if hyperbridge_current_timestamp.as_secs() > post_response.timeout_timestamp {
		// the request timed out before getting to hyper bridge
		return Ok(MessageStatusWithMetadata::Timeout);
	}

	Ok(MessageStatusWithMetadata::Pending)
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TimeoutStreamState {
	Pending,
	/// Destination state machine has been finalized on hyperbridge
	DestinationFinalized(u64),
	/// Message has been timed out on hyperbridge
	HyperbridgeTimedout(u64),
	/// Hyperbridge has been finalized on source chain
	HyperbridgeFinalized(u64),
	/// Stream has ended
	End,
}

/// Handles the timeout process internally and yields the encoded transaction data to be submitted
/// to the source chain This future does not check the request timeout status, only call it after
/// you have confirmed the request timeout status using `query_request_status`
pub async fn timeout_request_stream(
	hyperclient: &HyperClient,
	post: Post,
) -> Result<BoxStream<TimeoutStatus>, anyhow::Error> {
	let dest_client = hyperclient.dest.clone();
	let hyperbridge_client = hyperclient.hyperbridge.clone();
	let source_client = hyperclient.source.clone();

	let stream = stream::unfold(TimeoutStreamState::Pending, move |state| {
		let dest_client = dest_client.clone();
		let hyperbridge_client = hyperbridge_client.clone();
		let source_client = source_client.clone();
		let req = Request::Post(post.clone());
		let hash = hash_request::<Keccak256>(&req);

		async move {
			let lambda = || async {
				match state {
					TimeoutStreamState::Pending => {
						let relayer = hyperbridge_client.query_request_receipt(hash).await?;
						if relayer != H160::zero() {
							let height = hyperbridge_client
								.query_latest_state_machine_height(dest_client.state_machine_id())
								.await?;

							let state_commitment = hyperbridge_client
								.query_state_machine_commitment(StateMachineHeight {
									id: dest_client.state_machine_id(),
									height,
								})
								.await?;

							if state_commitment.timestamp > post.timeout_timestamp {
								// early return if the destination has already finalized the height
								return Ok(Some((
									Ok(TimeoutStatus::DestinationFinalized {
										meta: Default::default(),
									}),
									TimeoutStreamState::DestinationFinalized(height),
								)));
							}

							let mut stream = hyperbridge_client
								.state_machine_update_notification(dest_client.state_machine_id())
								.await?;
							let mut valid_proof_height = None;
							while let Some(event) = stream.next().await {
								match event {
									Ok(ev) => {
										let state_machine_height = StateMachineHeight {
											id: ev.event.state_machine_id,
											height: ev.event.latest_height,
										};
										let commitment = hyperbridge_client
											.query_state_machine_commitment(state_machine_height)
											.await?;
										if commitment.timestamp > post.timeout_timestamp {
											valid_proof_height = Some(ev);
											break;
										}
									},
									Err(e) =>
										return Ok(Some((
											Err(anyhow!(
												"Encountered error in time out stream {e:?}"
											)),
											state,
										))),
								}
							}
							Ok(valid_proof_height.map(|ev| {
								(
									Ok(TimeoutStatus::DestinationFinalized { meta: ev.meta }),
									TimeoutStreamState::DestinationFinalized(
										ev.event.latest_height,
									),
								)
							}))
						} else {
							let height = hyperbridge_client.query_latest_block_height().await?;
							Ok(Some((
								Ok(TimeoutStatus::HyperbridgeTimedout { meta: Default::default() }),
								TimeoutStreamState::HyperbridgeTimedout(height),
							)))
						}
					},
					TimeoutStreamState::DestinationFinalized(proof_height) => {
						let storage_key = dest_client.request_receipt_full_key(hash);
						let proof =
							dest_client.query_state_proof(proof_height, vec![storage_key]).await?;
						let height = StateMachineHeight {
							id: dest_client.state_machine_id(),
							height: proof_height,
						};
						let message = Message::Timeout(TimeoutMessage::Post {
							requests: vec![req.clone()],
							timeout_proof: Proof { height, proof },
						});
						let challenge_period = hyperbridge_client
							.query_challenge_period(
								dest_client.state_machine_id().consensus_state_id,
							)
							.await?;
						let update_time =
							hyperbridge_client.query_state_machine_update_time(height).await?;
						wait_for_challenge_period(
							&hyperbridge_client,
							update_time,
							challenge_period,
						)
						.await?;
						let meta = hyperbridge_client.submit(message).await?;
						Ok(Some((
							Ok(TimeoutStatus::HyperbridgeTimedout { meta }),
							TimeoutStreamState::HyperbridgeTimedout(meta.block_number),
						)))
					},
					TimeoutStreamState::HyperbridgeTimedout(hyperbridge_height) => {
						let latest_hyperbridge_height = source_client
							.query_latest_state_machine_height(
								hyperbridge_client.state_machine_id(),
							)
							.await?;
						// check if the height has already been finalized
						if latest_hyperbridge_height >= hyperbridge_height {
							let latest_height = source_client.query_latest_block_height().await?;
							let meta = source_client
								.query_ismp_event((latest_height - 500)..=latest_height)
								.await?
								.into_iter()
								.find_map(|event| match event.event {
									Event::StateMachineUpdated(updated)
										if updated.latest_height >= hyperbridge_height =>
										Some(event.meta),
									_ => None,
								});

							let Some(meta) = meta else {
								return Ok(Some((
									Ok(TimeoutStatus::HyperbridgeFinalized {
										meta: Default::default(),
									}),
									TimeoutStreamState::HyperbridgeFinalized(latest_height),
								)));
							};

							return Ok(Some((
								Ok(TimeoutStatus::HyperbridgeFinalized { meta: meta.clone() }),
								TimeoutStreamState::HyperbridgeFinalized(meta.block_number),
							)));
						}

						let mut state_machine_update_stream = source_client
							.state_machine_update_notification(
								hyperbridge_client.state_machine_id(),
							)
							.await?;

						let mut valid_proof_height = None;
						while let Some(event) = state_machine_update_stream.next().await {
							match event {
								Ok(ev) => {
									let state_machine_height = StateMachineHeight {
										id: ev.event.state_machine_id,
										height: ev.event.latest_height,
									};
									let commitment = source_client
										.query_state_machine_commitment(state_machine_height)
										.await?;
									if commitment.timestamp > post.timeout_timestamp &&
										ev.event.latest_height >= hyperbridge_height
									{
										valid_proof_height = Some(ev);
										break;
									}
								},
								Err(e) =>
									return Ok(Some((
										Err(anyhow!("Encountered error in time out stream {e:?}")),
										state,
									))),
							}
						}

						Ok(valid_proof_height.map(|event| {
							(
								Ok(TimeoutStatus::HyperbridgeFinalized { meta: event.meta }),
								TimeoutStreamState::HyperbridgeFinalized(event.event.latest_height),
							)
						}))
					},
					TimeoutStreamState::HyperbridgeFinalized(proof_height) => {
						let storage_key = hyperbridge_client.request_receipt_full_key(hash);
						let proof = hyperbridge_client
							.query_state_proof(proof_height, vec![storage_key])
							.await?;
						let height = StateMachineHeight {
							id: hyperbridge_client.state_machine,
							height: proof_height,
						};
						let message = Message::Timeout(TimeoutMessage::Post {
							requests: vec![req],
							timeout_proof: Proof { height, proof },
						});
						let challenge_period = source_client
							.query_challenge_period(
								hyperbridge_client.state_machine_id().consensus_state_id,
							)
							.await?;
						let update_time =
							source_client.query_state_machine_update_time(height).await?;
						wait_for_challenge_period(&source_client, update_time, challenge_period)
							.await?;
						let calldata = source_client.encode(message)?;

						Ok(Some((
							Ok(TimeoutStatus::TimeoutMessage { calldata: calldata.into() }),
							TimeoutStreamState::End,
						)))
					},
					TimeoutStreamState::End => Ok::<_, anyhow::Error>(None),
				}
			};

			lambda().await.unwrap_or_else(|e| {
				Some((
					Err(anyhow!("Encountered an error in stream {e:?}")),
					TimeoutStreamState::End,
				))
			})
		}
	});

	Ok(Box::pin(stream))
}

/// returns the query stream for a post
pub async fn request_status_stream(
	hyperclient: &HyperClient,
	post: Post,
	post_request_height: u64,
) -> BoxStream<MessageStatusWithMetadata> {
	let source_client = hyperclient.source.clone();
	let dest_client = hyperclient.dest.clone();
	let hyperbridge_client = hyperclient.hyperbridge.clone();
	let hyperclient_clone = hyperclient.clone();

	let stream = stream::unfold(PostStreamState::Pending, move |post_request_status| {
		let dest_client = dest_client.clone();
		let hyperbridge_client = hyperbridge_client.clone();
		let source_client = source_client.clone();
		let req = Request::Post(post.clone());
		let hash = hash_request::<Keccak256>(&req);
		let post = post.clone();
		let hyperclient_clone = hyperclient_clone.clone();
		async move {
			let lambda = || async {
				match post_request_status {
					PostStreamState::Pending => {
						let destination_current_timestamp = dest_client.query_timestamp().await?;
						let relayer_address = dest_client.query_request_receipt(hash).await?;

						if let Some(ref msg_status) =
							query_request_status_from_indexer(req.clone(), &hyperclient_clone)
								.await
								.ok()
								.flatten()
						{
							match msg_status {
								MessageStatusWithMetadata::SourceFinalized {
									finalized_height,
									..
								} => {
									return Ok::<
										Option<(Result<_, anyhow::Error>, PostStreamState)>,
										anyhow::Error,
									>(Some((
										Ok(msg_status.clone()),
										PostStreamState::SourceFinalized(*finalized_height),
									)));
								},
								MessageStatusWithMetadata::HyperbridgeDelivered { meta } => {
									return Ok::<
										Option<(Result<_, anyhow::Error>, PostStreamState)>,
										anyhow::Error,
									>(Some((
										Ok(msg_status.clone()),
										PostStreamState::HyperbridgeDelivered(meta.block_number),
									)));
								},
								MessageStatusWithMetadata::HyperbridgeFinalized {
									finalized_height,
									..
								} => {
									return Ok::<
										Option<(Result<_, anyhow::Error>, PostStreamState)>,
										anyhow::Error,
									>(Some((
										Ok(msg_status.clone()),
										PostStreamState::HyperbridgeFinalized(*finalized_height),
									)));
								},
								MessageStatusWithMetadata::DestinationDelivered { .. } => {
									return Ok::<
										Option<(Result<_, anyhow::Error>, PostStreamState)>,
										anyhow::Error,
									>(Some((
										Ok(msg_status.clone()),
										PostStreamState::DestinationDelivered,
									)));
								},
								_ => {},
							}
						}

						if relayer_address != H160::zero() {
							// This means the message has gotten to the destination chain
							return Ok::<
								Option<(Result<_, anyhow::Error>, PostStreamState)>,
								anyhow::Error,
							>(Some((
								Ok(MessageStatusWithMetadata::DestinationDelivered {
									meta: Default::default(),
								}),
								PostStreamState::End,
							)));
						}

						if destination_current_timestamp.as_secs() >= post.timeout_timestamp {
							// Checking to see if the message has timed-out
							return Ok(Some((
								Ok(MessageStatusWithMetadata::Timeout),
								PostStreamState::End,
							)));
						}

						let hyperbridge_current_timestamp =
							hyperbridge_client.query_timestamp().await?;
						let relayer = hyperbridge_client.query_request_receipt(hash).await?;

						if relayer != H160::zero() {
							// This means the message has gotten to the destination chain
							return Ok::<
								Option<(Result<_, anyhow::Error>, PostStreamState)>,
								anyhow::Error,
							>(Some((
								Ok(MessageStatusWithMetadata::HyperbridgeDelivered {
									meta: Default::default(),
								}),
								PostStreamState::HyperbridgeDelivered(
									hyperbridge_client.query_latest_block_height().await?,
								),
							)));
						}

						if hyperbridge_current_timestamp.as_secs() >= post.timeout_timestamp {
							// Checking to see if the message has timed-out
							return Ok(Some((
								Ok(MessageStatusWithMetadata::Timeout),
								PostStreamState::End,
							)));
						}

						let mut state_machine_updated_stream = hyperbridge_client
							.state_machine_update_notification(source_client.state_machine_id())
							.await?;

						while let Some(item) = state_machine_updated_stream.next().await {
							match item {
								Ok(state_machine_update) => {
									if state_machine_update.event.latest_height >=
										post_request_height && state_machine_update
										.event
										.state_machine_id
										.state_id == post.source
									{
										return Ok(Some((
											Ok(MessageStatusWithMetadata::SourceFinalized {
												finalized_height: state_machine_update
													.event
													.latest_height,
												meta: state_machine_update.meta,
											}),
											PostStreamState::SourceFinalized(
												state_machine_update.meta.block_number,
											),
										)));
									}
								},
								Err(e) =>
									return Ok(Some((
										Err(anyhow!(
											"Encountered an error {:?}: in {:?}",
											PostStreamState::Pending,
											e
										)),
										post_request_status,
									))),
							};
						}

						Ok(None)
					},

					PostStreamState::SourceFinalized(finalized_height) => {
						let relayer = hyperbridge_client.query_request_receipt(hash).await?;

						if let Some(ref msg_status) =
							query_request_status_from_indexer(req.clone(), &hyperclient_clone)
								.await
								.ok()
								.flatten()
						{
							match msg_status {
								MessageStatusWithMetadata::HyperbridgeDelivered { meta } => {
									return Ok::<
										Option<(Result<_, anyhow::Error>, PostStreamState)>,
										anyhow::Error,
									>(Some((
										Ok(msg_status.clone()),
										PostStreamState::HyperbridgeDelivered(meta.block_number),
									)));
								},
								MessageStatusWithMetadata::HyperbridgeFinalized {
									finalized_height,
									..
								} => {
									return Ok::<
										Option<(Result<_, anyhow::Error>, PostStreamState)>,
										anyhow::Error,
									>(Some((
										Ok(msg_status.clone()),
										PostStreamState::HyperbridgeFinalized(*finalized_height),
									)));
								},
								MessageStatusWithMetadata::DestinationDelivered { .. } => {
									return Ok::<
										Option<(Result<_, anyhow::Error>, PostStreamState)>,
										anyhow::Error,
									>(Some((
										Ok(msg_status.clone()),
										PostStreamState::DestinationDelivered,
									)));
								},
								_ => {},
							}
						}

						if relayer != H160::zero() {
							let latest_height =
								hyperbridge_client.query_latest_block_height().await?;

							let meta = dest_client
								.query_ismp_event(finalized_height..=latest_height)
								.await?
								.into_iter()
								.find_map(|event| match event.event {
									Event::PostRequest(post_event)
										if post.source == post_event.source &&
											post.nonce == post_event.nonce =>
										Some(event.meta),
									_ => None,
								});

							return Ok(Some((
								Ok(MessageStatusWithMetadata::HyperbridgeDelivered {
									meta: meta.unwrap_or_default(),
								}),
								PostStreamState::HyperbridgeDelivered(
									meta.map(|m| m.block_number).unwrap_or(latest_height),
								),
							)));
						}

						let mut stream = hyperbridge_client
							.ismp_events_stream(
								RequestOrResponse::Request(post.clone()),
								finalized_height,
							)
							.await?;
						while let Some(event) = stream.next().await {
							match event {
								Ok(event) => {
									return Ok(Some((
										Ok(MessageStatusWithMetadata::HyperbridgeDelivered {
											meta: event.meta.clone(),
										}),
										PostStreamState::HyperbridgeDelivered(
											event.meta.block_number,
										),
									)));
								},
								Err(e) => tracing::info!(
									"Encountered waiting for message on hyperbridge: {e:?}"
								),
							}
						}

						Ok(None)
					},

					PostStreamState::HyperbridgeDelivered(height) => {
						let res = dest_client.query_request_receipt(hash).await?;

						if let Some(ref msg_status) =
							query_request_status_from_indexer(req.clone(), &hyperclient_clone)
								.await
								.ok()
								.flatten()
						{
							match msg_status {
								MessageStatusWithMetadata::HyperbridgeFinalized {
									finalized_height,
									..
								} => {
									return Ok::<
										Option<(Result<_, anyhow::Error>, PostStreamState)>,
										anyhow::Error,
									>(Some((
										Ok(msg_status.clone()),
										PostStreamState::HyperbridgeFinalized(*finalized_height),
									)));
								},
								MessageStatusWithMetadata::DestinationDelivered { .. } => {
									return Ok::<
										Option<(Result<_, anyhow::Error>, PostStreamState)>,
										anyhow::Error,
									>(Some((
										Ok(msg_status.clone()),
										PostStreamState::DestinationDelivered,
									)));
								},
								_ => {},
							}
						}

						if res != H160::zero() {
							return Ok(Some((
								Ok(MessageStatusWithMetadata::DestinationDelivered {
									meta: Default::default(),
								}),
								PostStreamState::End,
							)));
						}

						let latest_hyperbridge_height = dest_client
							.query_latest_state_machine_height(
								hyperbridge_client.state_machine_id(),
							)
							.await?;
						// check if the height has already been finalized
						if latest_hyperbridge_height >= height {
							let latest_height = dest_client.query_latest_block_height().await?;
							let meta = dest_client
								.query_ismp_event((latest_height - 500)..=latest_height)
								.await?
								.into_iter()
								.find_map(|event| match event.event {
									Event::StateMachineUpdated(updated)
										if updated.latest_height >= height =>
										Some((event.meta, updated)),
									_ => None,
								});

							let Some((meta, update)) = meta else {
								let calldata =
									encode_request_message_and_wait_for_challenge_period(
										&hyperbridge_client,
										&dest_client,
										post.clone(),
										hash,
										latest_hyperbridge_height,
									)
									.await?;
								return Ok(Some((
									Ok(MessageStatusWithMetadata::HyperbridgeFinalized {
										finalized_height: height,
										meta: Default::default(),
										calldata: calldata.into(),
									}),
									PostStreamState::HyperbridgeFinalized(latest_height),
								)));
							};

							let calldata = encode_request_message_and_wait_for_challenge_period(
								&hyperbridge_client,
								&dest_client,
								post.clone(),
								hash,
								update.latest_height,
							)
							.await?;

							return Ok(Some((
								Ok(MessageStatusWithMetadata::HyperbridgeFinalized {
									finalized_height: update.latest_height,
									meta: meta.clone(),
									calldata: calldata.into(),
								}),
								PostStreamState::HyperbridgeFinalized(meta.block_number),
							)));
						}

						let mut stream = dest_client
							.state_machine_update_notification(
								hyperbridge_client.state_machine_id(),
							)
							.await?;
						while let Some(update) = stream.next().await {
							match update {
								Ok(event) =>
									if event.event.latest_height >= height {
										let calldata =
											encode_request_message_and_wait_for_challenge_period(
												&hyperbridge_client,
												&dest_client,
												post.clone(),
												hash,
												event.event.latest_height,
											)
											.await?;
										return Ok(Some((
											Ok(MessageStatusWithMetadata::HyperbridgeFinalized {
												finalized_height: event.event.latest_height,
												meta: event.meta,
												calldata: calldata.into(),
											}),
											PostStreamState::HyperbridgeFinalized(
												event.meta.block_number,
											),
										)));
									} else {
										continue;
									},
								Err(e) =>
									return Ok(Some((
										Err(anyhow!(
											"Encountered an error {:?}: in {:?}",
											PostStreamState::HyperbridgeDelivered(height),
											e
										)),
										post_request_status,
									))),
							}
						}
						Ok(None)
					},

					PostStreamState::HyperbridgeFinalized(finalized_height) => {
						let res = dest_client.query_request_receipt(hash).await?;

						if let Some(msg_status) =
							query_request_status_from_indexer(req.clone(), &hyperclient_clone)
								.await
								.ok()
								.flatten()
						{
							match &msg_status {
								MessageStatusWithMetadata::DestinationDelivered { .. } => {
									return Ok::<
										Option<(Result<_, anyhow::Error>, PostStreamState)>,
										anyhow::Error,
									>(Some((
										Ok(msg_status.clone()),
										PostStreamState::DestinationDelivered,
									)));
								},
								_ => {},
							}
						}
						let request_commitment =
							hash_request::<Keccak256>(&Request::Post(post.clone()));
						if res != H160::zero() {
							let latest_height = dest_client.query_latest_block_height().await?;
							let meta = dest_client
								.query_ismp_event(finalized_height..=latest_height)
								.await?
								.into_iter()
								.find_map(|event| match event.event {
									Event::PostRequestHandled(handled)
										if handled.commitment == request_commitment =>
										Some(event.meta),
									_ => None,
								})
								.unwrap_or_default();
							return Ok(Some((
								Ok(MessageStatusWithMetadata::DestinationDelivered { meta }),
								PostStreamState::DestinationDelivered,
							)));
						}
						let mut stream =
							dest_client.post_request_handled_stream(hash, finalized_height).await?;

						while let Some(event) = stream.next().await {
							match event {
                                Ok(event) => return Ok(Some((
                                    Ok(MessageStatusWithMetadata::DestinationDelivered {
                                        meta: event.meta,
                                    }),
                                    PostStreamState::DestinationDelivered,
                                ))),
                                Err(e) =>  tracing::info!(
                                    "Encountered an error waiting for message delivery to destination {e:?}",
                                ),
                            }
						}

						Ok(None)
					},

					PostStreamState::DestinationDelivered | PostStreamState::End =>
						Ok::<Option<(Result<_, _>, PostStreamState)>, anyhow::Error>(None),
				}
			};

			// terminate the stream once an error is encountered
			lambda().await.unwrap_or_else(|e| {
				Some((Err(anyhow!("Encountered an error in stream {e:?}")), PostStreamState::End))
			})
		}
	});

	Box::pin(stream)
}

/// This returns a stream that yields when the provided timeout value is reached on the chain for
/// the provided [`Client`]
pub async fn request_timeout_stream(
	timeout: u64,
	client: impl Client + Clone,
) -> BoxStream<MessageStatusWithMetadata> {
	let stream = stream::unfold(client, move |client| async move {
		let lambda = || async {
			let current_timestamp = client.query_timestamp().await?.as_secs();

			return if current_timestamp > timeout {
				Ok(true)
			} else {
				let sleep_time = timeout - current_timestamp;
				let _ = wasm_timer::Delay::new(Duration::from_secs(sleep_time)).await;
				Ok::<_, anyhow::Error>(false)
			};
		};

		let response = lambda().await;

		let value = match response {
			Ok(true) => Some((Ok(Some(MessageStatusWithMetadata::Timeout)), client)),
			Ok(false) => Some((Ok(None), client)),
			Err(e) =>
				Some((Err(anyhow!("Encountered an error in timeout stream: {:?}", e)), client)),
		};

		return value;
	})
	.filter_map(|item| async move {
		match item {
			Ok(None) => None,
			Ok(Some(event)) => Some(Ok(event)),
			Err(err) => Some(Err(err)),
		}
	});

	Box::pin(stream)
}

pub async fn encode_request_call_data(
	hyperbridge: &SubstrateClient<Hyperbridge>,
	dest_client: &AnyClient,
	post: Post,
	commitment: H256,
	height: u64,
) -> Result<Vec<u8>, anyhow::Error> {
	let proof = hyperbridge
		.query_requests_proof(
			height,
			vec![Query { source_chain: post.source, dest_chain: post.dest, commitment }],
		)
		.await?;
	let proof_height = StateMachineHeight { id: hyperbridge.state_machine, height };

	let message = Message::Request(RequestMessage {
		requests: vec![post.clone()],
		proof: Proof { height: proof_height, proof },
		signer: H160::zero().0.to_vec(),
	});
	let calldata = dest_client.encode(message)?;
	Ok(calldata)
}

pub async fn encode_response_call_data(
	hyperbridge: &SubstrateClient<Hyperbridge>,
	dest_client: &AnyClient,
	post_response: PostResponse,
	commitment: H256,
	height: u64,
) -> Result<Vec<u8>, anyhow::Error> {
	let proof = hyperbridge
		.query_responses_proof(
			height,
			vec![Query {
				source_chain: post_response.post.dest,
				dest_chain: post_response.post.source,
				commitment,
			}],
		)
		.await?;
	let proof_height = StateMachineHeight { id: hyperbridge.state_machine, height };

	let message = Message::Response(ResponseMessage {
		datagram: ismp::router::RequestResponse::Response(vec![Response::Post(post_response)]),
		proof: Proof { height: proof_height, proof },
		signer: H160::zero().0.to_vec(),
	});
	let calldata = dest_client.encode(message)?;
	Ok(calldata)
}
// Encodes the call data for the message but waits for the challenge period before yielding
pub async fn encode_request_message_and_wait_for_challenge_period(
	hyperbridge: &SubstrateClient<Hyperbridge>,
	dest_client: &AnyClient,
	post: Post,
	commitment: H256,
	height: u64,
) -> Result<Vec<u8>, anyhow::Error> {
	let calldata =
		encode_request_call_data(hyperbridge, dest_client, post, commitment, height).await?;
	let proof_height = StateMachineHeight { id: hyperbridge.state_machine, height };
	let challenge_period = dest_client
		.query_challenge_period(hyperbridge.state_machine_id().consensus_state_id)
		.await?;
	let update_time = dest_client.query_state_machine_update_time(proof_height).await?;
	wait_for_challenge_period(dest_client, update_time, challenge_period).await?;

	Ok(calldata)
}
