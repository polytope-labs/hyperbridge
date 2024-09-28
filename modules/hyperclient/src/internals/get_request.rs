use crate::{
	internals::encode_response_message_and_wait_for_challenge_period,
	providers::interface::Client,
	types::{BoxStream, MessageStatusStreamState, MessageStatusWithMetadata},
	HyperClient, Keccak256,
};
use anyhow::anyhow;
use futures::{stream, StreamExt};
use ismp::{
	messaging::hash_request,
	router::{GetRequest, Request, Response},
};

/// Queries the status of a Get request
pub async fn query_get_request_status(
	client: &HyperClient,
	get: GetRequest,
) -> Result<MessageStatusWithMetadata, anyhow::Error> {
	let source_client = if get.source == client.source.state_machine_id().state_id {
		&client.source
	} else if get.source == client.dest.state_machine_id().state_id {
		&client.dest
	} else {
		Err(anyhow!("Unknown client for {}", get.source))?
	};

	let commitment = hash_request::<Keccak256>(&Request::Get(get.clone()));

	let source_relayer = source_client.query_response_receipt(commitment).await?;
	if source_relayer != Default::default() {
		// request has been completed
		return Ok(MessageStatusWithMetadata::DestinationDelivered { meta: Default::default() })
	}

	let relayer = client.hyperbridge.query_request_receipt(commitment).await?;
	if relayer != Default::default() {
		// request has been handled by hyperbridge
		return Ok(MessageStatusWithMetadata::HyperbridgeVerified { meta: Default::default() })
	}

	let timestamp = client.hyperbridge.latest_timestamp().await?.as_secs();
	if get.timeout_timestamp > timestamp {
		// request has timed out
		return Ok(MessageStatusWithMetadata::Timeout)
	}

	Ok(MessageStatusWithMetadata::Pending)
}

/// Returns a stream that yeilds whenever the status of a get request changes
pub async fn get_request_status_stream(
	hyperclient: &HyperClient,
	get: GetRequest,
	intial_state: MessageStatusStreamState,
) -> Result<BoxStream<MessageStatusWithMetadata>, anyhow::Error> {
	let source_client = if get.source == hyperclient.dest.state_machine_id().state_id {
		hyperclient.dest.clone()
	} else if get.source == hyperclient.source.state_machine_id().state_id {
		hyperclient.source.clone()
	} else {
		Err(anyhow!("Unknown client for source: {}", get.source))?
	};

	let hyperbridge_client = hyperclient.hyperbridge.clone();
	let commitment = hash_request::<Keccak256>(&Request::Get(get.clone()));

	let stream = stream::unfold(intial_state, move |post_request_status| {
		let hyperbridge_client = hyperbridge_client.clone();
		let source_client = source_client.clone();

		async move {
			let lambda = || async {
				match post_request_status {
					MessageStatusStreamState::Dispatched(tx_height) => {
						// watch for the finalization of the get request
						let mut update_stream = hyperbridge_client
							.state_machine_update_notification(source_client.state_machine_id())
							.await?;

						while let Some(item) = update_stream.next().await {
							match item {
								Ok(state_machine_update) => {
									if state_machine_update.event.latest_height >= tx_height {
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
											MessageStatusStreamState::Dispatched(tx_height),
											e
										)),
										post_request_status,
									))),
							};
						}
						Ok(None)
					},
					MessageStatusStreamState::SourceFinalized(finalized_height) => {
						let mut stream = hyperbridge_client
							.ismp_events_stream(commitment, finalized_height)
							.await?;
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
						let mut stream = source_client
							.state_machine_update_notification(
								hyperbridge_client.state_machine_id(),
							)
							.await?;
						let start = height - 1;
						let response = hyperbridge_client
							.query_ismp_event(start..=height)
							.await?
							.into_iter()
							.find_map(|e| match e.event {
								ismp::events::Event::GetResponse(response) => {
									let event_hash = hash_request::<Keccak256>(&Request::Get(
										response.get.clone(),
									));
									if event_hash == commitment {
										Some(response)
									} else {
										None
									}
								},
								_ => None,
							})
							.expect("Event was emitted in provided height; qed");
						while let Some(update) = stream.next().await {
							match update {
								Ok(event) =>
									if event.event.latest_height >= height {
										let calldata =
											encode_response_message_and_wait_for_challenge_period(
												&hyperbridge_client,
												&source_client,
												Response::Get(response),
												event.event.latest_height,
											)
											.await?;
										return Ok(Some((
											Ok(MessageStatusWithMetadata::HyperbridgeFinalized {
												finalized_height: event.event.latest_height,
												meta: event.meta,
												calldata: calldata.into(),
											}),
											MessageStatusStreamState::End,
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
					_ => {
						// stream has ended
						Ok::<Option<(Result<_, _>, MessageStatusStreamState)>, anyhow::Error>(None)
					},
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
