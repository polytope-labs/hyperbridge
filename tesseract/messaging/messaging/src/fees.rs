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
	consensus::StateMachineHeight,
	host::StateMachine,
	messaging::{hash_request, Message, Proof, RequestMessage},
	router::Request,
};
use sp_core::U256;
use std::{collections::HashMap, sync::Arc};
use tesseract_primitives::{
	config::RelayerConfig, Cost, Hasher, HyperbridgeClaim, IsmpProvider, Query, WithdrawFundsResult,
};
use tracing::{instrument, Instrument};
use transaction_fees::TransactionPayment;

/// Runs a withdrawal pass on every `ProofAccepted` event emitted by Hyperbridge,
/// attempting to withdraw all unclaimed fees for every destination chain in
/// `clients`.
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
) -> anyhow::Result<()>
where
	C: IsmpProvider + HyperbridgeClaim + Clone,
{
	let min_amount_initial: U256 = (config
		.minimum_withdrawal_amount
		.map(|val| std::cmp::max(val, 10))
		.unwrap_or(100) as u128 *
		10u128.pow(18))
	.into();
	tracing::info!(
		min_amount_usd = %Cost(min_amount_initial),
		"auto-withdraw subscribed to ProofAccepted",
	);

	let mut stream = hyperbridge.proof_accepted_notification().await?;
	while let Some(item) = stream.next().await {
		match item {
			Ok(_) => withdraw_once(&hyperbridge, &clients, &config, &db).await,
			Err(err) => tracing::error!(?err, "proof_accepted stream error"),
		}
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
			let span = tracing::info_span!("withdraw_tick", %chain);
			async move {
				let lambda = || async {
					// Deliver any pending withdrawals persisted from a
					// previous run before making new ones.
					let (pending_withdrawals, ids): (Vec<_>, Vec<_>) =
						moved_db.pending_withdrawals(&chain).await?.into_iter().unzip();
					for pending in pending_withdrawals {
						deliver_post_request(client.clone(), &hyperbridge, vec![pending]).await?;
					}
					if let Err(err) = moved_db.delete_pending_withdrawals(ids).await {
						tracing::error!(
							?err,
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
							unclaimed = %amount,
							min = %min_amount,
							"balance below threshold; skipping",
						);
						return Ok::<_, anyhow::Error>(());
					}

					let amount_usd = amount / U256::from(10u128.pow(fee_token_decimals.into()));
					tracing::info!(amount_usd = %amount_usd, "submitting withdrawal request");
					let results = hyperbridge.withdraw_funds(client.clone(), chain).await?;
					tracing::info!("withdrawal request accepted; delivering to destination");

					// Persist so a crash before delivery doesn't lose the funds.
					let ids = moved_db.store_pending_withdrawals(results.clone()).await?;

					match deliver_post_request(client.clone(), &hyperbridge, results).await {
						Ok(_) =>
							if let Err(err) = moved_db.delete_pending_withdrawals(ids).await {
								tracing::error!(
									?err,
									"failed to delete pending withdrawals (delivered ok)"
								);
							},
						Err(err) => {
							tracing::info!(?err, "delivery failed; will be retried");
						},
					};
					Ok(())
				};

				if let Err(err) = lambda().await {
					tracing::error!(?err, "withdraw tick failed");
				}
			}
			.instrument(span)
		})
		.await;
}

#[instrument(
	name = "deliver_post_request",
	skip_all,
	fields(destination = %dest_chain.state_machine_id().state_id)
)]
async fn deliver_post_request<D: IsmpProvider>(
	dest_chain: Arc<dyn IsmpProvider>,
	hyperbridge: &D,
	results: Vec<WithdrawFundsResult>,
) -> anyhow::Result<()> {
	if results.is_empty() {
		return Ok(());
	}
	let max_block =
		results.iter().map(|r| r.block).max().expect("results non-empty, checked above");

	let mut latest_height =
		dest_chain.query_latest_height(hyperbridge.state_machine_id()).await? as u64;

	if max_block > latest_height {
		tracing::info!(target_height = max_block, "waiting for state machine update");
		let mut stream = dest_chain
			.state_machine_update_notification(hyperbridge.state_machine_id())
			.await?;

		latest_height = loop {
			match stream.next().await {
				Some(Ok(event)) =>
					if event.latest_height < max_block {
						continue;
					} else {
						tracing::info!(height = event.latest_height, "state machine update");
						break event.latest_height;
					},
				Some(Err(_)) => {
					tracing::error!(chain = %dest_chain.name(), "state_machine_update error; retrying");
				},
				None => return Err(anyhow!("State machine update stream ended")),
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
	tracing::debug!(height = latest_height, "querying request proof");
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
			tracing::info!(?err, retries_left = count, "withdrawal submit failed; retrying");
			count -= 1;
		} else {
			tracing::info!("withdrawal delivered");
			return Ok(());
		}
	}

	Err(anyhow!("Failed to deliver post request"))
}
