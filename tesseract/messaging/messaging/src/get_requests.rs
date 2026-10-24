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

use crate::retries::{next_retry_state, retry_is_due, RetryDue};

pub async fn process_get_request_events<
	A: IsmpProvider + HandleGetResponse + Clone + Clone + 'static,
>(
	mut receiver: Receiver<(Vec<GetRequest>, StateMachineUpdated)>,
	source: Arc<dyn IsmpProvider>,
	hyperbridge: A,
	client_map: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	tx_payment: Arc<TransactionPayment>,
	retry_frequency: Duration,
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

		let now = unix_now();
		let hyperbridge_timestamp = match hyperbridge.query_timestamp().await {
			Ok(timestamp) => Some(timestamp),
			Err(err) => {
				tracing::error!(target: crate::LOG_TARGET, "Failed to query timestamp of hyperbridge: {err:?}");
				None
			},
		};

		// Fetch each fresh request's destination proof and store it before trying the source
		// proof, so a restart from here still has the proof that ages fastest. A request whose
		// destination proof fails is stored without one and retried within the same caps as a
		// parked POST.
		let mut fresh = Vec::new();
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

			let commitment = hash_request::<Hasher>(&Request::Get(req.clone()));
			match fetch_destination_proof(client, &req).await {
				Ok(proof) => {
					let state = RetryState { created_at: now, attempts: 0, next_attempt_at: 0 };
					fresh.push((req, proof, state));
				},
				Err(err) => {
					let state =
						next_retry_state(RetryState::default(), now, retry_frequency, true);
					tracing::error!(target: crate::LOG_TARGET, ?err, commitment = %commitment, attempts = state.attempts, "Failed to fetch the destination proof for a get request; stored it without one");
					fresh.push((req.clone(), empty_destination_proof(client, &req), state));
				},
			}
		}

		let mut unstored = Vec::new();
		if !fresh.is_empty() {
			let count = fresh.len();
			let rows = fresh
				.iter()
				.map(|(request, proof, state)| {
					get_request_row(request, proof.clone(), source.address(), *state)
				})
				.collect::<Vec<_>>();
			match tx_payment.store_unprofitable_messages(rows, sink).await {
				Ok(_) => {},
				Err(err) => {
					tracing::error!(target: crate::LOG_TARGET, ?err, requests = count, "Failed to store get requests; proving them from memory for this update");
					unstored = fresh
						.into_iter()
						.map(|(request, destination_proof, state)| GetRequestEntry {
							request,
							row_id: None,
							destination_proof,
							state,
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
		for ParkedMessage { message, state, id } in rows {
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
				state,
			});
		}
		entries.extend(unstored);

		let Some(hyperbridge_timestamp) = hyperbridge_timestamp else {
			let unstored = entries.iter().filter(|entry| entry.row_id.is_none()).count();
			if unstored > 0 {
				tracing::error!(target: crate::LOG_TARGET, unstored, "Dropping get requests that could not be stored because the hyperbridge timestamp is unavailable");
			}
			continue
		};

		let ProvedGetRequests { messages, store, update, delete } = prove_get_requests(
			&source,
			&hyperbridge,
			&client_map,
			entries,
			&state_machine_update,
			hyperbridge_timestamp,
			retry_frequency,
			now,
			&in_flight,
		)
		.await;

		if !store.is_empty() {
			let new_requests = store
				.iter()
				.map(|parked| match &parked.message {
					Message::Response(msg) => msg.requests.len(),
					_ => 0,
				})
				.sum::<usize>();
			if let Err(err) = tx_payment.store_unprofitable_messages(store, sink).await {
				tracing::error!(
					target: crate::LOG_TARGET,
					?err,
					lost_requests = new_requests,
					"Failed to store get requests: {} new get requests lost",
					new_requests,
				);
			}
		}
		for (id, commitment, parked) in update {
			if let Err(err) = tx_payment.update_unprofitable_message(id, parked).await {
				tracing::error!(
					target: crate::LOG_TARGET,
					?err,
					id,
					commitment = %commitment,
					"Failed to update a get-request row; it keeps its old state",
				);
			}
		}
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
				.extend(messages.iter().map(|(commitment, _, _)| *commitment));

			// Submit in the background so a slow challenge period or submission never blocks
			// reading the next update from the channel.
			let hyperbridge = hyperbridge.clone();
			let source = source.clone();
			let in_flight = in_flight.clone();
			tokio::spawn(async move {
				let stream = futures::stream::iter(messages);
				stream
					.for_each_concurrent(None, |(commitment, msg, dest)| {
						let hyperbridge = Arc::new(hyperbridge.clone());
						let source = source.clone();
						let in_flight = in_flight.clone();
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

								tracing::info!(target: crate::LOG_TARGET, commitment = %commitment, "Submitting get response",);
								hyperbridge.submit_get_response(msg).await?;

								Ok::<_, anyhow::Error>(())
							};
							match lambda().await {
								Ok(()) => {},
								Err(err) => {
									tracing::error!(target: crate::LOG_TARGET, commitment = %commitment, "Error submitting get response \n{err:?}");
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
	/// Messages to submit, each with its commitment and destination client.
	messages: Vec<(H256, GetRequestsWithProof, Arc<dyn IsmpProvider>)>,
	/// Rows to insert for entries that have none yet.
	store: Vec<ParkedMessage>,
	/// Existing rows to rewrite in place, with the commitment for logging.
	update: Vec<(i32, H256, ParkedMessage)>,
	/// Rows that are no longer needed.
	delete: Vec<i32>,
}

/// One GET request to prove, with its destination proof and retry bookkeeping.
struct GetRequestEntry {
	request: GetRequest,
	/// The row this entry was read from, if any.
	row_id: Option<i32>,
	destination_proof: Proof,
	state: RetryState,
}

/// Build the `GetRequestsWithProof` messages for one update.
///
/// Each request is proved on its own, so its storage proof only covers its own keys. A request is
/// skipped until its retry backoff elapses, and is deleted once delivered, timed out, expired, or
/// without a destination client. A failed destination-proof fetch counts an attempt and is stored
/// back without a proof; a source-proof failure keeps the row unchanged, since the source height
/// only moves forward.
async fn prove_get_requests<H: IsmpProvider>(
	source: &Arc<dyn IsmpProvider>,
	hyperbridge: &H,
	client_map: &HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	entries: Vec<GetRequestEntry>,
	state_machine_update: &StateMachineUpdated,
	hyperbridge_timestamp: Duration,
	retry_frequency: Duration,
	now: u64,
	in_flight: &Arc<Mutex<HashSet<H256>>>,
) -> ProvedGetRequests {
	let mut result = ProvedGetRequests {
		messages: vec![],
		store: vec![],
		update: vec![],
		delete: vec![],
	};
	// One submission per GET per pass; the first row or fresh copy seen wins.
	let mut seen = HashSet::new();

	for entry in entries {
		let commitment = hash_request::<Hasher>(&Request::Get(entry.request.clone()));
		if !seen.insert(commitment) {
			// Another row for this GET already won; drop the duplicate.
			if let Some(id) = entry.row_id {
				result.delete.push(id);
			}
			continue
		}
		if in_flight.lock().unwrap().contains(&commitment) {
			if entry.row_id.is_none() {
				keep_row(
					&mut result,
					entry.row_id,
					commitment,
					get_request_row(
						&entry.request,
						entry.destination_proof,
						source.address(),
						entry.state,
					),
				);
			}
			continue
		}

		let due = retry_is_due(entry.state, now);
		if due == RetryDue::NotYet {
			if entry.row_id.is_none() {
				keep_row(
					&mut result,
					entry.row_id,
					commitment,
					get_request_row(
						&entry.request,
						entry.destination_proof,
						source.address(),
						entry.state,
					),
				);
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
				if entry.row_id.is_none() {
					keep_row(
						&mut result,
						entry.row_id,
						commitment,
						get_request_row(
							&entry.request,
							entry.destination_proof,
							source.address(),
							entry.state,
						),
					);
				}
				continue
			},
		}

		if due == RetryDue::Expired {
			tracing::warn!(
				target: crate::LOG_TARGET,
				commitment = %commitment,
				attempts = entry.state.attempts,
				age = now.saturating_sub(entry.state.created_at),
				"Dropping a get request that has used up its retries",
			);
			if let Some(id) = entry.row_id {
				result.delete.push(id);
			}
			continue
		}

		if Request::Get(entry.request.clone()).timed_out(hyperbridge_timestamp) {
			tracing::trace!(target: crate::LOG_TARGET, "Skipping timed out get request from {} with nonce {}",entry.request.source, entry.request.nonce);
			if let Some(id) = entry.row_id {
				result.delete.push(id);
			}
			continue
		}

		let Some(destination_client) = client_map.get(&entry.request.dest).cloned() else {
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
		};

		let proof_missing = entry.destination_proof.proof.is_empty();
		let destination_proof = if proof_missing {
			match fetch_destination_proof(&destination_client, &entry.request).await {
				Ok(proof) => proof,
				Err(err) => {
					let state = next_retry_state(entry.state, now, retry_frequency, true);
					tracing::error!(target: crate::LOG_TARGET, ?err, commitment = %commitment, attempts = state.attempts, "Failed to fetch the destination proof for a get request; backing off");
					keep_row(
						&mut result,
						entry.row_id,
						commitment,
						get_request_row(
							&entry.request,
							empty_destination_proof(&destination_client, &entry.request),
							source.address(),
							state,
						),
					);
					continue
				},
			}
		} else {
			entry.destination_proof
		};

		let query = StateProofQueryType::Ismp(source.request_commitment_full_key(commitment));
		tracing::trace!(target: crate::LOG_TARGET, "Fetching the source proof for get request {} from {}", entry.request.nonce, entry.request.source);
		match source
			.query_state_proof(state_machine_update.latest_height, query)
			.await
		{
			Ok(proof) => {
				let state = next_retry_state(entry.state, now, retry_frequency, true);
				keep_row(
					&mut result,
					entry.row_id,
					commitment,
					get_request_row(
						&entry.request,
						destination_proof.clone(),
						source.address(),
						state,
					),
				);
				result.messages.push((
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
						response: destination_proof,
						address: source.address(),
					},
					destination_client,
				));
			},
			Err(err) => {
				tracing::error!(target: crate::LOG_TARGET, ?err, commitment = %commitment, "Failed to fetch the source proof for a get request, kept it to prove at a newer height");
				// Keep a destination proof fetched in this pass, or a row that was never stored.
				if entry.row_id.is_none() || proof_missing {
					keep_row(
						&mut result,
						entry.row_id,
						commitment,
						get_request_row(
							&entry.request,
							destination_proof,
							source.address(),
							entry.state,
						),
					);
				}
			},
		}
	}

	result
}

/// Store a GET request with its destination proof and retry state as one row.
fn get_request_row(
	request: &GetRequest,
	proof: Proof,
	signer: Vec<u8>,
	state: RetryState,
) -> ParkedMessage {
	ParkedMessage {
		message: Message::Response(ResponseMessage {
			requests: vec![request.clone()],
			proof,
			signer,
		}),
		state,
		id: None,
	}
}

/// Keep an entry: rewrite its row in place, or store a new row when it has none.
fn keep_row(
	result: &mut ProvedGetRequests,
	row_id: Option<i32>,
	commitment: H256,
	parked: ParkedMessage,
) {
	match row_id {
		Some(id) => result.update.push((id, commitment, parked)),
		None => result.store.push(parked),
	}
}

/// A row with empty proof bytes has no destination proof yet. The carrier height records the
/// destination and retrieval height it belongs to.
fn empty_destination_proof(
	client: &Arc<dyn IsmpProvider>,
	request: &GetRequest,
) -> Proof {
	Proof {
		height: StateMachineHeight {
			id: client.state_machine_id(),
			height: request.height,
		},
		proof: Vec::new(),
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
	use ismp::{consensus::StateMachineId, messaging::RequestMessage, router::PostRequest};
	use std::sync::atomic::{AtomicU64, Ordering};
	use tesseract_primitives::mocks::MockHost;

	/// Makes every test database path unique within this process.
	static NEXT_DB: AtomicU64 = AtomicU64::new(0);

	const SOURCE: StateMachine = StateMachine::Evm(97);
	const DEST: StateMachine = StateMachine::Evm(80002);
	const COPROCESSOR: StateMachine = StateMachine::Kusama(4009);
	const STORED_PROOF: [u8; 3] = [7, 8, 9];
	const FREQUENCY: Duration = Duration::from_secs(60);
	const NOW: u64 = 1_000_000;

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

	fn fresh_state(now: u64) -> RetryState {
		RetryState { created_at: now, attempts: 0, next_attempt_at: 0 }
	}

	fn entry(
		request: GetRequest,
		destination_proof: Proof,
		row_id: Option<i32>,
		state: RetryState,
	) -> GetRequestEntry {
		GetRequestEntry { request, row_id, destination_proof, state }
	}

	fn stored_row(
		requests: Vec<GetRequest>,
		proof: Proof,
		state: RetryState,
		id: Option<i32>,
	) -> ParkedMessage {
		ParkedMessage {
			message: Message::Response(ResponseMessage { requests, proof, signer: Vec::new() }),
			state,
			id,
		}
	}

	fn default_state() -> RetryState {
		RetryState { created_at: 0, attempts: 0, next_attempt_at: 0 }
	}

	/// Build a unique on-disk path for a test database.
	fn temp_db_path() -> String {
		let pid = std::process::id();
		let nanos = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|d| d.as_nanos())
			.unwrap_or(0);
		let seq = NEXT_DB.fetch_add(1, Ordering::Relaxed);
		format!("/tmp/tesseract-get-requests-test-{pid}-{nanos}-{seq}.db")
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

	/// Wait under paused time until `count` dry runs have started.
	async fn wait_for_dry_runs(started: &Arc<Mutex<usize>>, count: usize) {
		for _ in 0..100 {
			if *started.lock().unwrap() >= count {
				return
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
		panic!("expected {count} dry runs, got {}", started.lock().unwrap());
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
			vec![entry(
				request,
				destination_proof(STORED_PROOF.to_vec()),
				Some(1),
				fresh_state(NOW),
			)],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			NOW,
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
			vec![entry(
				request,
				destination_proof(STORED_PROOF.to_vec()),
				Some(2),
				fresh_state(NOW),
			)],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			NOW,
			&in_flight,
		)
		.await;
		assert!(result.messages.is_empty());
		assert_eq!(result.delete, vec![2]);
	}

	/// While a submission is running, a newer update neither proves nor deletes the same GET.
	#[tokio::test(start_paused = true)]
	async fn the_in_flight_guard_stops_a_second_submission() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let gate = Arc::new(tokio::sync::Notify::new());
		let hyperbridge = MockHost::new((), 0, COPROCESSOR)
			.with_timestamp(Duration::from_secs(100))
			.with_dry_run_gate(gate.clone());
		let dry_runs = hyperbridge.dry_runs_started.clone();
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
			Duration::ZERO,
		));

		sender.send((vec![get()], update(200))).await.unwrap();
		wait_for_dry_runs(&dry_runs, 1).await;

		// The request is being submitted; the next update must not prove it again.
		sender.send((vec![], update(300))).await.unwrap();
		tokio::time::sleep(Duration::from_secs(1)).await;
		assert_eq!(*dry_runs.lock().unwrap(), 1);
		assert_eq!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().len(), 1);

		gate.notify_one();
		drop(sender);
		assert!(processor.await.unwrap().is_err());
		wait_for_responses(&responses, 1).await;
		assert_eq!(*dry_runs.lock().unwrap(), 1);
		assert_eq!(responses.lock().unwrap().len(), 1);
		cleanup(&path);
	}

	/// A response-receipt query error keeps the row and skips it this update.
	#[tokio::test(start_paused = true)]
	async fn a_receipt_query_error_keeps_the_row() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		tx_payment
			.store_unprofitable_messages(
				vec![stored_row(
					vec![get()],
					destination_proof(STORED_PROOF.to_vec()),
					default_state(),
					None,
				)],
				COPROCESSOR,
			)
			.await
			.unwrap();

		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge = MockHost::new((), 0, COPROCESSOR)
			.with_timestamp(Duration::from_secs(100))
			.with_response_receipt_fail();
		let responses = hyperbridge.get_responses.clone();
		let clients = dest_clients(MockHost::new((), 0, DEST));
		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			clients,
			tx_payment.clone(),
			Duration::ZERO,
		));
		sender.send((vec![], update(300))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());

		assert!(responses.lock().unwrap().is_empty());
		assert_eq!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().len(), 1);
		cleanup(&path);
	}

	/// A failed destination-proof fetch counts an attempt and backs off. Once due, the fetched
	/// proof is stored and the request is proved.
	#[tokio::test]
	async fn a_failed_destination_proof_fetch_counts_an_attempt_and_backs_off() {
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let destination_down = dest_clients(MockHost::new((), 0, DEST).with_state_proof_fail());
		let in_flight = Arc::new(Mutex::new(HashSet::new()));
		let request = get();

		let result = prove_get_requests(
			&source,
			&hyperbridge,
			&destination_down,
			vec![entry(request.clone(), destination_proof(Vec::new()), Some(5), fresh_state(NOW))],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			NOW,
			&in_flight,
		)
		.await;
		assert!(result.messages.is_empty());
		assert!(result.delete.is_empty());
		assert!(result.store.is_empty());
		assert_eq!(result.update.len(), 1);
		let (failed_id, _, failed) = &result.update[0];
		assert_eq!(*failed_id, 5);
		assert_eq!(failed.state.attempts, 1);
		assert_eq!(failed.state.next_attempt_at, NOW + FREQUENCY.as_secs());
		match &failed.message {
			Message::Response(msg) => assert!(msg.proof.proof.is_empty()),
			other => panic!("expected a response row, got {other:?}"),
		}
		let failed_state = failed.state;

		let destination_up =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));
		let retry = entry(
			request.clone(),
			destination_proof(Vec::new()),
			Some(5),
			failed_state,
		);

		// Before the backoff elapses the row is skipped.
		let result = prove_get_requests(
			&source,
			&hyperbridge,
			&destination_up,
			vec![retry],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			failed_state.next_attempt_at - 1,
			&in_flight,
		)
		.await;
		assert!(result.messages.is_empty());
		assert!(result.store.is_empty());
		assert!(result.update.is_empty());
		assert!(result.delete.is_empty());

		// Once due, the destination proof is fetched and kept with a counted attempt.
		let result = prove_get_requests(
			&source,
			&hyperbridge,
			&destination_up,
			vec![entry(
				request,
				destination_proof(Vec::new()),
				Some(5),
				failed_state,
			)],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			failed_state.next_attempt_at,
			&in_flight,
		)
		.await;
		assert_eq!(result.messages.len(), 1);
		assert_eq!(result.messages[0].1.response.proof, STORED_PROOF.to_vec());
		assert_eq!(result.update.len(), 1);
		assert_eq!(result.update[0].2.state.attempts, 2);
	}

	/// A destination proof fetched in a pass is kept when the source proof then fails, with no
	/// attempt counted.
	#[tokio::test]
	async fn a_fetched_destination_proof_is_kept_when_the_source_proof_fails() {
		let source = Arc::new(MockHost::new((), 0, SOURCE).with_state_proof_fail())
			as Arc<dyn IsmpProvider>;
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let clients =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));
		let in_flight = Arc::new(Mutex::new(HashSet::new()));
		let request = get();
		let state = RetryState { created_at: NOW - 100, attempts: 1, next_attempt_at: 0 };

		let result = prove_get_requests(
			&source,
			&hyperbridge,
			&clients,
			vec![entry(request.clone(), destination_proof(Vec::new()), Some(6), state)],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			NOW,
			&in_flight,
		)
		.await;
		assert!(result.messages.is_empty());
		assert!(result.delete.is_empty());
		assert!(result.store.is_empty());
		assert_eq!(result.update.len(), 1);
		let (id, _, stored) = &result.update[0];
		assert_eq!(*id, 6);
		assert_eq!(stored.state, state);
		match &stored.message {
			Message::Response(msg) => assert_eq!(msg.proof.proof, STORED_PROOF.to_vec()),
			other => panic!("expected a response row, got {other:?}"),
		}
	}

	/// Rows at the attempt cap, past the age cap, or not yet due are handled appropriately.
	#[tokio::test]
	async fn get_retry_caps_drop_expired_rows_and_skip_rows_that_are_not_due() {
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let clients =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));
		let in_flight = Arc::new(Mutex::new(HashSet::new()));

		// Ten attempts is the POST parking cap.
		let result = prove_get_requests(
			&source,
			&hyperbridge,
			&clients,
			vec![entry(
				get(),
				destination_proof(STORED_PROOF.to_vec()),
				Some(1),
				RetryState { created_at: NOW - 1, attempts: 10, next_attempt_at: 0 },
			)],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			NOW,
			&in_flight,
		)
		.await;
		assert!(result.messages.is_empty());
		assert_eq!(result.delete, vec![1]);

		// A failed attempt older than a day is expired.
		let result = prove_get_requests(
			&source,
			&hyperbridge,
			&clients,
			vec![entry(
				get_with_key(4),
				destination_proof(STORED_PROOF.to_vec()),
				Some(2),
				RetryState {
					created_at: NOW - 24 * 60 * 60 - 1,
					attempts: 1,
					next_attempt_at: 0,
				},
			)],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			NOW,
			&in_flight,
		)
		.await;
		assert!(result.messages.is_empty());
		assert_eq!(result.delete, vec![2]);

		// A row still in its backoff is neither proved nor deleted.
		let result = prove_get_requests(
			&source,
			&hyperbridge,
			&clients,
			vec![entry(
				get_with_key(5),
				destination_proof(STORED_PROOF.to_vec()),
				Some(3),
				RetryState { created_at: NOW - 1, attempts: 1, next_attempt_at: NOW + 100 },
			)],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			NOW,
			&in_flight,
		)
		.await;
		assert!(result.messages.is_empty());
		assert!(result.store.is_empty());
		assert!(result.delete.is_empty());
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
			Duration::ZERO,
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
		assert_ne!(responses[0].requests[0], responses[1].requests[0]);
		drop(responses);

		let queries = dest_queries.lock().unwrap();
		assert_eq!(queries.len(), 2);
		let mut seen = HashSet::new();
		for (height, keys) in queries.iter() {
			assert_eq!(*height, first.height);
			match keys {
				StateProofQueryType::Arbitrary(keys) => {
					assert!(seen.insert(keys.clone()));
				},
				other => panic!("expected an arbitrary key query, got {other:?}"),
			}
		}
		assert_eq!(seen, HashSet::from([vec![vec![3]], vec![vec![4]]]));
		drop(queries);

		assert_eq!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().len(), 2);
		cleanup(&path);
	}

	/// Two rows holding the same GET produce one message.
	#[tokio::test]
	async fn two_rows_for_the_same_get_produce_one_message() {
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let clients =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));
		let in_flight = Arc::new(Mutex::new(HashSet::new()));
		let request = get();

		let result = prove_get_requests(
			&source,
			&hyperbridge,
			&clients,
			vec![
				entry(
					request.clone(),
					destination_proof(STORED_PROOF.to_vec()),
					Some(1),
					fresh_state(NOW),
				),
				entry(
					request,
					destination_proof(Vec::new()),
					Some(2),
					fresh_state(NOW),
				),
			],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			NOW,
			&in_flight,
		)
		.await;
		assert_eq!(result.messages.len(), 1);
		assert_eq!(result.delete, vec![2]);
		assert_eq!(result.update.len(), 1);
		assert_eq!(result.update[0].0, 1);
	}

	/// A failed destination-proof fetch on a fresh GET is stored and tried again once due.
	#[tokio::test(start_paused = true)]
	async fn a_failed_destination_proof_on_a_fresh_get_backs_off_then_proves() {
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
			FREQUENCY,
		));
		sender.send((vec![get()], update(200))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());

		let rows = tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap();
		assert_eq!(rows.len(), 1);
		let row = rows.into_iter().next().unwrap();
		assert_eq!(row.state.attempts, 1);
		assert!(row.state.next_attempt_at > row.state.created_at);
		match &row.message {
			Message::Response(msg) => assert!(msg.proof.proof.is_empty()),
			other => panic!("expected a response row, got {other:?}"),
		}
		assert!(responses.lock().unwrap().is_empty());

		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let destination_up =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));
		let in_flight = Arc::new(Mutex::new(HashSet::new()));
		let retry = entry(get(), destination_proof(Vec::new()), row.id, row.state);

		let before = prove_get_requests(
			&source,
			&hyperbridge,
			&destination_up,
			vec![retry],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			row.state.next_attempt_at - 1,
			&in_flight,
		)
		.await;
		assert!(before.messages.is_empty());
		assert!(before.store.is_empty());
		assert!(before.delete.is_empty());

		let due = prove_get_requests(
			&source,
			&hyperbridge,
			&destination_up,
			vec![entry(get(), destination_proof(Vec::new()), row.id, row.state)],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			row.state.next_attempt_at,
			&in_flight,
		)
		.await;
		assert_eq!(due.messages.len(), 1);
		assert_eq!(due.messages[0].1.response.proof, STORED_PROOF.to_vec());
		assert_eq!(due.update[0].2.state.attempts, 2);
		cleanup(&path);
	}

	/// An unstored entry whose source proof fails is kept for a later store attempt.
	#[tokio::test]
	async fn an_unstored_entry_is_kept_when_the_source_proof_fails() {
		let source = Arc::new(MockHost::new((), 0, SOURCE).with_state_proof_fail())
			as Arc<dyn IsmpProvider>;
		let hyperbridge =
			MockHost::new((), 0, COPROCESSOR).with_timestamp(Duration::from_secs(100));
		let clients =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));
		let in_flight = Arc::new(Mutex::new(HashSet::new()));
		let state = fresh_state(NOW);

		let result = prove_get_requests(
			&source,
			&hyperbridge,
			&clients,
			vec![entry(
				get(),
				destination_proof(STORED_PROOF.to_vec()),
				None,
				state,
			)],
			&update(300),
			Duration::from_secs(100),
			FREQUENCY,
			NOW,
			&in_flight,
		)
		.await;
		assert!(result.messages.is_empty());
		assert!(result.update.is_empty());
		assert!(result.delete.is_empty());
		assert_eq!(result.store.len(), 1);
		assert_eq!(result.store[0].state, state);
		match &result.store[0].message {
			Message::Response(msg) => assert_eq!(msg.proof.proof, STORED_PROOF.to_vec()),
			other => panic!("expected a response row, got {other:?}"),
		}
	}

	/// A counted submission rewrites the same row in place.
	#[tokio::test(start_paused = true)]
	async fn a_counted_submission_updates_the_same_row() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let source = MockHost::new((), 0, SOURCE).with_state_proof_fail();
		let source_proof_ok = source.state_proof_ok.clone();
		let source: Arc<dyn IsmpProvider> = Arc::new(source);
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
			Duration::ZERO,
		));

		sender.send((vec![get()], update(200))).await.unwrap();
		tokio::time::sleep(Duration::from_secs(1)).await;
		let rows = tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap();
		assert_eq!(rows.len(), 1);
		let id = rows[0].id.unwrap();
		assert_eq!(rows[0].state.attempts, 0);
		assert!(responses.lock().unwrap().is_empty());

		*source_proof_ok.lock().unwrap() = true;
		sender.send((vec![], update(300))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());
		wait_for_responses(&responses, 1).await;

		let rows = tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap();
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].id, Some(id));
		assert_eq!(rows[0].state.attempts, 1);
		cleanup(&path);
	}

	/// A submission attempt counts toward the caps and sets the backoff.
	#[tokio::test(start_paused = true)]
	async fn each_submission_attempt_counts() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let source: Arc<dyn IsmpProvider> = Arc::new(MockHost::new((), 0, SOURCE));
		let hyperbridge = MockHost::new((), 0, COPROCESSOR)
			.with_timestamp(Duration::from_secs(100))
			.with_dry_run_fail();
		let dry_run_ok = hyperbridge.dry_run_ok.clone();
		let dry_runs = hyperbridge.dry_runs_started.clone();
		let responses = hyperbridge.get_responses.clone();
		let clients =
			dest_clients(MockHost::new((), 0, DEST).with_state_proof(STORED_PROOF.to_vec()));
		let frequency = Duration::from_secs(60 * 60);
		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			clients,
			tx_payment.clone(),
			frequency,
		));

		sender.send((vec![get()], update(200))).await.unwrap();
		// Let the submission run and fail.
		tokio::time::sleep(Duration::from_secs(1)).await;
		let rows = tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap();
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].state.attempts, 1);
		assert!(rows[0].state.next_attempt_at > transaction_fees::unix_now());
		assert_eq!(*dry_runs.lock().unwrap(), 1);

		// An update before the backoff elapses does not resubmit.
		*dry_run_ok.lock().unwrap() = true;
		sender.send((vec![], update(300))).await.unwrap();
		tokio::time::sleep(Duration::from_secs(1)).await;
		assert_eq!(*dry_runs.lock().unwrap(), 1);
		assert!(responses.lock().unwrap().is_empty());

		// Make the row due; the next update submits it.
		let row = tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().remove(0);
		let old_id = row.id;
		tx_payment
			.store_unprofitable_messages(
				vec![ParkedMessage {
					state: RetryState { next_attempt_at: 0, ..row.state },
					id: None,
					..row
				}],
				COPROCESSOR,
			)
			.await
			.unwrap();
		tx_payment.delete_unprofitable_messages(old_id).await.unwrap();

		sender.send((vec![], update(400))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());
		wait_for_responses(&responses, 1).await;
		assert_eq!(*dry_runs.lock().unwrap(), 2);
		assert_eq!(responses.lock().unwrap()[0].source.height.height, 400);
		cleanup(&path);
	}

	/// A submitted GET keeps its row until hyperbridge has a response receipt for it.
	#[tokio::test(start_paused = true)]
	async fn a_submitted_get_is_deleted_only_after_the_response_receipt() {
		let path = temp_db_path();
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let request = get();

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
			Duration::ZERO,
		));
		sender.send((vec![request.clone()], update(200))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());
		wait_for_responses(&responses, 1).await;
		assert_eq!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().len(), 1);

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
			Duration::ZERO,
		));
		sender.send((vec![], update(300))).await.unwrap();
		drop(sender);
		assert!(processor.await.unwrap().is_err());

		assert!(responses.lock().unwrap().is_empty());
		assert!(tx_payment.unprofitable_messages(&COPROCESSOR).await.unwrap().is_empty());
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
			Duration::ZERO,
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
			Duration::ZERO,
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
			Duration::ZERO,
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
					default_state(),
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
			Duration::ZERO,
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
				vec![stored_row(
					vec![get()],
					destination_proof(STORED_PROOF.to_vec()),
					default_state(),
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
		let (sender, receiver) = tokio::sync::mpsc::channel(8);
		let processor = tokio::spawn(process_get_request_events(
			receiver,
			source,
			hyperbridge,
			HashMap::new(),
			tx_payment.clone(),
			Duration::ZERO,
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
			Duration::ZERO,
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
					default_state(),
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
			Duration::ZERO,
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
			Duration::ZERO,
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
