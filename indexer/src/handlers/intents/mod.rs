//! Intent Gateway V3 orders, fills, escrow, dust and filler bids, ported from the TypeScript
//! indexer's `handlers/events/intentGatewayV3` and `handleBidPlaced`.

pub mod bids;
pub mod calls;
pub mod dust;
pub mod orders;

use std::str::FromStr;

use alloy::{
	eips::BlockId,
	primitives::{Address, B256},
	sol,
};
use async_trait::async_trait;

use self::{
	calls::{
		decode_fill_order, decode_place_order, find_user_op_hash, hex_bytes,
		match_delivery_transfers, nested_call_inputs, placement_user_op_hash, receipt_logs,
		transaction_input, Order, TokenAmount,
	},
	dust::Dust,
	orders::{
		bytes32_to_bytes20, lower_hex, FeeToken, FillKind, FillReceipt, Gateway, Market, TxMeta,
		DEFAULT_REFERRER,
	},
};
use super::{
	abi::{pallets::IntentsCoprocessor, IntentGatewayV3, IntentGatewayV3Legacy},
	Handlers,
};
use crate::{
	chains::Chain,
	errors::is_transient,
	registry::{EventContext, EventHandler, Source, Subscription},
	services::{decimal::Dec, prices::Orderbook, solvers::discover_solver_from_fill},
	store::{numeric, OrderStatus},
};

sol! {
	/** The `EscrowReleased` the gateway emitted before partial fills, without the solver. */
	interface LegacyEscrow {
		struct TokenInfo { bytes32 token; uint256 amount; }
		event EscrowReleased(bytes32 indexed commitment, TokenInfo[] tokens);
	}

	#[sol(rpc)]
	interface FeeTokenHost {
		function feeToken() external view returns (address);
	}

	#[sol(rpc)]
	interface Decimals {
		function decimals() external view returns (uint8);
	}
}

pub fn register(handlers: &mut Handlers) {
	let gateway = Source::IntentGatewayV3;
	let legacy = Source::IntentGatewayV3Legacy;
	handlers.subscriptions.extend([
		Subscription::new(gateway, OrderPlaced),
		Subscription::new(legacy, LegacyOrderPlaced),
		Subscription::new(gateway, OrderFilled),
		Subscription::new(gateway, PartialFill),
		Subscription::new(gateway, EscrowReleased),
		Subscription::new(legacy, LegacyEscrowReleased),
		Subscription::new(gateway, EscrowRefunded),
		Subscription::new(gateway, ProtocolFeeRefunded),
		Subscription::new(gateway, OrderCancelled),
		Subscription::new(gateway, DustCollected),
		Subscription::new(gateway, DustSwept),
		Subscription::new(Source::Pallet("intentsCoprocessor"), BidPlaced),
	]);
}

/** Token metadata at the log's block, and orderbook prices. */
struct ChainMarket<'a> {
	chain: &'a Chain,
	orderbook: &'a Orderbook,
	block: u64,
}

#[async_trait]
impl Market for ChainMarket<'_> {
	async fn token_metadata(&self, token: Address) -> anyhow::Result<(String, u8)> {
		crate::services::prices::token_metadata(self.chain, token, self.block).await
	}

	async fn usd_price(&self, symbol: &str) -> Option<Dec> {
		self.orderbook.token_usd_price(symbol).await
	}
}

/** What every gateway handler builds from its context. */
struct Scope<'a> {
	ctx: &'a EventContext<'a>,
	chain_id: String,
	market: ChainMarket<'a>,
	tx: TxMeta,
}

impl<'a> Scope<'a> {
	fn new(ctx: &'a EventContext<'a>) -> Self {
		Self {
			ctx,
			chain_id: ctx.chain.config.chain_id.to_string(),
			market: ChainMarket {
				chain: ctx.chain,
				orderbook: &ctx.indexer.orderbook,
				block: ctx.log.block_number,
			},
			tx: TxMeta {
				transaction_hash: ctx.log.transaction_hash.to_string(),
				block_number: ctx.log.block_number,
				timestamp: ctx.log.block_timestamp,
				log_index: ctx.log.log_index,
			},
		}
	}

	fn gateway(&self) -> Gateway<'_> {
		Gateway {
			db: self.ctx.db,
			chain: &self.chain_id,
			state_machine: self.ctx.chain.state_machine_id(),
			market: &self.market,
		}
	}
}

fn tokens<T>(
	tokens: &[T],
	parts: impl Fn(&T) -> (B256, alloy::primitives::U256),
) -> Vec<TokenAmount> {
	tokens
		.iter()
		.map(|t| {
			let (token, amount) = parts(t);
			TokenAmount { token, amount }
		})
		.collect()
}

fn gateway_tokens(list: &[IntentGatewayV3::TokenInfo]) -> Vec<TokenAmount> {
	tokens(list, |t| (t.token, t.amount))
}

/** `resolveGraffiti`: graffiti equal to the placing user means no referrer. */
fn resolve_graffiti(graffiti: B256, user: B256) -> String {
	if graffiti == user {
		DEFAULT_REFERRER.to_string()
	} else {
		graffiti.to_string()
	}
}

/** `getHostFeeToken`: the host's fee token and its decimals at the log's block, if readable. */
async fn host_fee_token(chain: &Chain, block: u64) -> anyhow::Result<FeeToken> {
	let host = chain
		.config
		.contracts
		.ethereum_host
		.ok_or_else(|| anyhow::anyhow!("no ISMP host configured for {}", chain.name))?;
	let at = BlockId::number(block);
	let token = FeeTokenHost::new(host, &chain.provider).feeToken().block(at).call().await?;
	let decimals = Decimals::new(token, &chain.provider).decimals().block(at).call().await?;
	Ok(FeeToken { address: lower_hex(token), decimals })
}

/** The rest of `handleOrderPlacedEventV3`, once the order is complete. */
async fn place_order(scope: &Scope<'_>, order: Order, graffiti: String) -> anyhow::Result<()> {
	let ctx = scope.ctx;
	let commitment = order.commitment().to_string();
	let fee_token = match host_fee_token(ctx.chain, ctx.log.block_number).await {
		Ok(fee_token) => Some(fee_token),
		Err(err) if is_transient(&err) => return Err(err),
		Err(err) => {
			tracing::warn!(commitment, "could not read the fee token: {err:#}");
			None
		},
	};
	let sender = Address::from_slice(&order.user[12..]);
	let user_op_hash = match receipt_logs(ctx.chain, ctx.log.transaction_hash).await {
		Ok(logs) => placement_user_op_hash(&logs, sender, ctx.log.log_index).map(|h| h.to_string()),
		Err(err) if is_transient(&err) => return Err(err),
		Err(err) => {
			tracing::warn!(commitment, "could not read the placement receipt: {err:#}");
			None
		},
	};
	let gateway = scope.gateway();
	gateway
		.get_or_create_order(
			&order,
			&commitment,
			&graffiti,
			fee_token.as_ref(),
			user_op_hash.as_deref(),
			&scope.tx,
		)
		.await?;
	gateway
		.update_order_status(&commitment, OrderStatus::Placed, &scope.tx, None)
		.await?;
	gateway.record_order_volume("PLACED", &order.inputs, scope.tx.timestamp).await
}

struct OrderPlaced;

#[async_trait]
impl EventHandler for OrderPlaced {
	type Event = IntentGatewayV3::OrderPlaced;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let order = Order {
			user: event.user,
			source: event.source,
			destination: event.destination,
			deadline: event.deadline,
			nonce: event.nonce,
			fees: event.fees,
			session: event.session,
			predispatch: gateway_tokens(&event.predispatch),
			predispatch_call: event.predispatchCall,
			inputs: gateway_tokens(&event.inputs),
			beneficiary: event.beneficiary,
			outputs: gateway_tokens(&event.outputs),
			output_call: event.outputCall,
		};
		let graffiti = resolve_graffiti(event.graffiti, event.user);
		place_order(&Scope::new(ctx), order, graffiti).await
	}
}

/** Pre-#1092 placements: the call payloads and graffiti are only in the `placeOrder` calldata. */
struct LegacyOrderPlaced;

#[async_trait]
impl EventHandler for LegacyOrderPlaced {
	type Event = IntentGatewayV3Legacy::OrderPlaced;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let hash = ctx.log.transaction_hash;
		let mut decoded = match transaction_input(ctx.chain, hash).await {
			Ok(input) => input.as_deref().and_then(hex_bytes).and_then(|i| decode_place_order(&i)),
			Err(err) if is_transient(&err) => return Err(err),
			Err(err) => {
				tracing::info!(%hash, "no transaction input, trying nested calls: {err:#}");
				None
			},
		};
		if decoded.is_none() {
			match ctx.chain.config.contracts.intent_gateway_v3 {
				None => tracing::error!("no IntentGatewayV3 address for {}", ctx.chain.name),
				Some(gateway) => match nested_call_inputs(ctx.chain, hash, gateway).await {
					Ok(inputs) => match inputs.first() {
						Some(input) =>
							decoded = hex_bytes(input).and_then(|i| decode_place_order(&i)),
						None => tracing::warn!(%hash, "no nested IntentGateway call"),
					},
					Err(err) => tracing::error!(%hash, "tracing the IntentGateway call: {err:#}"),
				},
			}
		}
		// The TypeScript dropped a legacy placement whose calldata it could not recover.
		let Some((call, graffiti)) = decoded else {
			tracing::warn!(%hash, "legacy OrderPlaced without recoverable calldata, not indexed");
			return Ok(());
		};
		let legacy_tokens =
			|list: &[IntentGatewayV3Legacy::TokenInfo]| tokens(list, |t| (t.token, t.amount));
		let order = Order {
			user: event.user,
			source: event.source,
			destination: event.destination,
			deadline: event.deadline,
			nonce: event.nonce,
			fees: event.fees,
			session: event.session,
			predispatch: legacy_tokens(&event.predispatch),
			predispatch_call: call.predispatch.call,
			inputs: legacy_tokens(&event.inputs),
			beneficiary: call.output.beneficiary,
			outputs: legacy_tokens(&event.outputs),
			output_call: call.output.call,
		};
		place_order(&Scope::new(ctx), order, resolve_graffiti(graffiti, event.user)).await
	}
}

/** `resolveBeneficiary`: the order's beneficiary, from its indexed outputs or from the fill's
 * calldata checked against the commitment. */
async fn resolve_beneficiary(
	ctx: &EventContext<'_>,
	commitment: &B256,
) -> anyhow::Result<Option<Address>> {
	let indexed: Option<String> = ctx
		.db
		.query_opt(
			"SELECT beneficiary FROM app.i_order_v3_output_assets WHERE id = $1",
			&[&format!("{commitment}-output-0")],
		)
		.await?
		.map(|row| row.get(0));
	if let Some(beneficiary) = indexed.filter(|b| !b.is_empty()) {
		return Ok(Address::from_str(&bytes32_to_bytes20(&beneficiary)).ok());
	}
	let from_calldata = |calldata: &str| {
		let order = decode_fill_order(&hex_bytes(calldata)?)?;
		(calls::commitment(&order) == *commitment)
			.then(|| Address::from_slice(&order.output.beneficiary[12..]))
	};
	let hash = ctx.log.transaction_hash;
	match transaction_input(ctx.chain, hash).await {
		Ok(input) =>
			if let Some(beneficiary) = input.as_deref().and_then(from_calldata) {
				return Ok(Some(beneficiary));
			},
		Err(err) if is_transient(&err) => return Err(err),
		Err(err) => tracing::warn!(%commitment, "could not read the fill transaction: {err:#}"),
	}
	match nested_call_inputs(ctx.chain, hash, ctx.log.address).await {
		Ok(inputs) =>
			if let Some(beneficiary) = inputs.iter().find_map(|input| from_calldata(input)) {
				return Ok(Some(beneficiary));
			},
		Err(err) =>
			tracing::warn!(%commitment, "could not recover the order from fill calldata: {err:#}"),
	}
	tracing::warn!(%commitment, "no beneficiary for the fill, amounts received left unset");
	Ok(None)
}

/** `resolveFillEnrichment`. A missing receipt leaves both fields unset. */
async fn fill_receipt(
	ctx: &EventContext<'_>,
	commitment: &B256,
	filler: Address,
	outputs: &[TokenAmount],
) -> anyhow::Result<FillReceipt> {
	let logs = match receipt_logs(ctx.chain, ctx.log.transaction_hash).await {
		Ok(logs) => logs,
		Err(err) if is_transient(&err) => return Err(err),
		Err(err) => {
			tracing::warn!(%commitment, "could not read the fill receipt: {err:#}");
			return Ok(FillReceipt::default());
		},
	};
	let user_op_hash = find_user_op_hash(&logs, filler, ctx.log.log_index).map(|h| h.to_string());
	let amounts_received = resolve_beneficiary(ctx, commitment).await?.map(|beneficiary| {
		match_delivery_transfers(
			&logs,
			ctx.log.address,
			ctx.log.log_index,
			filler,
			beneficiary,
			outputs,
		)
	});
	Ok(FillReceipt { user_op_hash, amounts_received })
}

async fn fill(
	ctx: &EventContext<'_>,
	kind: FillKind,
	commitment: B256,
	filler: Address,
	outputs: Vec<TokenAmount>,
	inputs: Vec<TokenAmount>,
) -> anyhow::Result<()> {
	let scope = Scope::new(ctx);
	let gateway = scope.gateway();
	let receipt = fill_receipt(ctx, &commitment, filler, &outputs).await?;
	let commitment = commitment.to_string();
	let filler = filler.to_string();
	gateway
		.record_fill(kind, &commitment, &filler, &outputs, &inputs, &scope.tx, &receipt)
		.await?;
	if kind == FillKind::Full {
		gateway
			.update_order_status(&commitment, OrderStatus::Filled, &scope.tx, Some(&filler))
			.await?;
	}
	discover_solver_from_fill(
		ctx.db,
		&ctx.chain.config,
		&filler,
		&scope.tx.transaction_hash,
		ctx.log.block_number,
		ctx.log.block_timestamp,
	)
	.await?;
	if kind == FillKind::Full {
		// Only full fills record FILLED token volume, and only this call's outputs.
		gateway.record_order_volume("FILLED", &outputs, scope.tx.timestamp).await?;
	}
	Ok(())
}

struct OrderFilled;

#[async_trait]
impl EventHandler for OrderFilled {
	type Event = IntentGatewayV3::OrderFilled;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let outputs = gateway_tokens(&event.outputs);
		let inputs = gateway_tokens(&event.inputs);
		fill(ctx, FillKind::Full, event.commitment, event.filler, outputs, inputs).await
	}
}

struct PartialFill;

#[async_trait]
impl EventHandler for PartialFill {
	type Event = IntentGatewayV3::PartialFill;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let outputs = gateway_tokens(&event.outputs);
		let inputs = gateway_tokens(&event.inputs);
		fill(ctx, FillKind::Partial, event.commitment, event.filler, outputs, inputs).await
	}
}

struct EscrowReleased;

#[async_trait]
impl EventHandler for EscrowReleased {
	type Event = IntentGatewayV3::EscrowReleased;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let scope = Scope::new(ctx);
		let solver = event.solver.to_string();
		scope
			.gateway()
			.record_escrow_release(
				&event.commitment.to_string(),
				Some(&solver),
				&gateway_tokens(&event.tokens),
				&scope.tx,
			)
			.await
	}
}

struct LegacyEscrowReleased;

#[async_trait]
impl EventHandler for LegacyEscrowReleased {
	type Event = LegacyEscrow::EscrowReleased;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let scope = Scope::new(ctx);
		let released = tokens(&event.tokens, |t| (t.token, t.amount));
		scope
			.gateway()
			.record_escrow_release(&event.commitment.to_string(), None, &released, &scope.tx)
			.await
	}
}

struct EscrowRefunded;

#[async_trait]
impl EventHandler for EscrowRefunded {
	type Event = IntentGatewayV3::EscrowRefunded;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let scope = Scope::new(ctx);
		let gateway = scope.gateway();
		let commitment = event.commitment.to_string();
		let refunded = gateway_tokens(&event.tokens);
		gateway.record_escrow_refund(&commitment, &refunded, &scope.tx).await?;
		// Cancelling a fully filled order refunds nothing and leaves its status alone.
		if refunded.iter().any(|t| !t.amount.is_zero()) {
			gateway
				.update_order_status(&commitment, OrderStatus::Refunded, &scope.tx, None)
				.await?;
		}
		Ok(())
	}
}

struct ProtocolFeeRefunded;

#[async_trait]
impl EventHandler for ProtocolFeeRefunded {
	type Event = IntentGatewayV3::ProtocolFeeRefunded;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let scope = Scope::new(ctx);
		scope
			.gateway()
			.record_protocol_fee_refund(
				&event.commitment.to_string(),
				&event.token.to_string(),
				event.amount,
				&scope.tx,
			)
			.await
	}
}

struct OrderCancelled;

#[async_trait]
impl EventHandler for OrderCancelled {
	type Event = IntentGatewayV3::OrderCancelled;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let scope = Scope::new(ctx);
		scope
			.gateway()
			.record_order_cancellation(
				&event.commitment.to_string(),
				&event.canceller.to_string(),
				&scope.tx,
			)
			.await
	}
}

async fn dust(
	ctx: &EventContext<'_>,
	kind: Dust,
	token: Address,
	amount: alloy::primitives::U256,
) -> anyhow::Result<()> {
	let symbol = dust::dust_symbol(ctx.chain, token, ctx.log.block_number).await?;
	let chain = ctx.chain.state_machine_id();
	dust::record_dust(ctx.db, kind, chain, token, amount, ctx.log.block_timestamp, &symbol).await
}

struct DustCollected;

#[async_trait]
impl EventHandler for DustCollected {
	type Event = IntentGatewayV3::DustCollected;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		dust(ctx, Dust::Collected, event.token, event.amount).await
	}
}

struct DustSwept;

#[async_trait]
impl EventHandler for DustSwept {
	type Event = IntentGatewayV3::DustSwept;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		dust(ctx, Dust::Swept, event.token, event.amount).await
	}
}

struct BidPlaced;

#[async_trait]
impl EventHandler for BidPlaced {
	type Event = IntentsCoprocessor::BidPlaced;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		let log = ctx.log;
		let id = bids::bid_id(&event.commitment, &event.filler, log.block_number, log.log_index);
		if ctx
			.db
			.query_opt("SELECT 1 FROM app.filler_bids WHERE id = $1", &[&id])
			.await?
			.is_some()
		{
			return Ok(());
		}
		let source =
			bids::resolve_bid_source(ctx.chain, log, &event.filler, &event.commitment, &event.bid)
				.await?;
		ctx.db
			.execute(
				"INSERT INTO app.filler_bids (id, commitment, filler, bid, bid_data, extrinsic_hash, block_number)
				 VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (id) DO NOTHING",
				&[
					&id,
					&event.commitment.to_string(),
					&crate::substrate::ss58(&event.filler.0, bids::SS58_PREFIX),
					&event.bid.to_string(),
					&source.bid_data,
					&source.extrinsic_hash,
					&numeric(log.block_number),
				],
			)
			.await?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use alloy::sol_types::SolEvent;

	use super::*;

	#[test]
	fn legacy_escrow_released_topic() {
		assert_eq!(
			LegacyEscrow::EscrowReleased::SIGNATURE_HASH.to_string(),
			"0x1c1f5b2e9173d11729cff59e7b4113dbc84cbb3f662fee1896e2d89e97f1236c"
		);
		assert_ne!(
			LegacyEscrow::EscrowReleased::SIGNATURE_HASH,
			IntentGatewayV3::EscrowReleased::SIGNATURE_HASH
		);
		assert_eq!(
			IntentGatewayV3Legacy::OrderPlaced::SIGNATURE_HASH.to_string(),
			"0x34509e625ac0d72654af70cb0b2d47eb5236c1370cf0d7ad459828d4d3a5cf1d"
		);
		assert_eq!(
			IntentGatewayV3::OrderPlaced::SIGNATURE_HASH.to_string(),
			"0x76dc946fbf27feefcbafcf57450e0a23d7ccec958f32382180886cada274f6fa"
		);
		assert_eq!(
			IntentsCoprocessor::BidPlaced::SIGNATURE_HASH.to_string(),
			"0xe748081ee84c233cccf40e896838190e251a6365294df61dfe135c4de730042a"
		);
	}

	#[test]
	fn graffiti_equal_to_the_user_is_no_referrer() {
		let user = B256::repeat_byte(0x11);
		assert_eq!(resolve_graffiti(user, user), DEFAULT_REFERRER);
		assert_eq!(resolve_graffiti(B256::ZERO, user), DEFAULT_REFERRER);
		let tag = B256::repeat_byte(0x48);
		assert_eq!(resolve_graffiti(tag, user), tag.to_string());
	}
}
