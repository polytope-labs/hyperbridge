use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
	time::Duration,
};

use futures::stream::FuturesOrdered;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::{hash_request, ConsensusMessage, Message, Proof, RequestMessage},
	router::{PostRequest, Request},
};
use primitive_types::H256;
use tesseract_primitives::{
	config::RelayerConfig, ConsensusProofSource, Hasher, IsmpProvider, ProofKey, Query,
	SimulationFailure, TxResult, BEEFY_CONSENSUS_STATE_ID,
};
use tokio_stream::StreamExt;
use transaction_fees::{ParkedMessage, RetryState, TransactionPayment};

use crate::{
	events::{chunk_size, is_retry_module, return_successful_queries, was_delivered},
	record_deliveries, FeeAccSender,
};

/// How often a pass runs when the operator hasn't set `retry_frequency`.
const DEFAULT_RETRY_FREQUENCY: Duration = Duration::from_secs(5 * 60);

/// A parked message is retried at `retry_frequency` doubling per attempt, up to this.
const MAX_BACKOFF: Duration = Duration::from_secs(60 * 60);

/// A message parked this many times is dropped even if it has not aged out.
const MAX_ATTEMPTS: u32 = 10;

/// A message parked this long is dropped: the module is not going to accept it.
const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// Everything one destination's retry loop needs, assembled once by the caller.
pub struct RetryContext {
	/// The chain the parked messages were meant for.
	pub dest: Arc<dyn IsmpProvider>,
	/// Where the requests came from, and where their proofs are read from.
	pub hyperbridge: Arc<dyn IsmpProvider>,
	/// Supplies the beefy proof that carries a destination up to the height a
	/// parked request needs.
	pub proof_source: Arc<dyn ConsensusProofSource>,
	pub client_map: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	pub tx_payment: Arc<TransactionPayment>,
	pub config: RelayerConfig,
	pub coprocessor: StateMachine,
	pub fee_acc_sender: Option<FeeAccSender>,
}

/// Redeliver requests to an EVM chain that never landed.
///
/// The outbound fan-out parks requests addressed to the modules the operator
/// listed in `retry_modules` whenever a batch is cancelled or fails to submit,
/// and this loop drains them every `retry_frequency` seconds, five minutes when
/// unset. Listing a module is what brings the loop up in the first place. Deliveries out of
/// hyperbridge are the usual reason to list a module: they are gated to a whitelisted relayer, so a
/// batch this relayer failed to submit is not going to be picked up by anyone else.
pub async fn retry_undelivered_messages(ctx: RetryContext) -> Result<(), anyhow::Error> {
	let frequency = ctx.config.retry_frequency.map_or(DEFAULT_RETRY_FREQUENCY, Duration::from_secs);
	tracing::trace!(
		target: crate::LOG_TARGET,
		dest = %ctx.dest.name(),
		?frequency,
		"Starting message retries background task",
	);

	let mut interval = tokio::time::interval(frequency);
	loop {
		interval.tick().await;
		if let Err(err) = retry_once(&ctx).await {
			tracing::error!(
				target: crate::LOG_TARGET,
				dest = %ctx.dest.name(),
				?err,
				"Retry pass failed",
			);
		}
	}
}

/// One pass over everything parked for this destination.
async fn retry_once(ctx: &RetryContext) -> Result<(), anyhow::Error> {
	let dest_state_machine = ctx.dest.state_machine_id().state_id;
	let frequency = ctx.config.retry_frequency.map_or(DEFAULT_RETRY_FREQUENCY, Duration::from_secs);
	let parked = ctx.tx_payment.unprofitable_messages(&dest_state_machine).await?;
	if parked.is_empty() {
		return Ok(());
	}

	let now = transaction_fees::unix_now();
	// Every row that is retried or dropped here is deleted at the end of the
	// pass; whatever is still worth another attempt is written back fresh.
	let mut rows = vec![];
	let mut due_messages = vec![];
	// Re-parking needs the bookkeeping of the row a request came from, but
	// `deliverable_requests` only deals in requests, so it is looked up again by
	// commitment at the end of the pass.
	let mut states: HashMap<H256, RetryState> = HashMap::new();

	for ParkedMessage { message, state, id } in parked {
		// Rows written before the backoff columns existed have no age.
		let created_at = if state.created_at == 0 { now } else { state.created_at };
		let state = RetryState { created_at, ..state };

		match retry_is_due(state, now) {
			RetryDue::Expired => {
				if let Some(id) = id {
					rows.push(id);
				}
				let commitments = match &message {
					Message::Request(request) => request
						.requests
						.iter()
						.map(|post| {
							hex::encode(hash_request::<Hasher>(&Request::Post(post.clone())))
						})
						.collect::<Vec<_>>(),
					_ => vec![],
				};
				tracing::warn!(
					target: crate::LOG_TARGET,
					dest = %ctx.dest.name(),
					age = now.saturating_sub(created_at),
					attempts = state.attempts,
					?commitments,
					"dropping parked message",
				);
			},
			RetryDue::NotYet => {},
			RetryDue::Due => {
				if let Some(id) = id {
					rows.push(id);
				}
				let Message::Request(request) = message else { continue };
				for post in &request.requests {
					let commitment = hash_request::<Hasher>(&Request::Post(post.clone()));
					states.entry(commitment).or_insert(state);
				}
				due_messages.push(request);
			},
		}
	}

	if due_messages.is_empty() {
		if !rows.is_empty() {
			ctx.tx_payment.delete_unprofitable_messages(rows).await?;
		}
		return Ok(())
	}

	// A request can be parked more than once, on its own and inside a batch that
	// failed later, and a batch carrying it twice reverts on the duplicate.
	// Keying by commitment keeps one copy, at the lowest height it was seen,
	// which is the one the destination is likeliest to already be past.
	let mut requests: BTreeMap<H256, (PostRequest, u64)> = BTreeMap::new();
	for msg in due_messages {
		let parked_at = msg.proof.height.height;
		// Only requests addressed to a listed module are retried. Rows left by an
		// older build, or by a config that has since dropped a module, are swept up
		// here and not carried any further.
		for post in msg.requests {
			if !is_retry_module(&ctx.config, &post.to) {
				continue;
			}
			let commitment = hash_request::<Hasher>(&Request::Post(post.clone()));
			requests
				.entry(commitment)
				.and_modify(|(_, height)| *height = (*height).min(parked_at))
				.or_insert((post, parked_at));
		}
	}

	let deliverable = deliverable_requests(&ctx.dest, requests.into_values().collect()).await?;
	if deliverable.is_empty() {
		ctx.tx_payment.delete_unprofitable_messages(rows).await?;
		return Ok(());
	}

	let dest_height =
		ctx.dest.query_latest_height(ctx.hyperbridge.state_machine_id()).await? as u64;

	// Each entry says whether this re-park was a failed delivery attempt, which
	// is what counts toward the attempt and age caps.
	let mut retriable: Vec<(Message, bool)> = vec![];
	for (target, posts) in group_by_proof_height(deliverable, dest_height) {
		let height = StateMachineHeight { id: ctx.hyperbridge.state_machine_id(), height: target };
		let messages = single_request_messages(ctx, &posts, height).await?;

		// A height the destination has not reached yet rides with the beefy proof
		// that takes it there, so the batch verifies now instead of waiting on the
		// next consensus update. Without that proof there is nothing to submit
		// against, so the rows go back in the table for the next pass.
		let prelude = match target > dest_height {
			true => match ctx.proof_source.fetch(ProofKey::Messaging(target)).await {
				Ok(consensus_proof) => Some(Message::Consensus(ConsensusMessage {
					consensus_proof,
					consensus_state_id: BEEFY_CONSENSUS_STATE_ID,
					signer: ctx.dest.address(),
				})),
				Err(err) => {
					tracing::warn!(
						target: crate::LOG_TARGET,
						dest = %ctx.dest.name(),
						height = target,
						?err,
						"No beefy proof for the parked height, leaving the rows for the next pass",
					);
					retriable.extend(messages.into_iter().map(|message| (message, false)));
					continue;
				},
			},
			false => None,
		};

		tracing::trace!(
			target: crate::LOG_TARGET,
			dest = %ctx.dest.name(),
			count = posts.len(),
			height = target,
			consensus = prelude.is_some(),
			"Retrying previously undelivered messages",
		);

		let queries = posts
			.iter()
			.map(|post| Query::from(&Request::Post(post.clone())))
			.collect::<Vec<_>>();
		let profitability = return_successful_queries(
			ctx.dest.clone(),
			messages,
			queries,
			ctx.config.minimum_profit_percentage,
			ctx.coprocessor,
			&ctx.client_map,
			ctx.config.deliver_failed.unwrap_or_default(),
			prelude.clone(),
		)
		.await?;
		retriable.extend(profitability.retriable_messages.into_iter().map(|retriable| {
			let delivery_failed =
				matches!(retriable.failure, Some(SimulationFailure::ModuleReverted { .. }));
			(retriable.message, delivery_failed)
		}));

		let profitable = posts
			.into_iter()
			.zip(profitability.queries)
			.filter_map(|(post, query)| query.map(|query| (post, query)))
			.collect::<Vec<_>>();
		let outgoing = batch_requests(ctx, &profitable, height).await?;
		if outgoing.is_empty() {
			continue;
		}

		tracing::info!(
			target: crate::LOG_TARGET,
			"🛰️ Retransmitting ismp messages from {} to {}",
			ctx.hyperbridge.name(),
			ctx.dest.name(),
		);
		let batch = prelude.into_iter().chain(outgoing.iter().cloned()).collect::<Vec<_>>();
		match ctx.dest.submit(batch, ctx.coprocessor).await {
			Ok(TxResult { receipts, unsuccessful, new_epochs: _ }) => {
				record_deliveries(&ctx.tx_payment, &ctx.fee_acc_sender, receipts, ctx.coprocessor)
					.await;
				retriable.extend(unsuccessful.into_iter().map(|message| (message, true)));
			},
			Err(err) => {
				tracing::error!(
					target: crate::LOG_TARGET,
					dest = %ctx.dest.name(),
					?err,
					"Retry submission failed",
				);
				retriable.extend(outgoing.into_iter().map(|message| (message, false)));
			},
		}
	}

	ctx.tx_payment.delete_unprofitable_messages(rows).await?;
	if !retriable.is_empty() {
		tracing::trace!(
			target: crate::LOG_TARGET,
			dest = %ctx.dest.name(),
			count = retriable.len(),
			"Parking messages for the next retry",
		);
		let parked = retriable
			.into_iter()
			.filter_map(|(message, delivery_failed)| {
				let Message::Request(request) = message else { return None };
				let state = state_for_requests(&request, &states, now);
				Some(ParkedMessage {
					message: Message::Request(request),
					state: next_retry_state(state, now, frequency, delivery_failed),
					id: None,
				})
			})
			.collect::<Vec<_>>();
		ctx.tx_payment.store_unprofitable_messages(parked, dest_state_machine).await?;
	}

	Ok(())
}

/// Fold the bookkeeping of every row a message came from into one state.
///
/// A multi-request message can carry requests parked on different passes, so
/// the most-tried request decides the attempt count and the oldest decides the
/// age clock. Requests with no known row fall back to a fresh state.
fn state_for_requests(
	request: &RequestMessage,
	states: &HashMap<H256, RetryState>,
	now: u64,
) -> RetryState {
	let mut attempts = 0;
	let mut created_at: Option<u64> = None;
	for post in &request.requests {
		let commitment = hash_request::<Hasher>(&Request::Post(post.clone()));
		let Some(state) = states.get(&commitment) else { continue };
		attempts = attempts.max(state.attempts);
		created_at =
			Some(created_at.map_or(state.created_at, |oldest| oldest.min(state.created_at)));
	}

	RetryState { attempts, created_at: created_at.unwrap_or(now), next_attempt_at: 0 }
}

/// What the retry loop should do with a parked row right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetryDue {
	/// Too old or tried too often; the module is not going to accept it.
	Expired,
	/// Backoff has not elapsed; leave the row alone.
	NotYet,
	/// Worth another attempt.
	Due,
}

fn retry_is_due(state: RetryState, now: u64) -> RetryDue {
	// A row that never had a failed delivery attempt is retried forever; the
	// caps only apply to deliveries the module actually rejected.
	if state.attempts >= MAX_ATTEMPTS ||
		(state.attempts > 0 && now.saturating_sub(state.created_at) > MAX_AGE.as_secs())
	{
		RetryDue::Expired
	} else if state.next_attempt_at > now {
		RetryDue::NotYet
	} else {
		RetryDue::Due
	}
}

/// Bookkeeping for a message that just failed another attempt. The delay
/// starts at `retry_frequency` and doubles per failed delivery up to
/// [`MAX_BACKOFF`]. Only a failed delivery counts toward the age clock.
fn next_retry_state(
	state: RetryState,
	now: u64,
	frequency: Duration,
	delivery_failed: bool,
) -> RetryState {
	let attempts = if delivery_failed { state.attempts.saturating_add(1) } else { state.attempts };
	let created_at = if delivery_failed && state.attempts == 0 { now } else { state.created_at };
	let doublings = attempts.saturating_sub(1).min(31);
	let delay = frequency.saturating_mul(1u32 << doublings).min(MAX_BACKOFF);
	RetryState { attempts, created_at, next_attempt_at: now.saturating_add(delay.as_secs()) }
}

/// Bucket each request by the height its proof should be built at: the height the
/// destination is already on once it has gone past where the request was parked,
/// and the parked height otherwise.
fn group_by_proof_height(
	deliverable: Vec<(PostRequest, u64)>,
	dest_height: u64,
) -> BTreeMap<u64, Vec<PostRequest>> {
	deliverable.into_iter().fold(BTreeMap::new(), |mut groups, (post, parked_at)| {
		groups.entry(parked_at.max(dest_height)).or_default().push(post);
		groups
	})
}

/// Requests that are still worth submitting.
///
/// A request the destination already has a receipt for was delivered by some
/// other submission, and one past its timeout is never accepted again. Both
/// revert on delivery, so neither is carried any further.
async fn deliverable_requests(
	dest: &Arc<dyn IsmpProvider>,
	requests: Vec<(PostRequest, u64)>,
) -> Result<Vec<(PostRequest, u64)>, anyhow::Error> {
	let timestamp = dest.query_timestamp().await?;
	let mut deliverable = vec![];

	for chunk in requests.chunks(dest.max_concurrent_queries()) {
		let checked = chunk
			.iter()
			.map(|(post, height)| async move {
				let request = Request::Post(post.clone());
				if request.timed_out(timestamp) {
					return Ok::<_, anyhow::Error>(None);
				}

				let receipt = dest.query_request_receipt(hash_request::<Hasher>(&request)).await?;
				Ok((!was_delivered(&receipt)).then(|| (post.clone(), *height)))
			})
			.collect::<FuturesOrdered<_>>()
			.collect::<Result<Vec<_>, _>>()
			.await?;

		deliverable.extend(checked.into_iter().flatten());
	}

	Ok(deliverable)
}

/// One message per request, each with its own proof, which is the shape gas
/// estimation and the profitability check work on.
async fn single_request_messages(
	ctx: &RetryContext,
	requests: &[PostRequest],
	height: StateMachineHeight,
) -> Result<Vec<Message>, anyhow::Error> {
	let dest_state_machine = ctx.dest.state_machine_id().state_id;
	let mut messages = vec![];

	for chunk in requests.chunks(ctx.hyperbridge.max_concurrent_queries()) {
		let built = chunk
			.iter()
			.map(|post| async move {
				let query = Query::from(&Request::Post(post.clone()));
				let proof = ctx
					.hyperbridge
					.query_requests_proof(height.height, vec![query], dest_state_machine)
					.await?;

				Ok::<_, anyhow::Error>(Message::Request(RequestMessage {
					requests: vec![post.clone()],
					proof: Proof { height, proof },
					signer: ctx.dest.address(),
				}))
			})
			.collect::<FuturesOrdered<_>>()
			.collect::<Result<Vec<_>, _>>()
			.await?;

		messages.extend(built);
	}

	Ok(messages)
}

/// Regroup the requests that made it through into batches the destination can
/// take in one call, each proved once for the whole batch.
async fn batch_requests(
	ctx: &RetryContext,
	profitable: &[(PostRequest, Query)],
	height: StateMachineHeight,
) -> Result<Vec<Message>, anyhow::Error> {
	let dest_state_machine = ctx.dest.state_machine_id().state_id;
	let mut messages = vec![];

	for chunk in profitable.chunks(chunk_size(dest_state_machine)) {
		let (requests, queries): (Vec<_>, Vec<_>) = chunk.iter().cloned().unzip();
		let proof = ctx
			.hyperbridge
			.query_requests_proof(height.height, queries, dest_state_machine)
			.await?;

		messages.push(Message::Request(RequestMessage {
			requests,
			proof: Proof { height, proof },
			signer: ctx.dest.address(),
		}));
	}

	Ok(messages)
}

#[cfg(test)]
mod tests {
	use super::*;
	use tesseract_primitives::mocks::MockHost;

	const HB: StateMachine = StateMachine::Kusama(4009);
	const DEST: StateMachine = StateMachine::Evm(1);

	fn post(nonce: u64, timeout_timestamp: u64) -> PostRequest {
		PostRequest {
			source: HB,
			dest: DEST,
			nonce,
			from: vec![1],
			to: vec![2],
			timeout_timestamp,
			body: vec![],
		}
	}

	fn commitment(post: &PostRequest) -> H256 {
		hash_request::<Hasher>(&Request::Post(post.clone()))
	}

	/// Requests the destination has already gone past are proved at its own height,
	/// so they need no consensus proof. One parked beyond it keeps its own height
	/// and rides with the proof that takes the destination there.
	#[test]
	fn requests_group_by_the_height_they_can_be_proved_at() {
		let dest_height = 100;
		let deliverable =
			vec![(post(1, 0), 90), (post(2, 0), 100), (post(3, 0), 130), (post(4, 0), 130)];

		let groups = group_by_proof_height(deliverable, dest_height);

		let nonces = |height: u64| {
			groups
				.get(&height)
				.map(|posts| posts.iter().map(|p| p.nonce).collect::<Vec<_>>())
		};
		assert_eq!(nonces(100), Some(vec![1, 2]), "proved at the height the destination is on");
		assert_eq!(nonces(130), Some(vec![3, 4]), "proved at the height they were parked at");
		assert_eq!(groups.len(), 2);
	}

	#[test]
	fn first_counting_repark_uses_one_frequency() {
		let frequency = Duration::from_secs(300);
		let parked = RetryState { attempts: 0, created_at: 1_000, next_attempt_at: 0 };

		let next = next_retry_state(parked, 2_000, frequency, true);

		assert_eq!(next.attempts, 1);
		assert_eq!(next.created_at, 2_000, "the age clock starts at the first failed delivery");
		assert_eq!(next.next_attempt_at, 2_000 + 300);
	}

	#[test]
	fn a_non_counting_repark_keeps_the_count_and_age() {
		let frequency = Duration::from_secs(300);
		let parked = RetryState { attempts: 2, created_at: 1_000, next_attempt_at: 0 };

		let next = next_retry_state(parked, 2_000, frequency, false);

		assert_eq!(next.attempts, 2);
		assert_eq!(next.created_at, 1_000);
		assert_eq!(next.next_attempt_at, 2_000 + 600);
	}

	#[test]
	fn backoff_is_capped() {
		let frequency = Duration::from_secs(300);
		let parked = RetryState { attempts: 9, created_at: 1_000, next_attempt_at: 0 };

		let next = next_retry_state(parked, 2_000, frequency, true);

		assert_eq!(next.attempts, 10);
		assert_eq!(next.next_attempt_at, 2_000 + MAX_BACKOFF.as_secs());
	}

	#[test]
	fn too_many_attempts_expires_the_row() {
		let state = RetryState { attempts: MAX_ATTEMPTS, created_at: 1_000, next_attempt_at: 0 };

		assert_eq!(retry_is_due(state, 2_000), RetryDue::Expired);
	}

	#[test]
	fn an_old_row_without_a_failed_delivery_is_still_due() {
		let now = 1_000_000;
		let state =
			RetryState { attempts: 0, created_at: now - MAX_AGE.as_secs() - 1, next_attempt_at: 0 };

		assert_eq!(retry_is_due(state, now), RetryDue::Due);
	}

	#[test]
	fn an_old_row_with_a_failed_delivery_expires() {
		let now = 1_000_000;
		let state =
			RetryState { attempts: 1, created_at: now - MAX_AGE.as_secs() - 1, next_attempt_at: 0 };

		assert_eq!(retry_is_due(state, now), RetryDue::Expired);
	}

	#[test]
	fn a_future_attempt_is_not_due() {
		let state = RetryState { attempts: 1, created_at: 1_000, next_attempt_at: 2_001 };

		assert_eq!(retry_is_due(state, 2_000), RetryDue::NotYet);
	}

	#[tokio::test]
	async fn keeps_only_requests_the_destination_can_still_take() {
		let pending = post(1, 0);
		let delivered = post(2, 0);
		let expired = post(3, 100);

		let dest: Arc<dyn IsmpProvider> = Arc::new(
			MockHost::new((), 0, DEST)
				.with_request_receipt(commitment(&delivered), vec![0xab; 20])
				.with_timestamp(Duration::from_secs(200)),
		);

		let deliverable = deliverable_requests(
			&dest,
			vec![(pending.clone(), 10), (delivered, 10), (expired, 10)],
		)
		.await
		.unwrap();

		assert_eq!(deliverable, vec![(pending, 10)]);
	}

	fn temp_db_path(test_name: &str) -> String {
		let pid = std::process::id();
		let nanos = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|elapsed| elapsed.as_nanos())
			.unwrap_or(0);
		let path = format!("/tmp/tesseract-retries-test-{test_name}-{pid}-{nanos}.db");
		let _ = std::fs::remove_file(&path);
		path
	}

	fn parked(nonce: u64, state: RetryState) -> ParkedMessage {
		ParkedMessage {
			message: Message::Request(RequestMessage {
				requests: vec![post(nonce, 0)],
				proof: Proof {
					height: StateMachineHeight {
						id: StateMachineId {
							state_id: HB,
							consensus_state_id: BEEFY_CONSENSUS_STATE_ID,
						},
						height: 1,
					},
					proof: vec![],
				},
				signer: vec![],
			}),
			state,
			id: None,
		}
	}

	struct NoProof;

	#[async_trait::async_trait]
	impl ConsensusProofSource for NoProof {
		async fn fetch(&self, _key: ProofKey) -> Result<Vec<u8>, anyhow::Error> {
			Err(anyhow::anyhow!("no proof in this test"))
		}
	}

	#[tokio::test]
	async fn expired_rows_are_dropped_and_future_rows_are_left_alone() {
		let path = temp_db_path("expiry");
		let tx_payment = Arc::new(TransactionPayment::initialize(&path).await.unwrap());
		let now = transaction_fees::unix_now();

		let expired = parked(
			1,
			RetryState { attempts: MAX_ATTEMPTS, created_at: now - 10, next_attempt_at: 0 },
		);
		let not_yet =
			parked(2, RetryState { attempts: 0, created_at: now, next_attempt_at: now + 10_000 });
		tx_payment
			.store_unprofitable_messages(vec![expired, not_yet], DEST)
			.await
			.unwrap();

		let ctx = RetryContext {
			dest: Arc::new(MockHost::new((), 0, DEST)),
			hyperbridge: Arc::new(MockHost::new((), 0, HB)),
			proof_source: Arc::new(NoProof),
			client_map: HashMap::new(),
			tx_payment: tx_payment.clone(),
			config: RelayerConfig::default(),
			coprocessor: HB,
			fee_acc_sender: None,
		};

		retry_once(&ctx).await.unwrap();

		let remaining = tx_payment.unprofitable_messages(&DEST).await.unwrap();
		assert_eq!(remaining.len(), 1);
		assert_eq!(remaining[0].state.attempts, 0);

		let _ = std::fs::remove_file(&path);
		let _ = std::fs::remove_file(format!("{path}-journal"));
	}
}
