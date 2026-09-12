use std::{sync::Arc, time::Duration};

use alloy::providers::Provider;
use anyhow::{anyhow, Error};
use futures::FutureExt;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::{Event, StateMachineUpdated},
	host::StateMachine,
};
use primitive_types::H256;
use tesseract_primitives::{BoxStream, ByzantineHandler, IsmpProvider};

use crate::{AlloyProvider, EvmClient};

/// Supermajority quorum threshold over `total` providers, computed as the
/// classic BFT bound `⌊2/3·N⌋ + 1` so the threshold scales with the
/// configured RPC fan-out instead of being a hard-coded floor.
fn quorum_threshold(total: usize) -> usize {
	total * 2 / 3 + 1
}

/// Each per-provider block fetch is retried up to this many times on transport
/// errors before being recorded as a non-signal. Transport errors do not by
/// themselves justify a veto.
const MAX_TRANSPORT_RETRIES: usize = 3;

/// Backoff between retries.
const RETRY_BACKOFF: Duration = Duration::from_millis(250);

/// The header fields that back a [`StateCommitment`](ismp::consensus::StateCommitment): both are
/// compared across providers and then against the commitment hyperbridge recorded, so a forged
/// timestamp is vetoed just like a forged state root.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct HeaderFields {
	/// Root hash of the global state trie.
	state_root: H256,
	/// Block timestamp in seconds, the same unit as `StateCommitment::timestamp`.
	timestamp: u64,
}

/// Outcome of fetching the L2 block for a single provider, after retries.
enum FetchOutcome {
	/// Provider returned a block header at the queried height. We carry the
	/// commitment fields so the caller can compare across providers.
	Found(HeaderFields),
	/// Provider definitively reports there is no block at this height.
	Missing,
	/// Provider failed with transport errors on every attempt. Treated as a
	/// non-signal.
	Errored,
}

/// Fetch the block at `height` from a single provider, retrying transport
/// errors up to `MAX_TRANSPORT_RETRIES` before giving up. `Ok(None)` (block
/// genuinely not yet on this node) is returned immediately as `Missing` —
/// it's a real signal, not a transport failure.
async fn fetch_with_retry(provider: &AlloyProvider, height: u64) -> FetchOutcome {
	for attempt in 1..=MAX_TRANSPORT_RETRIES {
		match provider.get_block(height.into()).await {
			Ok(Some(block)) => {
				return FetchOutcome::Found(HeaderFields {
					state_root: H256(block.header.state_root.0),
					timestamp: block.header.timestamp,
				})
			},
			Ok(None) => return FetchOutcome::Missing,
			Err(e) => {
				log::warn!(
					target: crate::LOG_TARGET,
					"byzantine fetch attempt {attempt}/{MAX_TRANSPORT_RETRIES} for height {height} failed: {e:?}",
				);
				if attempt < MAX_TRANSPORT_RETRIES {
					tokio::time::sleep(RETRY_BACKOFF).await;
				}
			},
		}
	}
	FetchOutcome::Errored
}

#[async_trait::async_trait]
impl ByzantineHandler for EvmClient {
	async fn check_for_byzantine_attack(
		&self,
		_coprocessor: StateMachine,
		counterparty: Arc<dyn IsmpProvider>,
		event: StateMachineUpdated,
	) -> Result<(), anyhow::Error> {
		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: self.state_machine,
				consensus_state_id: self.consensus_state_id,
			},
			height: event.latest_height,
		};

		let counterparty_state_id = counterparty.state_machine_id().state_id;

		// Multi-RPC quorum is mandatory. With no byzantine providers there's
		// nothing to fan out to; bail before constructing any futures.
		if self.byzantine_providers.is_empty() {
			return Ok(());
		}

		// Transport errors after retries don't count toward the quorum, no
		// veto on RPC failure.
		let outcomes = futures::future::join_all(
			self.byzantine_providers
				.iter()
				.map(|p| fetch_with_retry(p.as_ref(), event.latest_height)),
		)
		.await;

		let quorum = quorum_threshold(self.byzantine_providers.len());
		let mut headers: Vec<HeaderFields> = Vec::with_capacity(outcomes.len());
		let mut missing = 0usize;
		for outcome in outcomes {
			match outcome {
				FetchOutcome::Found(fields) => headers.push(fields),
				FetchOutcome::Missing => missing += 1,
				// Transport errors after retries don't drive any decision —
				// they're silent non-signals.
				FetchOutcome::Errored => {},
			}
		}

		// Quorum of providers report the height doesn't exist on the L2 yet
		// hyperbridge holds a commitment for it: fraud, veto.
		if missing >= quorum {
			log::info!(
				target: crate::LOG_TARGET,
				"Vetoing State Machine Update for {} on {}: {missing} providers report no block at height {}",
				self.state_machine,
				counterparty_state_id,
				event.latest_height,
			);
			counterparty.veto_state_commitment(height).await?;
			return Ok(());
		}

		// Below quorum on positive responses (block headers), no signal worth
		// acting on. Either too few providers responded, or the remaining
		// responses split between Found and Missing without either side
		// reaching quorum.
		if headers.len() < quorum {
			log::warn!(
				target: crate::LOG_TARGET,
				"insufficient quorum for {} on {} at height {}: {} headers, {missing} missing (threshold {quorum}). Abstaining.",
				self.state_machine,
				counterparty_state_id,
				event.latest_height,
				headers.len(),
			);
			return Ok(());
		}

		let first = headers[0];
		// State root *and* timestamp must be unanimous: providers serving the same block can only
		// disagree on either field if one of them is lying.
		let unanimous = headers.iter().all(|fields| *fields == first);
		if !unanimous {
			log::info!(
				target: crate::LOG_TARGET,
				"Vetoing State Machine Update for {} on {}: providers disagree at height {}: {headers:?}",
				self.state_machine,
				counterparty_state_id,
				event.latest_height,
			);
			counterparty.veto_state_commitment(height).await?;
			return Ok(());
		}

		let recorded = counterparty.query_state_machine_commitment(height).await?;
		let root_mismatch = recorded.state_root.0 != first.state_root.0;
		// A commitment carrying the right state root but a forged timestamp is just as fraudulent:
		// timestamps gate request/response timeouts, so a skewed one either strands messages or
		// expires them early.
		let timestamp_mismatch = recorded.timestamp != first.timestamp;
		if root_mismatch || timestamp_mismatch {
			log::info!(
				target: crate::LOG_TARGET,
				"Vetoing State Machine Update for {} on {} at height {}: recorded (state root {:?}, timestamp {}) disagrees with quorum (state root {:?}, timestamp {})",
				self.state_machine,
				counterparty_state_id,
				event.latest_height,
				recorded.state_root,
				recorded.timestamp,
				first.state_root,
				first.timestamp,
			);
			counterparty.veto_state_commitment(height).await?;
		}

		Ok(())
	}

	async fn state_machine_updates(
		&self,
		_counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<Vec<StateMachineUpdated>>, Error> {
		use futures::StreamExt;
		let (tx, recv) = tokio::sync::broadcast::channel(512);

		let initial_height = self.client.get_block_number().await?;
		let client = self.clone();
		let poll_interval = 5;
		tokio::spawn(async move {
				let mut latest_height = initial_height;
				let state_machine = client.state_machine;
				loop {
					tokio::time::sleep(Duration::from_secs(poll_interval)).await;
					// wait for an update with a greater height
					let block_number = match client.client.get_block_number().await {
						Ok(number) => number,
						Err(err) => {
							if let Err(err) = tx
								.send(Err(anyhow!(
									"Error fetching latest block height on {state_machine:?} {err:?}"
								).into()))
							{
								log::error!(target: crate::LOG_TARGET, "Failed to send message over channel on {state_machine:?} \n {err:?}");
								return
							}
							continue;
						},
					};

					if block_number <= latest_height {
						continue;
					}

					let event = StateMachineUpdated {
						state_machine_id: client.state_machine_id(),
						latest_height: block_number,
					};

					let events = match client.query_ismp_events(latest_height, event).await {
						Ok(events) => events,
						Err(err) => {
							if let Err(err) = tx
								.send(Err(anyhow!(
									"Error encountered while querying ismp events {err:?}"
								).into()))
							{
								log::error!(target: crate::LOG_TARGET, "Failed to send message over channel on {state_machine:?} \n {err:?}");
								return
							}
							latest_height = block_number;
							continue;
						},
					};

					let events = events
						.into_iter()
						.filter_map(|ev| match ev {
							Event::StateMachineUpdated(update) => Some(update),
							_ => None,
						}).collect::<Vec<_>>();

					if !events.is_empty() {
						if let Err(err) = tx
									.send(Ok(events))
								{
									log::error!(target: crate::LOG_TARGET, "Failed to send message over channel on {state_machine:?} \n {err:?}");
									return
								}
					}
					latest_height = block_number;
				}
			}.boxed());

		let stream = tokio_stream::wrappers::BroadcastStream::new(recv).filter_map(|res| async {
			match res {
				Ok(res) => Some(res),
				Err(err) => Some(Err(anyhow!("{err:?}").into())),
			}
		});

		Ok(Box::pin(stream))
	}
}
