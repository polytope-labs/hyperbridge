use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::anyhow;
use futures::StreamExt;
use ismp::{
	consensus::StateMachineHeight,
	host::StateMachine,
	messaging::{hash_request, Message, Proof, ResponseMessage},
	router::{GetRequest, Request},
};
use pallet_state_coprocessor::impls::GetRequestsWithProof;
use tesseract_primitives::{
	observe_challenge_period, HandleGetResponse, Hasher, IsmpProvider, StateMachineUpdated,
	StateProofQueryType,
};
use tokio::sync::mpsc::Receiver;
use transaction_fees::{unix_now, ParkedMessage, RetryState, TransactionPayment};

pub async fn process_get_request_events<
	A: IsmpProvider + HandleGetResponse + Clone + Clone + 'static,
>(
	mut receiver: Receiver<(Vec<GetRequest>, StateMachineUpdated)>,
	source: Arc<dyn IsmpProvider>,
	hyperbridge: A,
	client_map: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	tx_payment: Arc<TransactionPayment>,
) -> Result<(), anyhow::Error> {
	let sink = hyperbridge.state_machine_id().state_id;
	let source_state = source.state_machine_id().state_id;

	while let Some((mut get_requests, mut state_machine_update)) = receiver.recv().await {
		// Updates arrive in height order. Drain what is already queued and prove everything at
		// the newest height hyperbridge knows, so a backlog does not prove stale heights.
		while let Ok((more, update)) = receiver.try_recv() {
			get_requests.extend(more);
			state_machine_update = update;
		}

		let rows = match tx_payment.unprofitable_messages(&sink).await {
			Ok(rows) => rows,
			Err(err) => {
				tracing::error!(target: crate::LOG_TARGET, ?err, "Failed to read parked get requests");
				Vec::new()
			},
		};
		// Only this source chain's response rows are ours. Other rows belong to POST parking or
		// to another source chain's processor and are left untouched.
		let own_rows = rows
			.into_iter()
			.filter(|ParkedMessage { message, .. }| match message {
				Message::Response(msg) =>
					!msg.requests.is_empty() &&
						msg.requests.iter().all(|req| req.source == source_state),
				_ => false,
			})
			.collect::<Vec<_>>();

		if get_requests.is_empty() && own_rows.is_empty() {
			continue;
		}

		let hyperbridge_timestamp = match hyperbridge.query_timestamp().await {
			Ok(timestamp) => timestamp,
			Err(err) => {
				tracing::error!(target: crate::LOG_TARGET, "Failed to query timestamp of hyperbridge: {err:?}");
				// The timeout filter cannot run, but the destination proof does not depend on
				// the timestamp: park the fresh requests with it and leave existing rows alone.
				park_get_requests(
					&tx_payment,
					&source,
					&client_map,
					get_requests,
					sink,
				)
				.await;
				continue;
			},
		};

		let ProvedGetRequests { messages, store, mut delete } = prove_get_requests(
			&source,
			&client_map,
			get_requests,
			own_rows,
			&state_machine_update,
			hyperbridge_timestamp,
		)
		.await;

		// Store replacements before deleting their old rows, so a failed write cannot lose a
		// request. A row only reaches `delete` once its replacement was stored.
		let replaced = store.iter().filter_map(|parked| parked.id).collect::<Vec<_>>();
		let new_requests = store
			.iter()
			.filter(|parked| parked.id.is_none())
			.map(|parked| match &parked.message {
				Message::Response(msg) => msg.requests.len(),
				_ => 0,
			})
			.sum::<usize>();
		let kept_rows = replaced.len();
		if !store.is_empty() {
			match tx_payment.store_unprofitable_messages(store, sink).await {
				Ok(_) => delete.extend(replaced),
				Err(err) => {
					tracing::error!(
						target: crate::LOG_TARGET,
						?err,
						lost_requests = new_requests,
						kept_rows,
						"Failed to store get requests: {} new get requests lost, {} replaced rows kept",
						new_requests,
						kept_rows,
					);
				},
			}
		}
		if !delete.is_empty() {
			if let Err(err) = tx_payment.delete_unprofitable_messages(delete).await {
				tracing::error!(target: crate::LOG_TARGET, ?err, "Failed to delete proved get-request rows");
			}
		}

		if !messages.is_empty() {
			// Submit in the background so a slow challenge period or submission never blocks
			// reading the next update from the channel.
			let hyperbridge = hyperbridge.clone();
			let source = source.clone();
			let client_map = client_map.clone();
			tokio::spawn(async move {
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
								// Wait for challenge period for source and dest state machine
								// heights to elapse on hyperbridge
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
			});
		}
	}

	Err(anyhow!("Get Request Stream closed unexpectedly"))
}

/// The outcome of proving one update's GET requests.
struct ProvedGetRequests {
	/// Messages ready to submit to hyperbridge.
	messages: Vec<GetRequestsWithProof>,
	/// Rows to insert. A `Some` id means the row replaces that old row.
	store: Vec<ParkedMessage>,
	/// Rows that are no longer needed and have no replacement.
	delete: Vec<i32>,
}

/// One proving group: requests that share a destination and height.
struct GetRequestGroup {
	dest: StateMachine,
	height: u64,
	requests: Vec<GetRequest>,
	row_id: Option<i32>,
	// The stored destination proof, if this group came from a row.
	stored_proof: Option<Proof>,
}

/// Build the `GetRequestsWithProof` messages for one update and decide which rows to store or
/// delete.
///
/// Stored rows are proved again from their own destination proof, so only the source proof is
/// fetched at the update's newer height. A failed destination proof drops the group; a failed
/// source proof stores the group with its destination proof for a later update.
async fn prove_get_requests(
	source: &Arc<dyn IsmpProvider>,
	client_map: &HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	requests: Vec<GetRequest>,
	rows: Vec<ParkedMessage>,
	state_machine_update: &StateMachineUpdated,
	hyperbridge_timestamp: Duration,
) -> ProvedGetRequests {
	let mut groups = Vec::new();
	let mut fresh_groups = HashMap::<(StateMachine, u64), Vec<GetRequest>>::new();
	requests.into_iter().for_each(|req| {
		fresh_groups.entry((req.dest, req.height)).or_default().push(req);
	});
	groups.extend(fresh_groups.into_iter().map(|((dest, height), requests)| GetRequestGroup {
		dest,
		height,
		requests,
		row_id: None,
		stored_proof: None,
	}));
	for ParkedMessage { message, id, .. } in rows {
		let Message::Response(msg) = message else { continue };
		let Some(first) = msg.requests.first() else { continue };
		groups.push(GetRequestGroup {
			dest: first.dest,
			height: first.height,
			requests: msg.requests,
			row_id: id,
			stored_proof: Some(msg.proof),
		});
	}

	let mut result = ProvedGetRequests { messages: vec![], store: vec![], delete: vec![] };

	for mut group in groups {
		// Filter out timed out requests
		let original_len = group.requests.len();
		group
			.requests
			.retain(|req| !Request::Get(req.clone()).timed_out(hyperbridge_timestamp));
		let timed_out = group.requests.len() != original_len;
		if group.requests.is_empty() {
			if let Some(id) = group.row_id {
				result.delete.push(id);
			}
			continue;
		}

		if let Some(client) = client_map.get(&group.dest) {
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

			// Fetch the destination proof first, so a failure there never costs a source proof.
			let destination_proof = match group.stored_proof.take() {
				Some(proof) => proof,
				None => match fetch_destination_proof(client, group.height, &group.requests).await
				{
					Ok(proof) => proof,
					Err(err) => {
						tracing::error!(target: crate::LOG_TARGET, ?err, "Failed to fetch the destination proof for {} get requests; they were dropped", group.requests.len());
						if let Some(id) = group.row_id {
							result.delete.push(id);
						}
						continue;
					},
				},
			};

			let request_commitment_keys = group.requests.iter().map(|req| {
				let full = Request::Get(req.clone());
				let commitment = hash_request::<Hasher>(&full);
				source.request_commitment_full_key(commitment)
			});

			let query = StateProofQueryType::Ismp(request_commitment_keys.flatten().collect());

			tracing::trace!(target: crate::LOG_TARGET, "Fetching source proofs for {} get_requests from {}", group.requests.len(), state_machine_update.state_machine_id.state_id);
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
					tracing::error!(target: crate::LOG_TARGET, ?err, "Failed to fetch source proofs for {} get requests, kept them to prove at a newer height", group.requests.len());
					// Keep the row untouched when its requests did not change.
					if group.row_id.is_none() || timed_out {
						result.store.push(ParkedMessage {
							message: Message::Response(ResponseMessage {
								requests: group.requests,
								proof: destination_proof,
								signer: source.address(),
							}),
							state: RetryState { created_at: unix_now(), ..Default::default() },
							id: group.row_id,
						});
					}
					continue;
				},
			};

			tracing::trace!(target: crate::LOG_TARGET, "Handling {} get_requests for the chain pair {}:{}", group.requests.len(), state_machine_update.state_machine_id.state_id, group.dest);
			result.messages.push(GetRequestsWithProof {
				requests: group.requests,
				source: source_proof,
				response: destination_proof,
				address: source.address(),
			});
			if let Some(id) = group.row_id {
				result.delete.push(id);
			}
		} else {
			if group.row_id.is_some() {
				tracing::warn!(
					target: crate::LOG_TARGET, "Dropping a stored get request because client for {} was not found",
					group.dest
				);
			} else {
				tracing::debug!(
					target: crate::LOG_TARGET, "Skipping get requests because client for {} was not found",
					group.dest
				);
			}
			if let Some(id) = group.row_id {
				result.delete.push(id);
			}
		}
	}

	result
}

/// Store fresh GET requests when the hyperbridge timestamp cannot be queried.
///
/// The timeout filter is skipped, but the destination proof does not depend on the timestamp, so
/// each group is stored with it. A group whose destination proof cannot be fetched is dropped.
async fn park_get_requests(
	tx_payment: &TransactionPayment,
	source: &Arc<dyn IsmpProvider>,
	client_map: &HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	requests: Vec<GetRequest>,
	sink: StateMachine,
) {
	if requests.is_empty() {
		return
	}

	let mut groups = HashMap::<_, Vec<GetRequest>>::new();
	for req in requests {
		groups.entry((req.dest, req.height)).or_default().push(req);
	}

	let mut parked = Vec::new();
	for ((dest, height), requests) in groups {
		let Some(client) = client_map.get(&dest) else {
			tracing::debug!(
				target: crate::LOG_TARGET, "Skipping get requests because client for {dest} was not found"
			);
			continue;
		};

		match fetch_destination_proof(client, height, &requests).await {
			Ok(proof) => parked.push(ParkedMessage {
				message: Message::Response(ResponseMessage {
					requests,
					proof,
					signer: source.address(),
				}),
				state: RetryState { created_at: unix_now(), ..Default::default() },
				id: None,
			}),
			Err(err) => {
				tracing::error!(target: crate::LOG_TARGET, ?err, "Failed to fetch the destination proof for {} get requests; they were dropped while the hyperbridge timestamp is unavailable", requests.len());
			},
		}
	}

	if parked.is_empty() {
		return
	}
	if let Err(err) = tx_payment.store_unprofitable_messages(parked, sink).await {
		tracing::error!(target: crate::LOG_TARGET, ?err, "Failed to store get requests after a timestamp failure");
	}
}

/// Fetch the destination storage proof for `requests` at `height`.
async fn fetch_destination_proof(
	client: &Arc<dyn IsmpProvider>,
	height: u64,
	requests: &[GetRequest],
) -> Result<Proof, anyhow::Error> {
	let keys = requests.iter().flat_map(|req| req.keys.clone()).collect::<Vec<_>>();
	tracing::trace!(target: crate::LOG_TARGET, "Fetching state proofs for {} keys from {}", keys.len(), client.state_machine_id().state_id);
	let proof = client
		.query_state_proof(height, StateProofQueryType::Arbitrary(keys))
		.await?;
	Ok(Proof {
		height: StateMachineHeight { id: client.state_machine_id(), height },
		proof,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use ismp::{consensus::StateMachineId, messaging::RequestMessage, router::PostRequest};
	use tesseract_primitives::mocks::MockHost;

	const SOURCE: StateMachine = StateMachine::Evm(97);
	const DEST: StateMachine = StateMachine::Evm(80002);
	const COPROCESSOR: StateMachine = StateMachine::Kusama(4009);
	const STORED_PROOF: [u8; 3] = [7, 8, 9];

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

	fn post() -> PostRequest {
		PostRequest {
			source: SOURCE,
			dest: DEST,
			nonce: 1,
			from: vec![1],
			to: vec![2],
			timeout_timestamp: 0,
			body: vec![],
		}
	}

	fn dest_clients(dest: MockHost<()>) -> HashMap<StateMachine, Arc<dyn IsmpProvider>> {
		let mut clients = HashMap::new();
		clients.insert(DEST, Arc::new(dest) as Arc<dyn IsmpProvider>);
		clients
	}

	fn destination_proof(proof: Vec<u8>) -> Proof {
		Proof {
			height: StateMachineHeight {
				id: StateMachineId { state_id: DEST, consensus_state_id: *b"Mock" },
				height: get().height,
			},
			proof,
		}
	}

	fn stored_row(
		requests: Vec<GetRequest>,
		proof: Proof,
		id: Option<i32>,
	) -> ParkedMessage {
		ParkedMessage {
			message: Message::Response(ResponseMessage { requests, proof, signer: Vec::new() }),
			state: RetryState::default(),
			id,
		}
	}

	/// Build a unique on-disk path for a test database.
	fn temp_db_path() -> String {
		let pid = std::process::id();
		let nanos = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|d| d.as_nanos())
			.unwrap_or(0);
		format!("/tmp/tesseract-get-requests-test-{pid}-{nanos}.db")
	}

	fn cleanup(path: &str) {
		let _ = std::fs::remove_file(path);
		let _ = std::fs::remove_file(format!("{path}-journal"));
	}

	/// Wait under paused time until the submission task has recorded `count` messages.
	async fn wait_for_responses(
		responses: &Arc<std::sync::Mutex<Vec<GetRequestsWithProof>>>,
		count: usize,
	) {
		for _ in 0..100 {
			if responses.lock().unwrap().len() >= count {
				return
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
		panic!("expected {count} get responses, got {}", responses.lock().unwrap().len());
	}

	/// A source proof failure stores the group with the destination proof, at the destination's
	/// own height. A later attempt with the destination down reuses that proof.
	#[tokio::test]
	async fn a_source_proof_failure_stores_the_destination_proof() {
		let source = MockHost::new((), 0, SOURCE).with_state_proof_fail();
		let proof_ok = source.state_proof_ok.clone();
		let source: Arc<dyn IsmpProvider> = std::sync::Arc::new(source);
		let destination_up =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));

		let ProvedGetRequests { messages, store, delete } = prove_get_requests(
			&source,
			&destination_up,
			vec![get()],
			vec![],
			&update(200),
			Duration::ZERO,
		)
		.await;
		assert!(messages.is_empty());
		assert!(delete.is_empty());
		assert_eq!(store.len(), 1);

		let ParkedMessage { message: Message::Response(row), id, .. } = &store[0] else {
			panic!("expected a response row, got {:?}", store[0].message);
		};
		assert!(id.is_none());
		assert_eq!(row.requests, vec![get()]);
		assert_eq!(row.proof.height.id.state_id, DEST);
		assert_eq!(row.proof.height.height, get().height);
		assert_eq!(row.proof.proof, STORED_PROOF.to_vec());

		// The source recovers while the destination is down: the stored proof is reused.
		*proof_ok.lock().unwrap() = true;
		let destination_down = dest_clients(MockHost::new((), 0, DEST).with_state_proof_fail());
		let rows =
			vec![stored_row(row.requests.clone(), row.proof.clone(), Some(3))];
		let result = prove_get_requests(
			&source,
			&destination_down,
			vec![],
			rows,
			&update(300),
			Duration::ZERO,
		)
		.await;
		assert!(result.store.is_empty());
		assert_eq!(result.delete, vec![3]);
		assert_eq!(result.messages.len(), 1);
		assert_eq!(result.messages[0].requests, vec![get()]);
		assert_eq!(result.messages[0].source.height.height, 300);
		assert_eq!(result.messages[0].response, row.proof);
	}

	/// A destination proof failure drops the request instead of parking it.
	#[tokio::test]
	async fn a_destination_proof_failure_drops_the_request() {
		let source: Arc<dyn IsmpProvider> = std::sync::Arc::new(MockHost::new((), 0, SOURCE));
		let clients = dest_clients(MockHost::new((), 0, DEST).with_state_proof_fail());

		let result = prove_get_requests(
			&source,
			&clients,
			vec![get()],
			vec![],
			&update(200),
			Duration::ZERO,
		)
		.await;
		assert!(result.messages.is_empty());
		assert!(result.store.is_empty());
		assert!(result.delete.is_empty());
	}

	/// A stored row whose requests all timed out is deleted.
	#[tokio::test]
	async fn a_stored_request_that_times_out_is_deleted() {
		let source: Arc<dyn IsmpProvider> = std::sync::Arc::new(MockHost::new((), 0, SOURCE));
		let clients = dest_clients(MockHost::new((), 0, DEST));
		let mut request = get();
		request.timeout_timestamp = 100;
		let row = stored_row(vec![request], destination_proof(STORED_PROOF.to_vec()), Some(4));

		let result = prove_get_requests(
			&source,
			&clients,
			vec![],
			vec![row],
			&update(300),
			Duration::from_secs(101),
		)
		.await;
		assert!(result.messages.is_empty());
		assert!(result.store.is_empty());
		assert_eq!(result.delete, vec![4]);
	}

	/// A row whose destination proof and requests did not change is left untouched when only the
	/// source proof fails.
	#[tokio::test]
	async fn an_unchanged_row_is_left_alone_on_a_source_proof_failure() {
		let source = MockHost::new((), 0, SOURCE).with_state_proof_fail();
		let source: Arc<dyn IsmpProvider> = std::sync::Arc::new(source);
		let clients = dest_clients(MockHost::new((), 0, DEST));
		let row = stored_row(vec![get()], destination_proof(STORED_PROOF.to_vec()), Some(5));

		let result = prove_get_requests(
			&source,
			&clients,
			vec![],
			vec![row],
			&update(300),
			Duration::ZERO,
		)
		.await;
		assert!(result.messages.is_empty());
		assert!(result.store.is_empty());
		assert!(result.delete.is_empty());
	}

	/// A source proof failure stores the destination proof; a new processor on the same database
	/// proves the request later even with the destination down, then deletes the row.
	#[tokio::test(start_paused = true)]
	async fn a_new_processor_reuses_the_stored_destination_proof() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());

		// First processor: the source proof fails while the destination proof works.
		let source = Arc::new(MockHost::new((), 0, SOURCE).with_state_proof_fail())
			as Arc<dyn IsmpProvider>;
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let clients =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));
		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			clients,
			tx_payment.clone(),
		));
		sender.send((vec![get()], update(200))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());

		let rows = tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap();
		assert_eq!(rows.len(), 1);
		let stored_proof = match &rows[0].message {
			Message::Response(row) => {
				assert_eq!(row.requests, vec![get()]);
				assert_eq!(row.proof.height.id.state_id, DEST);
				assert_eq!(row.proof.height.height, get().height);
				assert_eq!(row.proof.proof, STORED_PROOF.to_vec());
				row.proof.clone()
			},
			other => panic!("expected a response row, got {other:?}"),
		};

		// A new processor on the same database: the source works, the destination is down.
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let responses = hyperbridge.get_responses.clone();
		let destination_down = dest_clients(MockHost::new((), 0, DEST).with_state_proof_fail());
		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			destination_down,
			tx_payment.clone(),
		));
		sender.send((vec![], update(300))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());

		wait_for_responses(&responses, 1).await;
		let responses = responses.lock().unwrap();
		assert_eq!(responses.len(), 1);
		assert_eq!(responses[0].requests, vec![get()]);
		assert_eq!(responses[0].source.height.height, 300);
		assert_eq!(responses[0].response, stored_proof);
		drop(responses);

		assert!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().is_empty());
		cleanup(&path);
	}

	/// A response row for another source chain is neither proved nor deleted.
	#[tokio::test(start_paused = true)]
	async fn a_row_from_another_source_chain_is_left_alone() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let foreign = GetRequest { source: StateMachine::Evm(8453), ..get() };
		tx_payment
			.store_unprofitable_messages(
				vec![stored_row(
					vec![foreign.clone()],
					destination_proof(STORED_PROOF.to_vec()),
					None,
				)],
				COPROCESSOR,
			)
			.await
			.unwrap();

		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let responses = hyperbridge.get_responses.clone();
		let clients = dest_clients(MockHost::new((), 0, DEST));
		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			clients,
			tx_payment.clone(),
		));
		sender.send((vec![], update(200))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());

		assert!(responses.lock().unwrap().is_empty());
		let rows = tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap();
		assert_eq!(rows.len(), 1);
		match &rows[0].message {
			Message::Response(row) => assert_eq!(row.requests, vec![foreign]),
			other => panic!("expected the foreign row to stay, got {other:?}"),
		}
		cleanup(&path);
	}

	/// POST-parking rows share the hyperbridge key. The GET processor must not prove, delete or
	/// rewrite them.
	#[tokio::test(start_paused = true)]
	async fn post_rows_under_the_hyperbridge_key_are_left_alone() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let post = post();
		tx_payment
			.store_unprofitable_messages(
				vec![ParkedMessage {
					message: Message::Request(RequestMessage {
						requests: vec![post.clone()],
						proof: Proof {
							height: StateMachineHeight {
								id: StateMachineId {
									state_id: SOURCE,
									consensus_state_id: *b"Mock",
								},
								height: 10,
							},
							proof: vec![1],
						},
						signer: Vec::new(),
					}),
					state: RetryState::default(),
					id: None,
				}],
				COPROCESSOR,
			)
			.await
			.unwrap();

		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let responses = hyperbridge.get_responses.clone();
		let clients =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));
		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			clients,
			tx_payment.clone(),
		));
		sender.send((vec![get()], update(200))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());
		wait_for_responses(&responses, 1).await;

		let rows = tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap();
		assert_eq!(rows.len(), 1);
		match &rows[0].message {
			Message::Request(row) => assert_eq!(row.requests, vec![post]),
			other => panic!("expected the post row to stay, got {other:?}"),
		}
		cleanup(&path);
	}

	/// A stored row with one timed-out request is replaced by a row holding only the remaining
	/// request and the same destination proof, and the old row is deleted.
	#[tokio::test(start_paused = true)]
	async fn a_partially_timed_out_row_is_replaced() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let mut expired = get();
		expired.nonce = 3;
		expired.timeout_timestamp = 100;
		let alive = get();
		let stored_proof = destination_proof(STORED_PROOF.to_vec());
		let ids = tx_payment
			.store_unprofitable_messages(
				vec![stored_row(
					vec![expired, alive.clone()],
					stored_proof.clone(),
					None,
				)],
				COPROCESSOR,
			)
			.await
			.unwrap();
		let old_id = ids[0];

		let source = Arc::new(MockHost::new((), 0, SOURCE).with_state_proof_fail())
			as Arc<dyn IsmpProvider>;
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(200));
		let responses = hyperbridge.get_responses.clone();
		let clients = dest_clients(MockHost::new((), 0, DEST));
		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			clients,
			tx_payment.clone(),
		));
		sender.send((vec![], update(300))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());

		assert!(responses.lock().unwrap().is_empty());
		let rows = tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap();
		assert_eq!(rows.len(), 1);
		let row = rows.into_iter().next().unwrap();
		assert_ne!(row.id, Some(old_id));
		match row.message {
			Message::Response(msg) => {
				assert_eq!(msg.requests, vec![alive]);
				assert_eq!(msg.proof, stored_proof);
			},
			other => panic!("expected a replacement response row, got {other:?}"),
		}
		cleanup(&path);
	}

	/// A stored request that has timed out is deleted and not submitted.
	#[tokio::test(start_paused = true)]
	async fn a_stored_request_that_times_out_is_deleted_from_the_database() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let mut request = get();
		request.timeout_timestamp = 100;
		tx_payment
			.store_unprofitable_messages(
				vec![stored_row(
					vec![request],
					destination_proof(STORED_PROOF.to_vec()),
					None,
				)],
				COPROCESSOR,
			)
			.await
			.unwrap();

		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(200));
		let responses = hyperbridge.get_responses.clone();
		let clients = dest_clients(MockHost::new((), 0, DEST));
		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			clients,
			tx_payment.clone(),
		));
		sender.send((vec![], update(300))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());

		assert!(responses.lock().unwrap().is_empty());
		assert!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().is_empty());
		cleanup(&path);
	}

	/// A timestamp failure still stores the destination proof; the next update proves the request
	/// from that stored proof even with the destination RPC down.
	#[tokio::test(start_paused = true)]
	async fn a_timestamp_failure_stores_the_destination_proof() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());

		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge = MockHost::new((), 0, COPROCESSOR)
			.with_timestamp(Duration::from_secs(100))
			.with_timestamp_fail();
		let timestamp_ok = hyperbridge.timestamp_ok.clone();
		let responses = hyperbridge.get_responses.clone();
		let dest = MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec());
		let dest_proof_ok = dest.state_proof_ok.clone();
		let clients = dest_clients(dest);
		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			clients,
			tx_payment.clone(),
		));
		sender.send((vec![get()], update(200))).await.unwrap();
		// Let the processor hit the timestamp failure and park the request.
		tokio::time::sleep(Duration::from_secs(1)).await;

		let rows = tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap();
		assert_eq!(rows.len(), 1);
		let stored_proof = match &rows[0].message {
			Message::Response(row) => {
				assert_eq!(row.requests, vec![get()]);
				assert_eq!(row.proof.height.id.state_id, DEST);
				assert_eq!(row.proof.height.height, get().height);
				assert_eq!(row.proof.proof, STORED_PROOF.to_vec());
				row.proof.clone()
			},
			other => panic!("expected a response row, got {other:?}"),
		};

		// The destination RPC is down now, but the stored proof is reused.
		*dest_proof_ok.lock().unwrap() = false;
		*timestamp_ok.lock().unwrap() = true;
		sender.send((vec![], update(300))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());

		wait_for_responses(&responses, 1).await;
		let responses = responses.lock().unwrap();
		assert_eq!(responses.len(), 1);
		assert_eq!(responses[0].requests, vec![get()]);
		assert_eq!(responses[0].source.height.height, 300);
		assert_eq!(responses[0].response, stored_proof);
		drop(responses);

		assert!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().is_empty());
		cleanup(&path);
	}

	/// Updates queued behind the one being handled are drained into the newest height.
	#[tokio::test(start_paused = true)]
	async fn the_processor_drains_queued_updates_before_proving() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let responses = hyperbridge.get_responses.clone();
		let clients =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));

		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		sender.send((vec![get()], update(200))).await.unwrap();
		sender.send((vec![], update(300))).await.unwrap();
		drop(sender);

		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			clients,
			tx_payment.clone(),
		));
		assert!(processor.await.unwrap().is_err());
		wait_for_responses(&responses, 1).await;
		let responses = responses.lock().unwrap();
		assert_eq!(responses.len(), 1);
		assert_eq!(responses[0].source.height.height, 300);
		drop(responses);

		assert!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().is_empty());
		cleanup(&path);
	}
}
