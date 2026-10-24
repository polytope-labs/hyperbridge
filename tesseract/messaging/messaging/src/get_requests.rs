use std::{
	collections::{HashMap, HashSet},
	sync::{Arc, Mutex},
	time::Duration,
};

use anyhow::anyhow;
use futures::StreamExt;
use ismp::{
	consensus::StateMachineHeight,
	host::StateMachine,
	messaging::{hash_request, Message, Proof, ResponseMessage},
	router::{GetRequest, Request},
};
use pallet_state_coprocessor::impls::GetRequestsWithProof;
use primitive_types::H256;
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
	// Requests whose submission is still running. They are not proved again until it finishes.
	let in_flight = Arc::new(Mutex::new(HashSet::<H256>::new()));

	while let Some((mut get_requests, mut state_machine_update)) = receiver.recv().await {
		// Updates arrive in height order. Drain what is already queued and prove everything at
		// the newest height hyperbridge knows, so a backlog does not prove stale heights.
		while let Ok((more, update)) = receiver.try_recv() {
			get_requests.extend(more);
			state_machine_update = update;
		}

		let hyperbridge_timestamp = match hyperbridge.query_timestamp().await {
			Ok(timestamp) => Some(timestamp),
			Err(err) => {
				tracing::error!(target: crate::LOG_TARGET, "Failed to query timestamp of hyperbridge: {err:?}");
				None
			},
		};

		// Fetch each fresh request's destination proof and store it before trying the source
		// proof, so a restart from here still has the proof that ages fastest.
		let mut fetched = Vec::new();
		for req in get_requests {
			if let Some(timestamp) = hyperbridge_timestamp {
				if Request::Get(req.clone()).timed_out(timestamp) {
					tracing::trace!(target: crate::LOG_TARGET, "Skipping timed out get request from {} with nonce {}",req.source, req.nonce);
					continue
				}
			}

			let Some(client) = client_map.get(&req.dest) else {
				tracing::debug!(
					target: crate::LOG_TARGET, "Skipping get requests because client for {} was not found",
					req.dest
				);
				continue
			};

			match fetch_destination_proof(client, &req).await {
				Ok(proof) => fetched.push((req, proof)),
				Err(err) => {
					tracing::error!(target: crate::LOG_TARGET, ?err, "Failed to fetch the destination proof for a get request from {}; it was dropped", req.dest);
				},
			}
		}

		let mut unstored = Vec::new();
		if !fetched.is_empty() {
			let count = fetched.len();
			let rows = fetched
				.iter()
				.map(|(request, proof)| {
					get_request_row(request, proof.clone(), source.address())
				})
				.collect::<Vec<_>>();
			match tx_payment.store_unprofitable_messages(rows, sink).await {
				Ok(_) => {},
				Err(err) => {
					tracing::error!(target: crate::LOG_TARGET, ?err, requests = count, "Failed to store get requests; proving them from memory for this update");
					unstored = fetched
						.into_iter()
						.map(|(request, destination_proof)| GetRequestEntry {
							request,
							row_id: None,
							destination_proof,
						})
						.collect();
				},
			}
		}

		let rows = match tx_payment.unprofitable_messages(&sink).await {
			Ok(rows) => rows,
			Err(err) => {
				tracing::error!(target: crate::LOG_TARGET, ?err, "Failed to read parked get requests");
				Vec::new()
			},
		};
		// Only this source chain's single-request response rows are ours. Other rows belong to
		// POST parking or to another source chain's processor and are left untouched.
		let mut entries = Vec::new();
		for ParkedMessage { message, id, .. } in rows {
			let Message::Response(msg) = message else { continue };
			if msg.requests.len() != 1 {
				if !msg.requests.is_empty() {
					tracing::debug!(target: crate::LOG_TARGET, requests = msg.requests.len(), "Ignoring a get-request row that does not hold exactly one request");
				}
				continue
			}
			let request = msg.requests.into_iter().next().expect("length checked, qed");
			if request.source != source_state {
				continue
			}
			entries.push(GetRequestEntry {
				request,
				row_id: id,
				destination_proof: msg.proof,
			});
		}
		entries.extend(unstored);

		let Some(hyperbridge_timestamp) = hyperbridge_timestamp else { continue };

		let ProvedGetRequests { messages, delete } = prove_get_requests(
			&source,
			&hyperbridge,
			&client_map,
			entries,
			&state_machine_update,
			hyperbridge_timestamp,
			&in_flight,
		)
		.await;

		if !delete.is_empty() {
			if let Err(err) = tx_payment.delete_unprofitable_messages(delete).await {
				tracing::error!(target: crate::LOG_TARGET, ?err, "Failed to delete proved get-request rows");
			}
		}

		if !messages.is_empty() {
			// Mark before spawning so later updates do not prove the same GET again.
			in_flight
				.lock()
				.unwrap()
				.extend(messages.iter().map(|(commitment, _)| *commitment));

			// Submit in the background so a slow challenge period or submission never blocks
			// reading the next update from the channel.
			let hyperbridge = hyperbridge.clone();
			let source = source.clone();
			let client_map = client_map.clone();
			let in_flight = in_flight.clone();
			tokio::spawn(async move {
				let stream = futures::stream::iter(messages);
				stream
					.for_each_concurrent(None, |(commitment, msg)| {
						let hyperbridge = Arc::new(hyperbridge.clone());
						let source = source.clone();
						let in_flight = in_flight.clone();
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
							in_flight.lock().unwrap().remove(&commitment);
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
	/// Messages to submit, each with the request commitment it proves.
	messages: Vec<(H256, GetRequestsWithProof)>,
	/// Rows that are no longer needed: delivered, timed out, or without a destination client.
	delete: Vec<i32>,
}

/// One GET request to prove, with its destination proof.
struct GetRequestEntry {
	request: GetRequest,
	/// The row this entry was read from, if any.
	row_id: Option<i32>,
	destination_proof: Proof,
}

/// Build the `GetRequestsWithProof` messages for one update.
///
/// Each request is proved on its own, so its storage proof only covers its own keys. A request
/// already delivered to hyperbridge, timed out, or without a destination client contributes its
/// row to `delete`; one already being submitted is skipped. A source proof failure keeps the row
/// for a later update.
async fn prove_get_requests<H: IsmpProvider>(
	source: &Arc<dyn IsmpProvider>,
	hyperbridge: &H,
	client_map: &HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	entries: Vec<GetRequestEntry>,
	state_machine_update: &StateMachineUpdated,
	hyperbridge_timestamp: Duration,
	in_flight: &Arc<Mutex<HashSet<H256>>>,
) -> ProvedGetRequests {
	let mut result = ProvedGetRequests { messages: vec![], delete: vec![] };

	for entry in entries {
		let commitment = hash_request::<Hasher>(&Request::Get(entry.request.clone()));
		if in_flight.lock().unwrap().contains(&commitment) {
			continue
		}

		if Request::Get(entry.request.clone()).timed_out(hyperbridge_timestamp) {
			tracing::trace!(target: crate::LOG_TARGET, "Skipping timed out get request from {} with nonce {}",entry.request.source, entry.request.nonce);
			if let Some(id) = entry.row_id {
				result.delete.push(id);
			}
			continue
		}

		if !client_map.contains_key(&entry.request.dest) {
			if entry.row_id.is_some() {
				tracing::warn!(
					target: crate::LOG_TARGET, "Dropping a stored get request because client for {} was not found",
					entry.request.dest
				);
			} else {
				tracing::debug!(
					target: crate::LOG_TARGET, "Skipping get requests because client for {} was not found",
					entry.request.dest
				);
			}
			if let Some(id) = entry.row_id {
				result.delete.push(id);
			}
			continue
		}

		match hyperbridge.query_response_receipt(commitment).await {
			Ok(receipt) if is_delivered(&receipt) => {
				tracing::debug!(target: crate::LOG_TARGET, commitment = %commitment, "Get request already delivered; removing its row");
				if let Some(id) = entry.row_id {
					result.delete.push(id);
				}
				continue
			},
			Ok(_) => {},
			Err(err) => {
				tracing::error!(target: crate::LOG_TARGET, ?err, commitment = %commitment, "Failed to query the response receipt for a get request; keeping it for the next update");
				continue
			},
		}

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

		let query = StateProofQueryType::Ismp(source.request_commitment_full_key(commitment));
		tracing::trace!(target: crate::LOG_TARGET, "Fetching the source proof for get request {} from {}", entry.request.nonce, entry.request.source);
		match source
			.query_state_proof(state_machine_update.latest_height, query)
			.await
		{
			Ok(proof) => result.messages.push((
				commitment,
				GetRequestsWithProof {
					requests: vec![entry.request],
					source: Proof {
						height: StateMachineHeight {
							id: state_machine_update.state_machine_id,
							height: state_machine_update.latest_height,
						},
						proof,
					},
					response: entry.destination_proof,
					address: source.address(),
				},
			)),
			Err(err) => {
				tracing::error!(target: crate::LOG_TARGET, ?err, "Failed to fetch the source proof for a get request, kept it to prove at a newer height");
			},
		}
	}

	result
}

/// Store a GET request with its destination proof as one row.
fn get_request_row(request: &GetRequest, proof: Proof, signer: Vec<u8>) -> ParkedMessage {
	ParkedMessage {
		message: Message::Response(ResponseMessage {
			requests: vec![request.clone()],
			proof,
			signer,
		}),
		state: RetryState { created_at: unix_now(), ..Default::default() },
		id: None,
	}
}

/// Fetch the destination storage proof for one request's keys.
async fn fetch_destination_proof(
	client: &Arc<dyn IsmpProvider>,
	request: &GetRequest,
) -> Result<Proof, anyhow::Error> {
	tracing::trace!(target: crate::LOG_TARGET, "Fetching state proofs for {} keys from {}", request.keys.len(), request.dest);
	let proof = client
		.query_state_proof(
			request.height,
			StateProofQueryType::Arbitrary(request.keys.clone()),
		)
		.await?;
	Ok(Proof {
		height: StateMachineHeight {
			id: client.state_machine_id(),
			height: request.height,
		},
		proof,
	})
}

/// Whether a response receipt means the GET was delivered.
///
/// The substrate client returns 20 zero bytes when no receipt exists.
fn is_delivered(receipt: &[u8]) -> bool {
	!receipt.is_empty() && receipt.iter().any(|byte| *byte != 0)
}

#[cfg(test)]
mod tests {
	use super::*;
	use ismp::{
		consensus::StateMachineId,
		messaging::RequestMessage,
		router::PostRequest,
	};
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

	fn get_with_key(key: u8) -> GetRequest {
		GetRequest { keys: vec![vec![key]], ..get() }
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

	fn commitment(request: &GetRequest) -> H256 {
		hash_request::<Hasher>(&Request::Get(request.clone()))
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

	fn entry(
		request: GetRequest,
		destination_proof: Proof,
		row_id: Option<i32>,
	) -> GetRequestEntry {
		GetRequestEntry { request, row_id, destination_proof }
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

	/// A request being submitted is neither proved again nor deleted.
	#[tokio::test]
	async fn an_in_flight_get_request_is_neither_proved_nor_deleted() {
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let clients =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));
		let request = get();
		let in_flight = Arc::new(Mutex::new(HashSet::from([commitment(&request)])));

		let result = prove_get_requests(
			&source,
			&hyperbridge,
			&clients,
			vec![entry(request, destination_proof(STORED_PROOF.to_vec()), Some(1))],
			&update(300),
			Duration::from_secs(100),
			&in_flight,
		)
		.await;
		assert!(result.messages.is_empty());
		assert!(result.delete.is_empty());
	}

	/// A request with a response receipt is deleted without being proved again.
	#[tokio::test]
	async fn a_delivered_get_request_is_deleted() {
		let request = get();
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge = MockHost::new((), 0, COPROCESSOR)
			.with_timestamp(Duration::from_secs(100))
			.with_response_receipt(commitment(&request), vec![1]);
		let clients =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));
		let in_flight = Arc::new(Mutex::new(HashSet::new()));

		let result = prove_get_requests(
			&source,
			&hyperbridge,
			&clients,
			vec![entry(request, destination_proof(STORED_PROOF.to_vec()), Some(2))],
			&update(300),
			Duration::from_secs(100),
			&in_flight,
		)
		.await;
		assert!(result.messages.is_empty());
		assert_eq!(result.delete, vec![2]);
	}

	/// Two GETs at the same destination and height are proved separately, each with a destination
	/// proof fetched for its own keys.
	#[tokio::test(start_paused = true)]
	async fn two_gets_at_the_same_height_get_their_own_proofs() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let responses = hyperbridge.get_responses.clone();
		let dest = MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec());
		let dest_queries = dest.state_proof_queries.clone();
		let clients = dest_clients(dest);

		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			clients,
			tx_payment.clone(),
		));
		let (first, second) = (get_with_key(3), get_with_key(4));
		sender.send((vec![first.clone(), second.clone()], update(200))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());
		wait_for_responses(&responses, 2).await;

		let responses = responses.lock().unwrap();
		assert_eq!(responses.len(), 2);
		for response in responses.iter() {
			assert_eq!(response.requests.len(), 1);
			assert!(response.requests[0] == first || response.requests[0] == second);
		}
		drop(responses);

		let queries = dest_queries.lock().unwrap();
		assert_eq!(queries.len(), 2);
		for (height, keys) in queries.iter() {
			assert_eq!(*height, first.height);
			match keys {
				StateProofQueryType::Arbitrary(keys) => {
					assert!(keys == &first.keys || keys == &second.keys);
				},
				other => panic!("expected an arbitrary key query, got {other:?}"),
			}
		}
		drop(queries);

		assert_eq!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().len(), 2);
		cleanup(&path);
	}

	/// A submitted GET keeps its row until hyperbridge has a response receipt for it.
	#[tokio::test(start_paused = true)]
	async fn a_submitted_get_is_deleted_only_after_the_response_receipt() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let request = get();

		// First processor submits the request but no receipt exists yet.
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
		sender.send((vec![request.clone()], update(200))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());
		wait_for_responses(&responses, 1).await;
		assert_eq!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().len(), 1);

		// A later processor sees the receipt and removes the row without submitting again.
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge = MockHost::new((), 0, COPROCESSOR)
			.with_timestamp(Duration::from_secs(100))
			.with_response_receipt(commitment(&request), vec![1]);
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
		sender.send((vec![], update(300))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());

		assert!(responses.lock().unwrap().is_empty());
		assert!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().is_empty());
		cleanup(&path);
	}

	/// A failed dry run leaves the row in place and the next update submits it again.
	#[tokio::test(start_paused = true)]
	async fn a_failed_dry_run_is_retried_at_the_next_update() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge = MockHost::new((), 0, COPROCESSOR)
			.with_timestamp(Duration::from_secs(100))
			.with_dry_run_fail();
		let dry_run_ok = hyperbridge.dry_run_ok.clone();
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
		// Let the submission run and fail, which also clears the in-flight mark.
		tokio::time::sleep(Duration::from_secs(1)).await;
		assert!(responses.lock().unwrap().is_empty());
		assert_eq!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().len(), 1);

		*dry_run_ok.lock().unwrap() = true;
		sender.send((vec![], update(300))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());

		wait_for_responses(&responses, 1).await;
		let responses = responses.lock().unwrap();
		assert_eq!(responses.len(), 1);
		assert_eq!(responses[0].source.height.height, 300);
		drop(responses);
		assert_eq!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().len(), 1);
		cleanup(&path);
	}

	/// A source proof failure stores the destination proof; a new processor on the same database
	/// proves the request later even with the destination down.
	#[tokio::test(start_paused = true)]
	async fn a_new_processor_reuses_the_stored_destination_proof() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());

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

		assert_eq!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().len(), 1);
		cleanup(&path);
	}

	/// A destination proof failure drops the request without storing or submitting it.
	#[tokio::test(start_paused = true)]
	async fn a_destination_proof_failure_drops_the_request() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let responses = hyperbridge.get_responses.clone();
		let clients = dest_clients(MockHost::new((), 0, DEST).with_state_proof_fail());
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

		assert!(responses.lock().unwrap().is_empty());
		assert!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().is_empty());
		cleanup(&path);
	}

	/// A timestamp failure still stores the destination proof, and the next update proves the
	/// request from that stored proof.
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
		assert!(responses.lock().unwrap().is_empty());

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

		assert_eq!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().len(), 1);
		cleanup(&path);
	}

	/// A stored request that has timed out is deleted and not submitted.
	#[tokio::test(start_paused = true)]
	async fn a_stored_request_that_times_out_is_deleted() {
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

	/// A stored request whose destination client is gone is deleted and not submitted.
	#[tokio::test(start_paused = true)]
	async fn a_stored_request_without_a_destination_client_is_deleted() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		tx_payment
			.store_unprofitable_messages(
				vec![stored_row(vec![get()], destination_proof(STORED_PROOF.to_vec()), None)],
				COPROCESSOR,
			)
			.await
			.unwrap();

		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let responses = hyperbridge.get_responses.clone();
		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			HashMap::new(),
			tx_payment.clone(),
		));
		sender.send((vec![], update(300))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());

		assert!(responses.lock().unwrap().is_empty());
		assert!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().is_empty());
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
		assert_eq!(rows.len(), 2);
		assert!(rows.iter().any(|row| {
			matches!(&row.message, Message::Request(msg) if msg.requests == vec![post.clone()])
		}));
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

		assert_eq!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().len(), 1);
		cleanup(&path);
	}
}
