// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0.

//! `IsmpHost` implementation — the consensus loop proper.

use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use codec::Decode;
use polkadot_sdk::sp_runtime::{
	generic::Header as SpHeader,
	traits::{BlakeTwo256, Header as HeaderT},
};
use subxt::{config::HashFor, utils::H256};
use tokio::time::MissedTickBehavior;

use codec::Encode;
use ismp::{
	events::Event,
	messaging::{ConsensusMessage, CreateConsensusState, Message},
};
use ismp_parachain::consensus::{parachain_header_storage_key, ParachainConsensusProof};
use tesseract_primitives::{IsmpHost, IsmpProvider, StateMachineUpdated, StorageKey};

use crate::{ParachainHost, CONSENSUS_UPDATE_FREQUENCY};

#[async_trait::async_trait]
impl<R> IsmpHost for ParachainHost<R>
where
	R: subxt::Config + Send + Sync + Clone,
	R::Header: Send + Sync,
	HashFor<R>: From<H256>,
{
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let self_provider = self.provider();

		// --- initial cursors ---------------------------------------------------
		// 1. Last known relay height tracked by the counterparty — read from its
		//    `IsmpParachain::KnownRelayHeights` storage value at its latest *finalized* block (we
		//    anchor every read at the finalized head so we don't chase a height that's still
		//    reorg-eligible on the counterparty). The bounded set iterates ascending, so the last
		//    entry is the most recent relay block the counterparty has a state root for.
		let counterparty_finalized = counterparty.query_finalized_height().await?;
		let mut last_relay_height =
			query_latest_known_relay_height(&*counterparty, counterparty_finalized)
				.await?
				.ok_or_else(|| anyhow!("counterparty has no KnownRelayHeights entries yet"))?;

		// 2. Last finalized height on self — seed from self's `IsmpProvider`.
		let mut last_self_height = self_provider.query_finalized_height().await?;

		tracing::info!(
			target: crate::LOG_TARGET,
			host = %self.state_machine,
			counterparty = %counterparty.name(),
			%last_relay_height,
			%last_self_height,
			"starting parachain consensus loop",
		);

		// --- loop --------------------------------------------------------------
		let mut interval = tokio::time::interval(Duration::from_secs(CONSENSUS_UPDATE_FREQUENCY));
		// Counterparty RPC blips shouldn't pile up ticks.
		interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

		loop {
			interval.tick().await;

			let step = self.tick(&*counterparty, last_relay_height, last_self_height).await;
			match step {
				Ok(None) => continue,
				Ok(Some(TickOutcome { new_relay_height, new_self_height, message })) => {
					last_relay_height = new_relay_height;
					last_self_height = new_self_height;
					if let Some(message) = message {
						tracing::info!(
							target: "tesseract",
							host = %self.state_machine,
							counterparty = %counterparty.name(),
							self_height = last_self_height,
							relay_height = last_relay_height,
							"🛰️ Submitting parachain consensus proof",
						);
						if let Err(err) = counterparty
							.submit(
								vec![Message::Consensus(message)],
								counterparty.state_machine_id().state_id,
							)
							.await
						{
							tracing::error!(
								target: "tesseract",
								?err,
								dest = %counterparty.name(),
								"failed to submit consensus message",
							);
						}
					}
				},
				Err(err) => {
					tracing::error!(
						target: "tesseract",
						?err,
						"consensus tick failed; will retry on next interval",
					);
				},
			}
		}
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		// Parachain consensus is stateless wrt its own `consensus_state` blob —
		// verification resolves relay-state roots via the host's own
		// `RelayChainOracle`, not from the consensus state. Returning `None`
		// lets the consolidated relayer skip the one-shot `log-consensus-state`
		// path; concrete deployments typically seed the counterparty's consensus
		// state via governance instead.
		Ok(None)
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}

/// Outcome of one consensus tick: updated cursors, plus an optional consensus
/// message to submit to the counterparty.
struct TickOutcome {
	new_relay_height: u32,
	new_self_height: u64,
	message: Option<ConsensusMessage>,
}

impl<R> ParachainHost<R>
where
	R: subxt::Config + Send + Sync + Clone,
	HashFor<R>: From<H256>,
{
	/// Executes one iteration of the consensus loop: fetch the counterparty's
	/// latest known relay height at its *finalized* head, resolve it to a
	/// parachain head, and (if that head surfaces new ISMP requests on self)
	/// build a consensus proof.
	async fn tick(
		&self,
		counterparty: &dyn IsmpProvider,
		last_relay_height: u32,
		last_self_height: u64,
	) -> Result<Option<TickOutcome>, anyhow::Error> {
		// 1. Latest relay height the counterparty has a state root for, read at the counterparty's
		//    latest finalized block.
		let counterparty_finalized = counterparty.query_finalized_height().await?;
		let latest_relay_height =
			match query_latest_known_relay_height(counterparty, counterparty_finalized).await? {
				Some(h) => h,
				None => {
					tracing::trace!(
						target: crate::LOG_TARGET,
						"counterparty has no KnownRelayHeights entries; waiting"
					);
					return Ok(None);
				},
			};
		if latest_relay_height <= last_relay_height {
			return Ok(None);
		}

		// 2. Fetch the relay block hash + raw parachain head from `Paras::Heads`.
		let relay_block_hash = self
			.relay_rpc
			.chain_get_block_hash(Some((latest_relay_height as u64).into()))
			.await?
			.ok_or_else(|| {
				anyhow!("relay chain has no block hash for height {latest_relay_height}")
			})?;

		let head_key = parachain_header_storage_key(self.para_id());
		let raw_head = self
			.relay_client
			.storage()
			.at(relay_block_hash)
			.fetch_raw(head_key.0.clone())
			.await?
			.ok_or_else(|| {
				anyhow!(
					"Paras::Heads[{}] missing on relay chain at height {latest_relay_height}",
					self.para_id(),
				)
			})?;

		// `Paras::Heads` stores the header length-prefix-encoded inside a
		// `Vec<u8>` — mirrors the decode pair in the inherent provider.
		let intermediate = Vec::<u8>::decode(&mut &raw_head[..])?;
		let header = SpHeader::<u32, BlakeTwo256>::decode(&mut &intermediate[..])?;
		let self_height = *header.number() as u64;

		if self_height <= last_self_height {
			// Counterparty moved its relay pointer forward, but self's para head
			// at that relay block isn't newer than what we already proved. No
			// new self-side events — bump the relay cursor only.
			return Ok(Some(TickOutcome {
				new_relay_height: latest_relay_height,
				new_self_height: last_self_height,
				message: None,
			}));
		}

		// 3. Query ISMP events on self for (last_self_height, self_height].
		let self_provider = self.provider();
		let synth = StateMachineUpdated {
			state_machine_id: self_provider.state_machine_id(),
			latest_height: self_height,
		};
		let events = self_provider.query_ismp_events(last_self_height, synth).await?;

		if !has_relay_worthy_events(&events) {
			tracing::trace!(
				target: crate::LOG_TARGET,
				host = %self.state_machine,
				from = last_self_height + 1,
				to = self_height,
				events = events.len(),
				"no ISMP requests in range; skipping proof",
			);
			return Ok(Some(TickOutcome {
				new_relay_height: latest_relay_height,
				new_self_height: self_height,
				message: None,
			}));
		}

		// 4. Build the state-proof for `Paras::Heads[self_para_id]` against the relay block — same
		//    shape the parachain inherent provider produces.
		let read_proof = self
			.relay_rpc
			.state_get_read_proof(vec![&head_key.0[..]], Some(relay_block_hash))
			.await?;
		let storage_proof: Vec<Vec<u8>> = read_proof.proof.into_iter().map(|b| b.0).collect();

		let consensus_proof =
			ParachainConsensusProof { relay_height: latest_relay_height, storage_proof };
		let message = ConsensusMessage {
			consensus_state_id: self.consensus_state_id,
			consensus_proof: consensus_proof.encode(),
			signer: H256::random().0.to_vec(),
		};

		Ok(Some(TickOutcome {
			new_relay_height: latest_relay_height,
			new_self_height: self_height,
			message: Some(message),
		}))
	}
}

/// Reads `IsmpParachain::KnownRelayHeights` on the counterparty at the
/// provided block height and returns the largest relay block number it has a
/// state root for.
///
/// `KnownRelayHeights` is a `StorageValue<BoundedBTreeSet<u32, _>>`. Because
/// `BTreeSet` serialises (and decodes) in ascending key order, `.last()` is
/// guaranteed to be the largest known relay height without an extra sort.
///
/// Callers pass the counterparty's latest *finalized* height (instead of the
/// unfinalized tip) to avoid chasing a height that's still reorg-eligible on
/// the counterparty before we submit a proof that references it.
async fn query_latest_known_relay_height(
	counterparty: &dyn IsmpProvider,
	at: u64,
) -> Result<Option<u32>, anyhow::Error> {
	let key = StorageKey::Substrate(known_relay_heights_storage_key());
	let Some(raw) = counterparty.query_storage(key, Some(at)).await? else {
		return Ok(None);
	};
	let heights: std::collections::BTreeSet<u32> = Decode::decode(&mut &raw[..])?;
	Ok(heights.last().copied())
}

/// Storage key for `pallet_ismp_parachain::KnownRelayHeights` — `twox_128("IsmpParachain")
/// ++ twox_128("KnownRelayHeights")`.
fn known_relay_heights_storage_key() -> Vec<u8> {
	let mut key = sp_core::twox_128(b"IsmpParachain").to_vec();
	key.extend_from_slice(&sp_core::twox_128(b"KnownRelayHeights"));
	key
}

/// True if any of the events represent an application-level ISMP message the
/// counterparty would need to relay — i.e. a PostRequest or GetRequest
/// originating from self. Pure consensus/bookkeeping events
/// (`StateMachineUpdated`, `PostRequestHandled`, ...) don't require a new
/// consensus proof on their own.
fn has_relay_worthy_events(events: &[Event]) -> bool {
	events.iter().any(|ev| {
		matches!(ev, Event::PostRequest(_) | Event::GetRequest(_))
	})
}
