use std::{collections::HashMap, sync::Arc};

use anyhow::anyhow;
use futures::StreamExt;
use ismp::{
	consensus::StateMachineHeight,
	host::StateMachine,
	messaging::{hash_request, Proof},
	router::{GetRequest, Request},
};
use pallet_state_coprocessor::impls::GetRequestsWithProof;
use tesseract_primitives::{
	observe_challenge_period, HandleGetResponse, Hasher, IsmpProvider, StateMachineUpdated,
	StateProofQueryType,
};
use tokio::sync::mpsc::Receiver;

pub async fn process_get_request_events<
	A: IsmpProvider + HandleGetResponse + Clone + Clone + 'static,
>(
	mut receiver: Receiver<(Vec<GetRequest>, StateMachineUpdated)>,
	source: Arc<dyn IsmpProvider>,
	hyperbridge: A,
	client_map: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
) -> Result<(), anyhow::Error> {
	while let Some((get_requests, state_machine_update)) = receiver.recv().await {
		if get_requests.is_empty() {
			continue;
		}

		// Group requests by destination chain and height
		// Fetch source chain proofs
		// Fetch destination chain storage proofs for all keys

		let mut groups = HashMap::<_, Vec<GetRequest>>::new();
		let hyperbridge_timestamp = match hyperbridge.query_timestamp().await {
			Ok(timestamp) => timestamp,
			Err(err) => {
				tracing::error!("Failed to query timestamp of hyperbridge: {err:?}");
				continue;
			},
		};

		get_requests.into_iter().for_each(|req| {
			// Filter out timed out requests
			if req.timeout() <= hyperbridge_timestamp {
				let key = (req.dest, req.height);
				let entry = groups.entry(key);
				let requests = entry.or_default();
				requests.push(req);
			}
		});

		let mut messages = vec![];

		let group_keys = groups.keys().cloned().collect::<Vec<_>>();

		for (state_machine, height) in group_keys {
			if let Some(client) = client_map.get(&state_machine) {
				let requests = groups.remove(&(state_machine, height)).unwrap_or_default();

				let request_commitment_keys = requests
					.iter()
					.map(|req| {
						let full = Request::Get(req.clone());
						let commitment = hash_request::<Hasher>(&full);
						source
							.request_commitment_full_key(commitment)
							.into_iter()
							.flatten()
							.collect()
					})
					.collect::<Vec<_>>();

				let query = StateProofQueryType::Ismp(request_commitment_keys);

				let source_proof =
					match source.query_state_proof(state_machine_update.latest_height, query).await
					{
						Ok(proof) => Proof {
							height: StateMachineHeight {
								id: state_machine_update.state_machine_id,
								height: state_machine_update.latest_height,
							},
							proof,
						},
						Err(err) => {
							tracing::error!("Failed to fetch proofs for get requests: {err:?}");
							continue;
						},
					};

				let keys = requests.iter().map(|req| req.keys.clone()).flatten().collect();
				let storage_proof = match client
					.query_state_proof(height, StateProofQueryType::Arbitrary(keys))
					.await
				{
					Ok(proof) => Proof {
						height: StateMachineHeight { id: client.state_machine_id(), height },
						proof,
					},
					Err(err) => {
						tracing::error!("Failed to fetch get response proof: {err:?}");
						continue;
					},
				};

				tracing::info!(target: "tesseract", "Handling {} get_requests {}-{state_machine}", requests.len(), state_machine_update.state_machine_id.state_id);
				let msg = GetRequestsWithProof {
					requests,
					source: source_proof,
					response: storage_proof,
					address: source.address(),
				};

				messages.push(msg)
			} else {
				tracing::debug!(
					"Skipping get requests because client for {} was not found",
					state_machine
				);
			}
		}

		if !messages.is_empty() {
			let stream = futures::stream::iter(messages);
			stream
				.for_each_concurrent(None, |msg| {
					let hyperbridge = Arc::new(hyperbridge.clone());
					let source = source.clone();
					let dest = client_map
						.get(&msg.response.height.id.state_id)
						.cloned()
						.expect("Client exists, we have a proof");
					async move {
						let lambda = || async {
							// Wait for challenge period for source and dest state machine heights
							// to elapse on hyperbridge
							observe_challenge_period(
								source.clone(),
								hyperbridge.clone(),
								msg.source.height.height,
							)
							.await?;
							observe_challenge_period(
								dest.clone(),
								hyperbridge.clone(),
								msg.response.height.height,
							)
							.await?;

							// Submit messages to Hyperbridge

							hyperbridge.submit_get_response(msg).await?;

							Ok::<_, anyhow::Error>(())
						};
						match lambda().await {
							Ok(()) => {},
							Err(err) => {
								tracing::error!("Error submitting get response \n{err:?}");
							},
						}
					}
				})
				.await;
		}
	}

	Err(anyhow!("Get Request Stream closed unexpectedly"))
}
