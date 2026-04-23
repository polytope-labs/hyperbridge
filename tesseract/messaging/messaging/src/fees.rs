// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0.

//! Automatic fee withdrawal loop.
//!
//! Periodically queries each destination chain's unclaimed balance on
//! Hyperbridge, submits a withdrawal request once a minimum threshold is
//! crossed, and delivers the resulting GET response back to the destination.
//! Persists in-flight withdrawals to the DB so a crash between "HB request
//! submitted" and "destination delivered" doesn't lose funds.
//!
//! Extracted from `tesseract/messaging/relayer/src/fees.rs` into the shared
//! messaging crate so the consolidated relayer in `tesseract-relayer` can
//! reuse it.

use anyhow::anyhow;
use futures::StreamExt;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::{hash_request, ConsensusMessage, Message, Proof, RequestMessage},
	router::Request,
};
use sp_core::U256;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tesseract_primitives::{
	config::RelayerConfig, ConsensusProofSource, Cost, Hasher, HyperbridgeClaim, IsmpProvider,
	Query, WithdrawFundsResult, BEEFY_CONSENSUS_STATE_ID,
};
use tokio_stream::wrappers::IntervalStream;
use tracing::{instrument, Instrument};
use transaction_fees::TransactionPayment;

/// For every configured `withdrawal_frequency`, attempts to withdraw all
/// unclaimed fees on hyperbridge for every destination chain in `clients`.
///
/// - Delivers any pending (persisted) withdrawals from a previous run first.
/// - Skips chains whose unclaimed balance is below `minimum_withdrawal_amount` (default 100 units
///   of the fee token).
/// - Records every new withdrawal request in the DB before submitting it to the destination so a
///   crash between "HB accepted" and "destination delivered" doesn't lose funds.
pub async fn auto_withdraw<C>(
	hyperbridge: C,
	clients: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	config: RelayerConfig,
	db: Arc<TransactionPayment>,
	proof_source: Arc<dyn ConsensusProofSource>,
) -> anyhow::Result<()>
where
	C: IsmpProvider + HyperbridgeClaim + Clone,
{
	// default to 1 day
	let frequency = Duration::from_secs(config.withdrawal_frequency.unwrap_or(86_400));
	let min_amount_initial: U256 = (config
		.minimum_withdrawal_amount
		.map(|val| std::cmp::max(val, 10))
		.unwrap_or(100) as u128 *
		10u128.pow(18))
	.into();
	tracing::info!(
		target: crate::LOG_TARGET, frequency_secs = frequency.as_secs(),
		min_amount_usd = %Cost(min_amount_initial),
		"auto-withdraw configured",
	);
	let mut interval = IntervalStream::new(tokio::time::interval(frequency));

	while let Some(_) = interval.next().await {
		withdraw_once(&hyperbridge, &clients, &config, &db, &proof_source).await;
	}

	Ok(())
}

/// Run a single withdrawal pass across every chain in `clients`: check the
/// unclaimed balance, deliver any persisted pending withdrawals from a
/// previous run, then submit a new withdrawal if the balance crossed the
/// configured threshold. Shared between the periodic [`auto_withdraw`] loop
/// and the `withdraw` subcommand.
pub async fn withdraw_once<C>(
	hyperbridge: &C,
	clients: &HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	config: &RelayerConfig,
	db: &Arc<TransactionPayment>,
	proof_source: &Arc<dyn ConsensusProofSource>,
) where
	C: IsmpProvider + HyperbridgeClaim + Clone,
{
	let stream = futures::stream::iter(clients.keys().cloned());
	stream
		.for_each_concurrent(None, |chain| {
			let client =
				clients.get(&chain).expect(&format!("Client not found for {chain}")).clone();
			let hyperbridge = hyperbridge.clone();
			let moved_db = db.clone();
			let config = config.clone();
			let proof_source = proof_source.clone();
			let span = tracing::info_span!("withdraw_tick", %chain);
			async move {
				let lambda = || async {
					// Deliver any pending withdrawals persisted from a
					// previous run before making new ones.
					let (pending_withdrawals, ids): (Vec<_>, Vec<_>) =
						moved_db.pending_withdrawals(&chain).await?.into_iter().unzip();
					for pending in pending_withdrawals {
						deliver_post_request(
							client.clone(),
							&hyperbridge,
							&proof_source,
							vec![pending],
						)
						.await?;
					}
					if let Err(err) = moved_db.delete_pending_withdrawals(ids).await {
						tracing::error!(
							target: crate::LOG_TARGET, ?err,
							"failed to delete pending withdrawals (delivered ok)"
						);
					}

					let amount = hyperbridge.available_amount(client.clone(), &chain).await?;
					let fee_token_decimals = client.fee_token_decimals().await?;
					let min_amount: U256 = (config
						.minimum_withdrawal_amount
						.map(|val| std::cmp::max(val, 10))
						.unwrap_or(100) as u128 *
						10u128.pow(fee_token_decimals.into()))
					.into();
					if amount < min_amount {
						tracing::info!(
							target: crate::LOG_TARGET, unclaimed = %amount,
							min = %min_amount,
							"balance below threshold; skipping",
						);
						return Ok::<_, anyhow::Error>(());
					}

					let amount_usd = amount / U256::from(10u128.pow(fee_token_decimals.into()));
					tracing::info!(target: crate::LOG_TARGET, amount_usd = %amount_usd, "submitting withdrawal request");
					let results = hyperbridge.withdraw_funds(client.clone(), chain).await?;
					tracing::info!(target: crate::LOG_TARGET, "withdrawal request accepted; delivering to destination");

					// Persist so a crash before delivery doesn't lose the funds.
					let ids = moved_db.store_pending_withdrawals(results.clone()).await?;

					match deliver_post_request(client.clone(), &hyperbridge, &proof_source, results)
						.await
					{
						Ok(_) =>
							if let Err(err) = moved_db.delete_pending_withdrawals(ids).await {
								tracing::error!(
									target: crate::LOG_TARGET, ?err,
									"failed to delete pending withdrawals (delivered ok)"
								);
							},
						Err(err) => {
							tracing::info!(target: crate::LOG_TARGET, ?err, "delivery failed; will be retried");
						},
					};
					Ok(())
				};

				if let Err(err) = lambda().await {
					tracing::error!(target: crate::LOG_TARGET, ?err, "withdraw tick failed");
				}
			}
			.instrument(span)
		})
		.await;
}

/// Dispatch the withdrawal delivery along the path that matches the
/// destination family:
///
/// - **EVM destinations** get the "bundle a BEEFY consensus proof alongside
///   the request" path ([`deliver_post_request_evm`]): waits for HB to emit
///   a `ProofAccepted` at or above `max_block`, fetches the accepted proof,
///   builds a `ConsensusMessage` + `RequestMessage` batch, and submits both
///   in one tx so destinations that haven't yet seen this HB height can
///   verify both atomically. Every `hyperbridge.state_machine_id()` call on
///   this path has its `consensus_state_id` swapped for
///   [`BEEFY_CONSENSUS_STATE_ID`] — EVM hosts track the BEEFY client, not
///   the parachain one.
/// - **Non-EVM (substrate) destinations** go through
///   [`deliver_post_request_substrate`]: they don't need the consensus
///   bundle because an independent consensus task advances their light
///   client; we just wait for the destination's own
///   `StateMachineUpdated` to cross `max_block`, then submit the request
///   alone.
#[instrument(
	name = "deliver_post_request",
	skip_all,
	fields(destination = %dest_chain.state_machine_id().state_id)
)]
async fn deliver_post_request<D: IsmpProvider>(
	dest_chain: Arc<dyn IsmpProvider>,
	hyperbridge: &D,
	proof_source: &Arc<dyn ConsensusProofSource>,
	results: Vec<WithdrawFundsResult>,
) -> anyhow::Result<()> {
	if matches!(dest_chain.state_machine_id().state_id, StateMachine::Evm(_)) {
		deliver_post_request_evm(dest_chain, hyperbridge, proof_source, results).await
	} else {
		deliver_post_request_substrate(dest_chain, hyperbridge, results).await
	}
}

/// EVM-destination delivery: bundle the BEEFY consensus proof + request in
/// one submission. Every reference to Hyperbridge's state machine id uses
/// [`BEEFY_CONSENSUS_STATE_ID`] as the consensus id — the EVM host verifies
/// against the BEEFY light client, not the parachain one that HB's own
/// `state_machine_id()` returns.
async fn deliver_post_request_evm<D: IsmpProvider>(
	dest_chain: Arc<dyn IsmpProvider>,
	hyperbridge: &D,
	proof_source: &Arc<dyn ConsensusProofSource>,
	results: Vec<WithdrawFundsResult>,
) -> anyhow::Result<()> {
	if results.is_empty() {
		return Ok(());
	}
	let max_block =
		results.iter().map(|r| r.block).max().expect("results non-empty, checked above");

	// Same state-id as `hyperbridge.state_machine_id()` but with the
	// consensus id retargeted at the destination's BEEFY client.
	let hb_state_machine_id = StateMachineId {
		state_id: hyperbridge.state_machine_id().state_id,
		consensus_state_id: BEEFY_CONSENSUS_STATE_ID,
	};

	// Wait for HB's own ProofAccepted to reach max_block — that's the signal
	// that an accepted proof exists in offchain storage that we can bundle
	// alongside the request message below, advancing the destination's view
	// of HB high enough to verify the request proof in the same tx.
	let mut latest_height = hyperbridge.query_latest_height(hb_state_machine_id).await? as u64;

	if max_block > latest_height {
		tracing::info!(target: crate::LOG_TARGET, target_height = max_block, "waiting for proof accepted");
		let mut stream = hyperbridge.proof_accepted_notification().await?;

		latest_height = loop {
			match stream.next().await {
				Some(Ok(event)) =>
					if event.height < max_block {
						continue;
					} else {
						tracing::info!(target: crate::LOG_TARGET, height = event.height, "proof accepted");
						break event.height;
					},
				Some(Err(_)) => {
					tracing::error!(target: crate::LOG_TARGET, chain = %dest_chain.name(), "proof_accepted error; retrying");
				},
				None => return Err(anyhow!("Proof accepted stream ended")),
			}
		};
	}

	// Bundle the BEEFY consensus proof alongside the request message so
	// destinations that haven't yet seen this HB height can verify both in
	// one tx.
	let consensus_proof = proof_source.fetch(latest_height).await?;
	let consensus_msg = ConsensusMessage {
		consensus_proof,
		consensus_state_id: BEEFY_CONSENSUS_STATE_ID,
		signer: dest_chain.address(),
	};

	let queries = results
		.iter()
		.map(|result| Query {
			source_chain: result.post.source,
			dest_chain: result.post.dest,
			nonce: result.post.nonce,
			commitment: hash_request::<Hasher>(&Request::Post(result.post.clone())),
		})
		.collect::<Vec<_>>();

	let requests = results.iter().map(|r| r.post.clone()).collect::<Vec<_>>();
	tracing::debug!(target: crate::LOG_TARGET, height = latest_height, "querying request proof");
	let proof = hyperbridge
		.query_requests_proof(latest_height, queries, dest_chain.state_machine_id().state_id)
		.await?;
	let request_msg = RequestMessage {
		requests,
		proof: Proof {
			height: StateMachineHeight { id: hb_state_machine_id, height: latest_height },
			proof,
		},
		signer: dest_chain.address(),
	};

	let batch = vec![Message::Consensus(consensus_msg), Message::Request(request_msg)];

	let mut count = 5;
	while count != 0 {
		if let Err(err) = dest_chain.submit(batch.clone(), hb_state_machine_id.state_id).await {
			tracing::info!(target: crate::LOG_TARGET, ?err, retries_left = count, "withdrawal submit failed; retrying");
			count -= 1;
		} else {
			tracing::info!(target: crate::LOG_TARGET, "withdrawal delivered");
			return Ok(());
		}
	}

	Err(anyhow!("Failed to deliver post request"))
}

/// Substrate-destination delivery: no consensus bundle needed — an
/// independent consensus task already advances the destination's
/// hyperbridge light client. We wait for the destination's own
/// `StateMachineUpdated` for HB to cross `max_block`, then submit the
/// request message standalone.
///
/// Ported from the legacy `tesseract/messaging/relayer/src/fees.rs` path.
async fn deliver_post_request_substrate<D: IsmpProvider>(
	dest_chain: Arc<dyn IsmpProvider>,
	hyperbridge: &D,
	results: Vec<WithdrawFundsResult>,
) -> anyhow::Result<()> {
	if results.is_empty() {
		return Ok(());
	}
	let max_block = results
		.iter()
		.map(|r| r.block)
		.max()
		.expect("results non-empty, checked above");

	let mut latest_height =
		dest_chain.query_latest_height(hyperbridge.state_machine_id()).await? as u64;

	if max_block > latest_height {
		tracing::info!(
			target: crate::LOG_TARGET,
			target_height = max_block,
			"waiting for state machine update finalizing withdraw height",
		);
		let mut stream =
			dest_chain.state_machine_update_notification(hyperbridge.state_machine_id()).await?;

		latest_height = loop {
			match stream.next().await {
				Some(Ok(event)) =>
					if event.latest_height < max_block {
						continue;
					} else {
						tracing::trace!(
							target: crate::LOG_TARGET,
							height = event.latest_height,
							"found state machine update",
						);
						break event.latest_height;
					},
				Some(Err(_)) => {
					tracing::error!(
						target: crate::LOG_TARGET,
						chain = %dest_chain.name(),
						"state machine update stream error; retrying",
					);
				},
				None => return Err(anyhow!("state machine update stream ended")),
			}
		};
	}

	let queries = results
		.iter()
		.map(|result| Query {
			source_chain: result.post.source,
			dest_chain: result.post.dest,
			nonce: result.post.nonce,
			commitment: hash_request::<Hasher>(&Request::Post(result.post.clone())),
		})
		.collect::<Vec<_>>();

	let requests = results.iter().map(|r| r.post.clone()).collect::<Vec<_>>();
	tracing::info!(
		target: crate::LOG_TARGET,
		height = latest_height,
		"querying request proof from hyperbridge",
	);
	let proof = hyperbridge
		.query_requests_proof(latest_height, queries, dest_chain.state_machine_id().state_id)
		.await?;
	let msg = RequestMessage {
		requests,
		proof: Proof {
			height: StateMachineHeight {
				id: hyperbridge.state_machine_id(),
				height: latest_height,
			},
			proof,
		},
		signer: dest_chain.address(),
	};

	let mut count = 5;
	while count != 0 {
		if let Err(err) = dest_chain
			.submit(vec![Message::Request(msg.clone())], hyperbridge.state_machine_id().state_id)
			.await
		{
			tracing::info!(
				target: crate::LOG_TARGET,
				?err,
				retries_left = count,
				"withdrawal submit failed; retrying",
			);
			count -= 1;
		} else {
			tracing::info!(target: crate::LOG_TARGET, "withdrawal delivered");
			return Ok(());
		}
	}

	Err(anyhow!("Failed to deliver post request"))
}
