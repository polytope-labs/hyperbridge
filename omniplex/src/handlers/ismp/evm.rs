//! The EVM host's events (`handlers/events/evmHost`) and the fee token's `Transfer`
//! (`handlers/events/erc6160ext20`). Timestamps are the block's, in seconds.

use std::sync::Arc;

use alloy::primitives::{Address, U256};
use anyhow::anyhow;
use async_trait::async_trait;

use super::{
	calldata::MessageFunction,
	commitment::{get_request_commitment, post_request_commitment},
	delivery::{
		read_transfers, request_handled, store_transfer, store_transfers, sweep_transfers,
		Attribution,
	},
	non_critical,
	requests::{
		insert_status, update_status, upsert_get_request, upsert_post_request, GetRequestRow,
		PostRequestRow, StatusUpdate,
	},
	state_machine::{evm_commitment_timestamp, save_state_machine_update, StateMachineUpdate},
	Shared,
};
use crate::{
	handlers::abi::{Erc6160Ext20, EthereumHost},
	registry::{EventContext, EventHandler},
	services::{
		pending::PendingEntity,
		relayers::{
			add_chain_stats, add_relayer_stats, touch_relayer_activity, ChainStatsDelta,
			RelayerStatsDelta,
		},
	},
	store::{numeric, Status},
};

fn u64_of(value: U256, what: &str) -> anyhow::Result<u64> {
	u64::try_from(value).map_err(|_| anyhow!("{what} {value} overflows uint64"))
}

fn status<'a>(
	ctx: &'a EventContext<'_>,
	commitment: &'a str,
	status: Status,
	block_hash: &'a str,
	transaction_hash: &'a str,
) -> StatusUpdate<'a> {
	StatusUpdate {
		commitment,
		status,
		chain: ctx.chain.state_machine_id(),
		timestamp: ctx.log.block_timestamp,
		block_number: ctx.log.block_number,
		block_hash,
		transaction_hash,
	}
}

fn sent_message() -> ChainStatsDelta {
	ChainStatsDelta { messages_sent: numeric(1), ..Default::default() }
}

fn timed_out_message() -> ChainStatsDelta {
	ChainStatsDelta { timed_out_messages: numeric(1), ..Default::default() }
}

/** E1 `handlePostRequestEvent`. */
pub struct PostRequestEvent(pub Arc<Shared>);

#[async_trait]
impl EventHandler for PostRequestEvent {
	type Event = EthereumHost::PostRequestEvent;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let chain = ctx.chain.state_machine_id();
		let timestamp = ctx.log.block_timestamp;
		let nonce = u64_of(event.nonce, "nonce")?;
		let timeout = u64_of(event.timeoutTimestamp, "timeoutTimestamp")?;
		let commitment = format!(
			"{:#x}",
			post_request_commitment(
				&event.source,
				&event.dest,
				nonce,
				timeout,
				event.from.as_slice(),
				&event.to,
				&event.body,
			)
		);
		let from = event.from.to_checksum(None);
		upsert_post_request(
			ctx.db,
			&PostRequestRow {
				commitment: &commitment,
				chain,
				source: &event.source,
				dest: &event.dest,
				from: &from,
				to: &event.to.to_string(),
				body: &event.body.to_string(),
				nonce: numeric(nonce),
				timeout_timestamp: numeric(timeout),
				fee: numeric(event.fee),
				block_timestamp: timestamp,
			},
		)
		.await?;
		let block_hash = format!("{:#x}", ctx.log.block_hash);
		let tx = format!("{:#x}", ctx.log.transaction_hash);
		let source = status(ctx, &commitment, Status::Source, &block_hash, &tx);
		insert_status(ctx.db, "request_status_metadata", &source).await?;

		let transaction = self.0.transaction(ctx);
		let extras = async {
			add_chain_stats(ctx.db, chain, &sent_message()).await?;
			sweep_transfers(ctx, &transaction, timestamp, Attribution::Sender(&from)).await
		};
		non_critical(extras.await, "PostRequestEvent stats")
	}
}

/** E2 `handlePostRequestHandledEvent`. */
pub struct PostRequestHandled(pub Arc<Shared>);

#[async_trait]
impl EventHandler for PostRequestHandled {
	type Event = EthereumHost::PostRequestHandled;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let commitment = format!("{:#x}", event.commitment);
		let block_hash = format!("{:#x}", ctx.log.block_hash);
		let tx = format!("{:#x}", ctx.log.transaction_hash);
		let update = status(ctx, &commitment, Status::Destination, &block_hash, &tx);
		update_status(ctx.db, PendingEntity::Request, &update).await?;
		delivered(ctx, &self.0, event.relayer, MessageFunction::HandlePostRequests).await
	}
}

/** E5 `handleGetRequestHandledEvent`: on the GET's source chain, its response was delivered. */
pub struct GetRequestHandled(pub Arc<Shared>);

#[async_trait]
impl EventHandler for GetRequestHandled {
	type Event = EthereumHost::GetRequestHandled;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let commitment = format!("{:#x}", event.commitment);
		let block_hash = format!("{:#x}", ctx.log.block_hash);
		let tx = format!("{:#x}", ctx.log.transaction_hash);
		let update = status(ctx, &commitment, Status::Destination, &block_hash, &tx);
		update_status(ctx.db, PendingEntity::GetRequest, &update).await?;
		delivered(ctx, &self.0, event.relayer, MessageFunction::HandleGetResponses).await
	}
}

/** The non-critical half of E2 and E5: relayer stats, then the transfer sweep. */
async fn delivered(
	ctx: &EventContext<'_>,
	shared: &Shared,
	relayer: Address,
	function: MessageFunction,
) -> anyhow::Result<()> {
	let chain = ctx.chain.state_machine_id();
	let timestamp = ctx.log.block_timestamp;
	let transaction = shared.transaction(ctx);
	let relayer = relayer.to_checksum(None);
	let extras = async {
		// Every read comes before the writes: the relayer's rows, which other chains' processes
		// also write, stay locked until the block commits. A fee that can't be read ends the
		// work before the sweep, as the TypeScript threw before it.
		let fee = transaction.fee(ctx).await;
		let sweep = match fee {
			Ok(_) => Some(
				read_transfers(ctx, &transaction, timestamp, Attribution::Modules(function))
					.await?,
			),
			Err(_) => None,
		};
		request_handled(ctx, &relayer, chain, timestamp, Some(fee)).await?;
		match sweep {
			Some(sweep) => store_transfers(ctx, sweep).await,
			None => Ok(()),
		}
	};
	non_critical(extras.await, "request handled stats")
}

/** E3 `handlePostRequestTimeoutHandledEvent`. */
pub struct PostRequestTimeoutHandled(pub Arc<Shared>);

#[async_trait]
impl EventHandler for PostRequestTimeoutHandled {
	type Event = EthereumHost::PostRequestTimeoutHandled;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let commitment = format!("{:#x}", event.commitment);
		let block_hash = format!("{:#x}", ctx.log.block_hash);
		let tx = format!("{:#x}", ctx.log.transaction_hash);
		let update = status(ctx, &commitment, Status::TimedOut, &block_hash, &tx);
		update_status(ctx.db, PendingEntity::Request, &update).await?;
		timed_out(ctx, &self.0, MessageFunction::HandlePostRequestTimeouts).await
	}
}

/** E6 `handleGetRequestTimeoutHandledEvent`. */
pub struct GetRequestTimeoutHandled(pub Arc<Shared>);

#[async_trait]
impl EventHandler for GetRequestTimeoutHandled {
	type Event = EthereumHost::GetRequestTimeoutHandled;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let commitment = format!("{:#x}", event.commitment);
		let block_hash = format!("{:#x}", ctx.log.block_hash);
		let tx = format!("{:#x}", ctx.log.transaction_hash);
		let update = status(ctx, &commitment, Status::TimedOut, &block_hash, &tx);
		update_status(ctx.db, PendingEntity::GetRequest, &update).await?;
		timed_out(ctx, &self.0, MessageFunction::HandleGetRequestTimeouts).await
	}
}

/** The non-critical half of E3 and E6. */
async fn timed_out(
	ctx: &EventContext<'_>,
	shared: &Shared,
	function: MessageFunction,
) -> anyhow::Result<()> {
	let chain = ctx.chain.state_machine_id();
	let timestamp = ctx.log.block_timestamp;
	let transaction = shared.transaction(ctx);
	let extras = async {
		add_chain_stats(ctx.db, chain, &timed_out_message()).await?;
		sweep_transfers(ctx, &transaction, timestamp, Attribution::Modules(function)).await
	};
	non_critical(extras.await, "timeout stats")
}

/** E4 `handleGetRequestEvent`. */
pub struct GetRequestEvent(pub Arc<Shared>);

#[async_trait]
impl EventHandler for GetRequestEvent {
	type Event = EthereumHost::GetRequestEvent;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let chain = ctx.chain.state_machine_id();
		let timestamp = ctx.log.block_timestamp;
		let nonce = u64_of(event.nonce, "nonce")?;
		let height = u64_of(event.height, "height")?;
		let timeout = u64_of(event.timeoutTimestamp, "timeoutTimestamp")?;
		let commitment = format!(
			"{:#x}",
			get_request_commitment(
				&event.source,
				&event.dest,
				nonce,
				height,
				timeout,
				&event.from,
				&event.keys,
				&event.context,
			)
		);
		let from = event.from.to_string();
		let block_hash = format!("{:#x}", ctx.log.block_hash);
		let tx = format!("{:#x}", ctx.log.transaction_hash);
		upsert_get_request(
			ctx.db,
			&GetRequestRow {
				commitment: &commitment,
				chain,
				source: &event.source,
				dest: &event.dest,
				from: &from,
				keys: event.keys.iter().map(ToString::to_string).collect(),
				nonce: numeric(nonce),
				height: numeric(height),
				context: &event.context.to_string(),
				timeout_timestamp: numeric(timeout),
				fee: numeric(event.fee),
				block_number: ctx.log.block_number,
				block_hash: &block_hash,
				transaction_hash: &tx,
				block_timestamp: timestamp,
			},
		)
		.await?;
		let source = status(ctx, &commitment, Status::Source, &block_hash, &tx);
		insert_status(ctx.db, "get_request_status_metadata", &source).await?;

		let transaction = self.0.transaction(ctx);
		let extras = async {
			add_chain_stats(ctx.db, chain, &sent_message()).await?;
			sweep_transfers(ctx, &transaction, timestamp, Attribution::Sender(&from)).await
		};
		non_critical(extras.await, "GetRequestEvent stats")
	}
}

/** E7 `handleStateMachineUpdatedEvent`. */
pub struct StateMachineUpdated;

#[async_trait]
impl EventHandler for StateMachineUpdated {
	type Event = EthereumHost::StateMachineUpdated;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let chain = ctx.chain.state_machine_id();
		let host = ctx
			.chain
			.config
			.contracts
			.ethereum_host
			.ok_or_else(|| anyhow!("{} has no ethereum_host", ctx.chain.name))?;
		let height = u64_of(event.height, "height")?;
		let commitment = match evm_commitment_timestamp(
			ctx.chain,
			host,
			&event.stateMachineId,
			height,
			ctx.log.block_number,
		)
		.await
		{
			Ok(Some(commitment)) => commitment,
			Ok(None) => return Ok(()),
			Err(err) if super::is_transient(&err) => return Err(err),
			Err(err) => {
				tracing::warn!("state commitment for {}: {err:#}", event.stateMachineId);
				return Ok(());
			},
		};
		let tx = format!("{:#x}", ctx.log.transaction_hash);
		save_state_machine_update(
			ctx.db,
			&StateMachineUpdate {
				id: format!("{chain}_{tx}_{}_{height}", event.stateMachineId),
				state_machine_id: &event.stateMachineId,
				height,
				chain,
				block_hash: &format!("{:#x}", ctx.log.block_hash),
				block_number: ctx.log.block_number,
				transaction_hash: &tx,
				transaction_index: ctx.log.transaction_index,
				commitment_timestamp: commitment,
				timestamp: ctx.log.block_timestamp,
			},
		)
		.await
	}
}

/**
 * E8 `handleTransferEvent`: fee-token transfers into or out of any configured host. Transfers
 * out pay a relayer; the TypeScript created the recipient as a relayer first, so every transfer
 * out counts as relayer fees.
 */
pub struct FeeTokenTransfer;

#[async_trait]
impl EventHandler for FeeTokenTransfer {
	type Event = Erc6160Ext20::Transfer;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let hosts: Vec<Address> = ctx
			.indexer
			.config
			.chains
			.values()
			.filter_map(|chain| chain.contracts.ethereum_host)
			.collect();
		let from_host = hosts.contains(&event.from);
		let to_host = hosts.contains(&event.to);
		if !from_host && !to_host {
			return Ok(());
		}
		let chain = ctx.chain.state_machine_id();
		let from = event.from.to_checksum(None);
		let to = event.to.to_checksum(None);
		let tx = format!("{:#x}", ctx.log.transaction_hash);
		store_transfer(ctx.db, &tx, chain, event.value, &from, &to).await?;
		let amount = numeric(event.value);

		if from_host {
			let timestamp = ctx.log.block_timestamp;
			let fees = async {
				let delta = RelayerStatsDelta { fees_earned: amount.clone(), ..Default::default() };
				add_relayer_stats(ctx.db, &to, chain, &delta).await?;
				touch_relayer_activity(ctx.db, &to, timestamp).await?;
				let relayer =
					ctx.db.query_opt("SELECT 1 FROM app.relayer_v2s WHERE id = $1", &[&to]).await?;
				if relayer.is_some() {
					let delta = ChainStatsDelta {
						fees_paid_to_relayers: amount.clone(),
						..Default::default()
					};
					add_chain_stats(ctx.db, chain, &delta).await?;
				}
				anyhow::Ok(())
			};
			non_critical(fees.await, "fee token transfer")?;
		}
		if to_host {
			let delta = ChainStatsDelta { transfers_in: amount, ..Default::default() };
			add_chain_stats(ctx.db, chain, &delta).await?;
		}
		Ok(())
	}
}
