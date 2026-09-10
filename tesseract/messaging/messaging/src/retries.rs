use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
	time::Duration,
};

use futures::stream::FuturesOrdered;
use ismp::{
	consensus::StateMachineHeight,
	host::StateMachine,
	messaging::{hash_request, Message, Proof, RequestMessage},
	router::{PostRequest, Request},
};
use primitive_types::H256;
use tesseract_primitives::{config::RelayerConfig, Hasher, IsmpProvider, Query, TxResult};
use tokio_stream::StreamExt;
use transaction_fees::TransactionPayment;

use crate::{
	events::{chunk_size, return_successful_queries, was_delivered},
	record_deliveries, FeeAccSender,
};

/// How often a pass runs when the operator hasn't set `unprofitable_retry_frequency`.
const DEFAULT_RETRY_FREQUENCY: Duration = Duration::from_secs(5 * 60);

/// Everything one destination's retry loop needs, assembled once by the caller.
pub struct RetryContext {
	/// The chain the parked messages were meant for.
	pub dest: Arc<dyn IsmpProvider>,
	/// Where the requests came from, and where their proofs are read from.
	pub hyperbridge: Arc<dyn IsmpProvider>,
	pub client_map: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	pub tx_payment: Arc<TransactionPayment>,
	pub config: RelayerConfig,
	pub coprocessor: StateMachine,
	pub fee_acc_sender: Option<FeeAccSender>,
}

/// Redeliver requests from hyperbridge that never landed.
///
/// Deliveries out of hyperbridge are gated to a whitelisted relayer, so a batch
/// this relayer failed to submit is not going to be picked up by anyone else.
/// The delivery pipelines park those requests in the database and this loop
/// drains them on a timer.
pub async fn retry_undelivered_messages(ctx: RetryContext) -> Result<(), anyhow::Error> {
	let frequency = ctx
		.config
		.unprofitable_retry_frequency
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

	// Everything is proved again against the height the destination is on now.
	// The proof a request was parked with is anchored at a hyperbridge height the
	// destination may never have received, since the consensus update it rode
	// along with is exactly what failed to land.
	let height = StateMachineHeight {
		id: ctx.hyperbridge.state_machine_id(),
		height: ctx.dest.query_latest_height(ctx.hyperbridge.state_machine_id()).await? as u64,
	};

	let (ready, waiting): (Vec<_>, Vec<_>) =
		parked.into_iter().partition(|(message, _)| provable_at(message, height.height));
	if !waiting.is_empty() {
		tracing::trace!(
			target: crate::LOG_TARGET,
			dest = %ctx.dest.name(),
			count = waiting.len(),
			height = height.height,
			"Leaving rows parked until the destination catches up",
		);
	}
	if ready.is_empty() {
		return Ok(());
	}

	// Every row taken here is dropped at the end of the pass; whatever is still
	// worth another attempt is written back as a fresh row.
	let rows = ready.iter().map(|(_, id)| *id).collect::<Vec<_>>();

	// A request can be parked twice, once on its own and once inside a batch
	// that failed later, and a batch carrying the same request twice reverts on
	// the duplicate. Keying by commitment keeps one copy of each.
	let requests = ready
		.into_iter()
		.filter_map(|(message, _)| match message {
			Message::Request(msg) => Some(msg.requests),
			_ => None,
		})
		.flatten()
		.map(|post| (hash_request::<Hasher>(&Request::Post(post.clone())), post))
		.collect::<BTreeMap<H256, PostRequest>>();

	let deliverable = deliverable_requests(&ctx.dest, requests.into_values().collect()).await?;
	if deliverable.is_empty() {
		ctx.tx_payment.delete_unprofitable_messages(rows).await?;
		return Ok(());
	}

	tracing::trace!(
		target: crate::LOG_TARGET,
		dest = %ctx.dest.name(),
		count = deliverable.len(),
		height = height.height,
		"Retrying previously undelivered messages",
	);

	let queries = deliverable
		.iter()
		.map(|post| Query::from(&Request::Post(post.clone())))
		.collect::<Vec<_>>();
	let messages = single_request_messages(ctx, &deliverable, height).await?;
	let profitability = return_successful_queries(
		ctx.dest.clone(),
		messages,
		queries,
		ctx.config.minimum_profit_percentage,
		ctx.coprocessor,
		&ctx.client_map,
		ctx.config.deliver_failed.unwrap_or_default(),
		// No consensus update rides along with a retry, the destination's light
		// client is already at the height these proofs are built against.
		None,
	)
	.await?;

	let profitable = deliverable
		.into_iter()
		.zip(profitability.queries)
		.filter_map(|(post, query)| query.map(|query| (post, query)))
		.collect::<Vec<_>>();

	let mut retriable = profitability.retriable_messages;
	let outgoing = batch_requests(ctx, &profitable, height).await?;

	if !outgoing.is_empty() {
		tracing::info!(
			target: crate::LOG_TARGET,
			"🛰️ Retransmitting ismp messages from {} to {}",
			ctx.hyperbridge.name(),
			ctx.dest.name(),
		);
		match ctx.dest.submit(outgoing.clone(), ctx.coprocessor).await {
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

/// Whether hyperbridge can prove this row for a destination sitting at `height`.
/// A request only enters the mmr at the block it was committed in, so a row
/// built past where the destination's light client has reached cannot be proved
/// for it yet and has to wait.
fn provable_at(message: &Message, height: u64) -> bool {
	match message {
		Message::Request(msg) => msg.proof.height.height <= height,
		// Rows the delivery pipelines never write. Sweeping them up keeps a stale
		// one from sitting in the table for good.
		_ => true,
	}
}

/// Requests that are still worth submitting.
///
/// A request the destination already has a receipt for was delivered by some
/// other submission, and one past its timeout is never accepted again. Both
/// revert on delivery, so neither is carried any further.
async fn deliverable_requests(
	dest: &Arc<dyn IsmpProvider>,
	requests: Vec<PostRequest>,
) -> Result<Vec<PostRequest>, anyhow::Error> {
	let timestamp = dest.query_timestamp().await?;
	let mut deliverable = vec![];

	for chunk in requests.chunks(dest.max_concurrent_queries()) {
		let checked = chunk
			.iter()
			.map(|post| async move {
				let request = Request::Post(post.clone());
				if request.timed_out(timestamp) {
					return Ok::<_, anyhow::Error>(None);
				}

				let receipt = dest.query_request_receipt(hash_request::<Hasher>(&request)).await?;
				Ok((!was_delivered(&receipt)).then(|| post.clone()))
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
	use ismp::consensus::StateMachineId;
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

	fn parked_row(post: PostRequest, proof_height: u64, id: i32) -> (Message, i32) {
		let message = Message::Request(RequestMessage {
			requests: vec![post],
			proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId { state_id: HB, consensus_state_id: *b"BEEF" },
					height: proof_height,
				},
				proof: vec![],
			},
			signer: vec![],
		});
		(message, id)
	}

	/// Hyperbridge cannot prove a request at a height before it was committed, so
	/// a row built past where the destination's light client has reached has to
	/// wait rather than fail the whole pass.
	#[test]
	fn rows_past_the_destination_height_stay_parked() {
		let dest_height = 100;
		let parked = vec![
			parked_row(post(1, 0), 90, 1),
			parked_row(post(2, 0), 100, 2),
			parked_row(post(3, 0), 130, 3),
		];

		let (ready, waiting): (Vec<_>, Vec<_>) =
			parked.into_iter().partition(|(message, _)| provable_at(message, dest_height));

		assert_eq!(ready.iter().map(|(_, id)| *id).collect::<Vec<_>>(), vec![1, 2]);
		assert_eq!(waiting.iter().map(|(_, id)| *id).collect::<Vec<_>>(), vec![3]);
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

		let deliverable = deliverable_requests(&dest, vec![pending.clone(), delivered, expired])
			.await
			.unwrap();

		assert_eq!(deliverable, vec![pending]);
	}
}
