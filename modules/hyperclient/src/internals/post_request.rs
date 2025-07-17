// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
	internals::encode_request_message_and_wait_for_challenge_period,
	providers::interface::{wait_for_challenge_period, Client},
	types::{
		BoxStream, MessageStatusStreamState, MessageStatusWithMetadata, TimeoutStatus,
		TimeoutStreamState,
	},
	HyperClient, Keccak256,
};
use anyhow::anyhow;
use futures::{stream, StreamExt};
use ismp::{
	consensus::StateMachineHeight,
	events::Event,
	messaging::{hash_request, Message, Proof, TimeoutMessage},
	router::{PostRequest, Request},
};
use primitive_types::H160;

/// `query_request_status_internal` is an internal function that
/// checks the status of a message
pub async fn query_post_request_status_internal(
	client: &HyperClient,
	post: PostRequest,
) -> Result<MessageStatusWithMetadata, anyhow::Error> {
	let dest_client = if post.dest == client.dest.state_machine_id().state_id {
		&client.dest
	} else if post.dest == client.source.state_machine_id().state_id {
		&client.source
	} else {
		Err(anyhow!("Unknown client for {}", post.source))?
	};

	let destination_current_timestamp = dest_client.query_timestamp().await?;
	let req = Request::Post(post.clone());
	let hash = hash_request::<Keccak256>(&req);
	let relayer_address = dest_client.query_request_receipt(hash).await?;

	if relayer_address != H160::zero() {
		// This means the message has gotten the destination chain
		return Ok(MessageStatusWithMetadata::DestinationDelivered { meta: Default::default() });
	}

	// Checking to see if the messaging has timed-out
	if destination_current_timestamp.as_secs() >= post.timeout().as_secs() {
		// request timed out before reaching the destination chain
		return Ok(MessageStatusWithMetadata::Timeout);
	}

	let hyperbridge_current_timestamp = client.hyperbridge.query_timestamp().await?;
	let relayer = client.hyperbridge.query_request_receipt(hash).await?;

	if relayer != H160::zero() {
		return Ok(MessageStatusWithMetadata::HyperbridgeVerified { meta: Default::default() });
	}

	if hyperbridge_current_timestamp.as_secs() > post.timeout().as_secs() {
		// the request timed out before getting to hyper bridge
		return Ok(MessageStatusWithMetadata::Timeout);
	}

	Ok(MessageStatusWithMetadata::Pending)
}

/// returns the query stream for a post
pub async fn post_request_status_stream(
	hyperclient: &HyperClient,
	post: PostRequest,
	state: MessageStatusStreamState,
) -> Result<BoxStream<MessageStatusWithMetadata>, anyhow::Error> {
	let source_client = if post.source == hyperclient.dest.state_machine_id().state_id {
		hyperclient.dest.clone()
	} else if post.source == hyperclient.source.state_machine_id().state_id {
		hyperclient.source.clone()
	} else {
		Err(anyhow!("Unknown client for source: {}", post.source))?
	};
	let dest_client = if post.dest == hyperclient.dest.state_machine_id().state_id {
		hyperclient.dest.clone()
	} else if post.dest == hyperclient.source.state_machine_id().state_id {
		hyperclient.source.clone()
	} else {
		Err(anyhow!("Unknown client for dest: {}", post.dest))?
	};
	let hyperbridge_client = hyperclient.hyperbridge.clone();

	let stream = stream::unfold(state, move |post_request_status| {
		let dest_client = dest_client.clone();
		let hyperbridge_client = hyperbridge_client.clone();
		let source_client = source_client.clone();
		let req = Request::Post(post.clone());
		let hash = hash_request::<Keccak256>(&req);
		let post = post.clone();
		async move {
			let lambda = || async {
				match post_request_status {
					MessageStatusStreamState::Dispatched(post_request_height) => {
						tracing::trace!("Entered state: Dispatched({post_request_height:?})");

						let dest_finalized_height = hyperbridge_client
							.query_latest_state_machine_height(source_client.state_machine_id())
							.await?;
						if dest_finalized_height >= post_request_height {
							// we don't actually know when the block was finalized, so use this
							// instead
							let finalized_height =
								hyperbridge_client.query_latest_block_height().await?;
							return Ok(Some((
								Ok(MessageStatusWithMetadata::SourceFinalized {
									meta: Default::default(),
									finalized_height: dest_finalized_height,
								}),
								MessageStatusStreamState::SourceFinalized(finalized_height),
							)));
						}

						let mut state_machine_updated_stream = hyperbridge_client
							.state_machine_update_notification(source_client.state_machine_id())
							.await?;

						while let Some(item) = state_machine_updated_stream.next().await {
							match item {
								Ok(state_machine_update) => {
									if state_machine_update.event.latest_height >=
										post_request_height
									{
										return Ok(Some((
											Ok(MessageStatusWithMetadata::SourceFinalized {
												finalized_height: state_machine_update
													.event
													.latest_height,
												meta: state_machine_update.meta,
											}),
											MessageStatusStreamState::SourceFinalized(
												state_machine_update.meta.block_number,
											),
										)));
									}
								},
								Err(e) =>
									return Ok(Some((
										Err(anyhow!(
											"Encountered an error {:?}: in {:?}",
											MessageStatusStreamState::Dispatched(
												post_request_height
											),
											e
										)),
										post_request_status,
									))),
							};
						}

						Ok(None)
					},
					MessageStatusStreamState::SourceFinalized(finalized_height) => {
						tracing::trace!("Entered state: SourceFinalized({finalized_height:?})");

						let relayer = hyperbridge_client.query_request_receipt(hash).await?;

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
								Ok(MessageStatusWithMetadata::HyperbridgeVerified {
									meta: meta.unwrap_or_default(),
								}),
								MessageStatusStreamState::HyperbridgeVerified(
									meta.map(|m| m.block_number).unwrap_or(latest_height),
								),
							)));
						}

						let mut stream =
							hyperbridge_client.ismp_events_stream(hash, finalized_height).await?;
						while let Some(event) = stream.next().await {
							match event {
								Ok(event) => {
									return Ok(Some((
										Ok(MessageStatusWithMetadata::HyperbridgeVerified {
											meta: event.meta.clone(),
										}),
										MessageStatusStreamState::HyperbridgeVerified(
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
					MessageStatusStreamState::HyperbridgeVerified(height) => {
						tracing::trace!("Entered state: HyperbridgeVerified({height:?})");

						let res = dest_client.query_request_receipt(hash).await?;
						if res != H160::zero() {
							return Ok(Some((
								Ok(MessageStatusWithMetadata::DestinationDelivered {
									meta: Default::default(),
								}),
								MessageStatusStreamState::End,
							)));
						}

						let latest_hyperbridge_height = dest_client
							.query_latest_state_machine_height(
								hyperbridge_client.state_machine_id(),
							)
							.await?;

						tracing::trace!(
							"\n\n\nLatest hyperbridge height: {latest_hyperbridge_height}"
						);

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
									MessageStatusStreamState::HyperbridgeFinalized(latest_height),
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
								MessageStatusStreamState::HyperbridgeFinalized(meta.block_number),
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
											MessageStatusStreamState::HyperbridgeFinalized(
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
											MessageStatusStreamState::HyperbridgeVerified(height),
											e
										)),
										post_request_status,
									))),
							}
						}
						Ok(None)
					},
					MessageStatusStreamState::HyperbridgeFinalized(finalized_height) => {
						tracing::trace!(
							"Entered state: HyperbridgeFinalized({finalized_height:?})"
						);
						let res = dest_client.query_request_receipt(hash).await?;

						if res != H160::zero() {
							let request_commitment =
								hash_request::<Keccak256>(&Request::Post(post.clone()));
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
								MessageStatusStreamState::DestinationDelivered,
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
									MessageStatusStreamState::DestinationDelivered,
                                ))),
                                Err(e) =>  tracing::info!(
                                    "Encountered an error waiting for message delivery to destination {e:?}",
                                ),
                            }
						}

						Ok(None)
					},
					MessageStatusStreamState::DestinationDelivered |
					MessageStatusStreamState::End =>
						Ok::<Option<(Result<_, _>, MessageStatusStreamState)>, anyhow::Error>(None),
				}
			};

			// terminate the stream once an error is encountered
			lambda().await.unwrap_or_else(|e| {
				Some((
					Err(anyhow!("Encountered an error in stream {e:?}")),
					MessageStatusStreamState::End,
				))
			})
		}
	});

	Ok(Box::pin(stream))
}

// Handles the timeout process internally and yields the encoded transaction data to be submitted
/// to the source chain This future does not check the request timeout status, only call it after
/// you have confirmed the request timeout status using `query_request_status`
pub async fn timeout_post_request_stream(
	hyperclient: &HyperClient,
	post: PostRequest,
	state: TimeoutStreamState,
) -> Result<BoxStream<TimeoutStatus>, anyhow::Error> {
	let source_client = if post.source == hyperclient.dest.state_machine_id().state_id {
		hyperclient.dest.clone()
	} else if post.source == hyperclient.source.state_machine_id().state_id {
		hyperclient.source.clone()
	} else {
		Err(anyhow!("Unknown client for source: {}", post.source))?
	};
	let dest_client = if post.dest == hyperclient.dest.state_machine_id().state_id {
		hyperclient.dest.clone()
	} else if post.dest == hyperclient.source.state_machine_id().state_id {
		hyperclient.source.clone()
	} else {
		Err(anyhow!("Unknown client for dest: {}", post.dest))?
	};
	let hyperbridge_client = hyperclient.hyperbridge.clone();

	let stream = stream::unfold(state, move |state| {
		let dest_client = dest_client.clone();
		let hyperbridge_client = hyperbridge_client.clone();
		let source_client = source_client.clone();
		let req = Request::Post(post.clone());
		let hash = hash_request::<Keccak256>(&req);
		let post = post.clone();
		async move {
			let lambda = || async {
				match state {
					TimeoutStreamState::Pending => {
						let height = hyperbridge_client
							.query_latest_state_machine_height(dest_client.state_machine_id())
							.await?;

						let state_commitment = hyperbridge_client
							.query_state_machine_commitment(StateMachineHeight {
								id: dest_client.state_machine_id(),
								height,
							})
							.await?;

						if state_commitment.timestamp > post.timeout().as_secs() {
							// early return if the destination has already finalized the height
							return Ok(Some((
								Ok(TimeoutStatus::DestinationFinalized {
									meta: Default::default(),
									finalized_height: height,
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
									if commitment.timestamp > post.timeout().as_secs() {
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
						Ok(valid_proof_height.map(|ev| {
							(
								Ok(TimeoutStatus::DestinationFinalized {
									meta: ev.meta,
									finalized_height: ev.event.latest_height,
								}),
								TimeoutStreamState::DestinationFinalized(ev.event.latest_height),
							)
						}))
					},
					TimeoutStreamState::DestinationFinalized(proof_height) => {
						let relayer = hyperbridge_client.query_request_receipt(hash).await?;
						if relayer == H160::zero() && req.source_chain().is_evm() {
							// request was never delivered
							let latest_height =
								hyperbridge_client.rpc.chain_get_header(None).await?.ok_or_else(
									|| anyhow!("Failed to query latest hyperbridge height!"),
								)?;
							return Ok(Some((
								Ok(TimeoutStatus::HyperbridgeVerified { meta: Default::default() }),
								TimeoutStreamState::HyperbridgeVerified(
									latest_height.number.into(),
								),
							)));
						}

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
							.query_challenge_period(dest_client.state_machine_id())
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
							Ok(TimeoutStatus::HyperbridgeVerified { meta }),
							TimeoutStreamState::HyperbridgeVerified(meta.block_number),
						)))
					},
					TimeoutStreamState::HyperbridgeVerified(hyperbridge_height) => {
						let latest_hyperbridge_height = source_client
							.query_latest_state_machine_height(
								hyperbridge_client.state_machine_id(),
							)
							.await?;
						// check if the height has already been finalized
						let (meta, finalized_height) = if latest_hyperbridge_height >=
							hyperbridge_height
						{
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

							(meta.unwrap_or_default(), latest_hyperbridge_height)
						} else {
							let mut state_machine_update_stream = source_client
								.state_machine_update_notification(
									hyperbridge_client.state_machine_id(),
								)
								.await?;
							loop {
								if let Some(event) = state_machine_update_stream.next().await {
									match event {
										Ok(ev) =>
											if ev.event.latest_height >= hyperbridge_height {
												break (ev.meta, ev.event.latest_height);
											},
										Err(e) =>
											return Ok(Some((
												Err(anyhow!(
													"Encountered error in time out stream {e:?}"
												)),
												state,
											))),
									}
								} else {
									return Ok(Some((
										Err(anyhow!(
											"Encountered error in time out stream: Stream unexpectedly terminated"
										)),
										state,
									)));
								}
							}
						};

						let storage_key = hyperbridge_client.request_receipt_full_key(hash);
						let proof = hyperbridge_client
							.query_state_proof(finalized_height, vec![storage_key])
							.await?;
						let height = StateMachineHeight {
							id: hyperbridge_client.state_machine,
							height: finalized_height,
						};
						let message = Message::Timeout(TimeoutMessage::Post {
							requests: vec![req],
							timeout_proof: Proof { height, proof },
						});
						let challenge_period = source_client
							.query_challenge_period(hyperbridge_client.state_machine_id())
							.await?;
						let update_time =
							source_client.query_state_machine_update_time(height).await?;
						wait_for_challenge_period(&source_client, update_time, challenge_period)
							.await?;
						let calldata = source_client.encode(message)?;

						Ok(Some((
							Ok(TimeoutStatus::HyperbridgeFinalized {
								finalized_height,
								meta,
								calldata: calldata.into(),
							}),
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
