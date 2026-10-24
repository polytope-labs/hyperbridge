use std::{collections::HashMap, sync::Arc, time::Duration};

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
	// Requests whose proofs failed, to be proved at the next update's newer source height.
	let mut pending: Vec<GetRequest> = Vec::new();

	while let Some((mut get_requests, mut state_machine_update)) = receiver.recv().await {
		// Updates arrive in height order. Drain what is already queued and prove everything at
		// the newest height hyperbridge knows, so a backlog does not prove stale heights.
		while let Ok((more, update)) = receiver.try_recv() {
			get_requests.extend(more);
			state_machine_update = update;
		}

		let mut requests = std::mem::take(&mut pending);
		requests.extend(get_requests);
		if requests.is_empty() {
			continue;
		}

		tracing::info!(target: crate::LOG_TARGET, "Got {} get_requests from {}", requests.len(), state_machine_update.state_machine_id.state_id);

		let hyperbridge_timestamp = match hyperbridge.query_timestamp().await {
			Ok(timestamp) => timestamp,
			Err(err) => {
				tracing::error!(target: crate::LOG_TARGET, "Failed to query timestamp of hyperbridge: {err:?}");
				// The timeout filter could not run, so keep everything for the next update.
				pending = requests;
				continue;
			},
		};

		let (messages, kept) = prove_get_requests(
			&source,
			&client_map,
			requests,
			&state_machine_update,
			hyperbridge_timestamp,
		)
		.await;
		pending = kept;

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

							tracing::trace!(target: crate::LOG_TARGET, "Tracing get response message");
							hyperbridge.dry_run_submission(msg.clone()).await?;

							tracing::info!(target: crate::LOG_TARGET, "Submitting get response",);
							hyperbridge.submit_get_response(msg).await?;

							Ok::<_, anyhow::Error>(())
						};
						match lambda().await {
							Ok(()) => {},
							Err(err) => {
								tracing::error!(target: crate::LOG_TARGET, "Error submitting get response \n{err:?}");
							},
						}
					}
				})
				.await;
		}
	}

	Err(anyhow!("Get Request Stream closed unexpectedly"))
}

/// Build the `GetRequestsWithProof` messages for one update.
///
/// Returns the messages ready to submit and the requests that could not be proved, to be kept
/// for the next update. Timed-out requests are dropped. A request whose destination client is
/// missing is dropped; a proof failure keeps the whole group.
async fn prove_get_requests(
	source: &Arc<dyn IsmpProvider>,
	client_map: &HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	requests: Vec<GetRequest>,
	state_machine_update: &StateMachineUpdated,
	hyperbridge_timestamp: Duration,
) -> (Vec<GetRequestsWithProof>, Vec<GetRequest>) {
	// Group requests by destination chain and height
	let mut groups = HashMap::<_, Vec<GetRequest>>::new();
	requests.into_iter().for_each(|req| {
		// Filter out timed out requests
		let full = Request::Get(req.clone());
		if full.timed_out(hyperbridge_timestamp) {
			tracing::trace!(target: crate::LOG_TARGET, "Skipping timed out get request from {} with nonce {}",req.source, req.nonce);
		} else {
			let key = (req.dest, req.height);
			let entry = groups.entry(key);
			let requests = entry.or_default();
			requests.push(req);
		}
	});

	let mut messages = vec![];
	let mut kept = vec![];

	for ((state_machine, height), requests) in groups {
		if let Some(client) = client_map.get(&state_machine) {
			// let mut requests = vec![];

			// for req in all_requests {
			// 	let full = Request::Get(req.clone());
			// 	let commitment = hash_request::<Hasher>(&full);
			// 	if let Ok(fee) = source.query_request_fee_metadata(commitment).await {
			// 		if fee.is_zero() {
			// 			tracing::trace!(target: crate::LOG_TARGET, "Skipping unprofitable  get
			// request {:?}, fee provided {:?}", commitment, Cost(fee)); 		} else {
			// 			tracing::trace!(target: crate::LOG_TARGET, "Handling profitable  get
			// request {:?}, fee provided {:?}", commitment, Cost(fee));
			// requests.push(req) 		}
			// 	} else {
			// 		tracing::error!(target: crate::LOG_TARGET, "Failed to query fee for get
			// request {:?}", commitment); 		continue
			// 	}
			// }

			if requests.is_empty() {
				continue;
			}

			let request_commitment_keys = requests.iter().map(|req| {
				let full = Request::Get(req.clone());
				let commitment = hash_request::<Hasher>(&full);
				source.request_commitment_full_key(commitment)
			});

			let query = StateProofQueryType::Ismp(request_commitment_keys.flatten().collect());

			tracing::trace!(target: crate::LOG_TARGET, "Fetching source proofs for {} get_requests from {}", requests.len(), state_machine_update.state_machine_id.state_id);
			let source_proof = match source
				.query_state_proof(state_machine_update.latest_height, query)
				.await
			{
				Ok(proof) => Proof {
					height: StateMachineHeight {
						id: state_machine_update.state_machine_id,
						height: state_machine_update.latest_height,
					},
					proof,
				},
				Err(err) => {
					tracing::error!(target: crate::LOG_TARGET, ?err, "Failed to fetch proofs for get requests, kept {} get requests to prove at the next update", requests.len());
					kept.extend(requests);
					continue;
				},
			};

			let keys =
				requests.iter().map(|req| req.keys.clone()).flatten().collect::<Vec<_>>();
			tracing::trace!(target: crate::LOG_TARGET, "Fetching state proofs for {} keys from {state_machine}", keys.len());
			let storage_proof = match client
				.query_state_proof(height, StateProofQueryType::Arbitrary(keys))
				.await
			{
				Ok(proof) => Proof {
					height: StateMachineHeight { id: client.state_machine_id(), height },
					proof,
				},
				Err(err) => {
					tracing::error!(target: crate::LOG_TARGET, ?err, "Failed to fetch get response proof, kept {} get requests to prove at the next update", requests.len());
					kept.extend(requests);
					continue;
				},
			};

			tracing::trace!(target: crate::LOG_TARGET, "Handling {} get_requests for the chain pair {}:{state_machine}", requests.len(), state_machine_update.state_machine_id.state_id);
			let msg = GetRequestsWithProof {
				requests,
				source: source_proof,
				response: storage_proof,
				address: source.address(),
			};

			messages.push(msg)
		} else {
			tracing::debug!(
				target: crate::LOG_TARGET, "Skipping get requests because client for {} was not found",
				state_machine
			);
		}
	}

	(messages, kept)
}

#[cfg(test)]
mod tests {
	use super::*;
	use ismp::consensus::StateMachineId;
	use tesseract_primitives::mocks::MockHost;

	const SOURCE: StateMachine = StateMachine::Evm(97);
	const DEST: StateMachine = StateMachine::Evm(80002);

	fn get() -> GetRequest {
		GetRequest {
			source: SOURCE,
			dest: DEST,
			nonce: 2,
			from: vec![1],
			keys: vec![vec![3]],
			height: 10,
			context: vec![],
			timeout_timestamp: 0,
		}
	}

	fn update(height: u64) -> StateMachineUpdated {
		StateMachineUpdated {
			state_machine_id: StateMachineId {
				state_id: SOURCE,
				consensus_state_id: *b"Mock",
			},
			latest_height: height,
		}
	}

	fn clients(dest: MockHost<()>) -> HashMap<StateMachine, Arc<dyn IsmpProvider>> {
		let mut clients = HashMap::new();
		clients.insert(DEST, Arc::new(dest) as Arc<dyn IsmpProvider>);
		clients
	}

	/// A source proof failure keeps the request; the next update proves it at its own, newer
	/// source height even though it carries no new GET requests.
	#[tokio::test]
	async fn a_source_proof_failure_keeps_the_request_for_the_next_update() {
		let source = MockHost::new((), 0, SOURCE).with_state_proof_fail();
		let proof_ok = source.state_proof_ok.clone();
		let source: Arc<dyn IsmpProvider> = Arc::new(source);
		let clients = clients(MockHost::new((), 0, DEST));

		let (messages, kept) = prove_get_requests(
			&source,
			&clients,
			vec![get()],
			&update(200),
			Duration::ZERO,
		)
		.await;
		assert!(messages.is_empty());
		assert_eq!(kept, vec![get()]);

		*proof_ok.lock().unwrap() = true;
		let (messages, kept) =
			prove_get_requests(&source, &clients, kept, &update(300), Duration::ZERO).await;
		assert!(kept.is_empty());
		assert_eq!(messages.len(), 1);
		assert_eq!(messages[0].requests, vec![get()]);
		assert_eq!(messages[0].source.height.height, 300);
		assert_eq!(messages[0].source.height.id.state_id, SOURCE);
	}

	/// A destination storage proof failure keeps the request. The next update retries the same
	/// destination height, which cannot change.
	#[tokio::test]
	async fn a_storage_proof_failure_keeps_the_request_for_the_next_update() {
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let dest = MockHost::new((), 0, DEST).with_state_proof_fail();
		let proof_ok = dest.state_proof_ok.clone();
		let clients = clients(dest);

		let (messages, kept) = prove_get_requests(
			&source,
			&clients,
			vec![get()],
			&update(200),
			Duration::ZERO,
		)
		.await;
		assert!(messages.is_empty());
		assert_eq!(kept, vec![get()]);

		*proof_ok.lock().unwrap() = true;
		let (messages, kept) =
			prove_get_requests(&source, &clients, kept, &update(300), Duration::ZERO).await;
		assert!(kept.is_empty());
		assert_eq!(messages.len(), 1);
		assert_eq!(messages[0].requests, vec![get()]);
		assert_eq!(messages[0].response.height.height, get().height);
		assert_eq!(messages[0].response.height.id.state_id, DEST);
	}

	/// A parked request that times out is dropped instead of being kept forever.
	#[tokio::test]
	async fn a_parked_request_that_times_out_is_dropped() {
		let source = MockHost::new((), 0, SOURCE).with_state_proof_fail();
		let proof_ok = source.state_proof_ok.clone();
		let source: Arc<dyn IsmpProvider> = Arc::new(source);
		let clients = clients(MockHost::new((), 0, DEST));

		let mut request = get();
		request.timeout_timestamp = 100;

		let (messages, kept) = prove_get_requests(
			&source,
			&clients,
			vec![request.clone()],
			&update(200),
			Duration::from_secs(50),
		)
		.await;
		assert!(messages.is_empty());
		assert_eq!(kept, vec![request.clone()]);

		// The source has recovered, but the request's timeout has passed.
		*proof_ok.lock().unwrap() = true;
		let (messages, kept) = prove_get_requests(
			&source,
			&clients,
			kept,
			&update(300),
			Duration::from_secs(101),
		)
		.await;
		assert!(messages.is_empty());
		assert!(kept.is_empty());
	}
}
