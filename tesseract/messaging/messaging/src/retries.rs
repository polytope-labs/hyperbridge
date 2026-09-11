use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
	time::Duration,
};

use futures::stream::FuturesOrdered;
use ismp::{
	consensus::StateMachineHeight,
	host::StateMachine,
	messaging::{hash_request, ConsensusMessage, Message, Proof, RequestMessage},
	router::{PostRequest, Request},
};
use primitive_types::H256;
use tesseract_primitives::{
	config::RelayerConfig, ConsensusProofSource, Hasher, IsmpProvider, ProofKey, Query, TxResult,
	BEEFY_CONSENSUS_STATE_ID,
};
use tokio_stream::StreamExt;
use transaction_fees::TransactionPayment;

use crate::{
	events::{chunk_size, is_retry_module, return_successful_queries, was_delivered},
	record_deliveries, FeeAccSender,
};

/// How often a pass runs when the operator hasn't set `retry_frequency`.
const DEFAULT_RETRY_FREQUENCY: Duration = Duration::from_secs(5 * 60);

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
/// and this loop drains them on a timer. Deliveries out of hyperbridge are the usual
/// reason to list a module: they are gated to a whitelisted relayer, so a batch
/// this relayer failed to submit is not going to be picked up by anyone else.
pub async fn retry_undelivered_messages(ctx: RetryContext) -> Result<(), anyhow::Error> {
	let frequency = ctx
		.config
		.retry_frequency
		.map_or(DEFAULT_RETRY_FREQUENCY, Duration::from_secs);
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
	let parked = ctx.tx_payment.unprofitable_messages(&dest_state_machine).await?;
	if parked.is_empty() {
		return Ok(());
	}

	// Every row read here is dropped at the end of the pass; whatever is still
	// worth another attempt is written back as a fresh row.
	let rows = parked.iter().map(|(_, id)| *id).collect::<Vec<_>>();

	// A request can be parked more than once, on its own and inside a batch that
	// failed later, and a batch carrying it twice reverts on the duplicate.
	// Keying by commitment keeps one copy, at the lowest height it was seen,
	// which is the one the destination is likeliest to already be past.
	let mut requests: BTreeMap<H256, (PostRequest, u64)> = BTreeMap::new();
	for (message, _) in parked {
		let Message::Request(msg) = message else { continue };
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

	let mut retriable = vec![];
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
					retriable.extend(messages);
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
		retriable.extend(profitability.retriable_messages);

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
				retriable.extend(unsuccessful);
			},
			Err(err) => {
				tracing::error!(
					target: crate::LOG_TARGET,
					dest = %ctx.dest.name(),
					?err,
					"Retry submission failed",
				);
				retriable.extend(outgoing);
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
		ctx.tx_payment
			.store_unprofitable_messages(retriable, dest_state_machine)
			.await?;
	}

	Ok(())
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
}
