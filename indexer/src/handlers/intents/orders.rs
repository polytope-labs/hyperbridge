//! `IntentGatewayV3Service`: orders, their status, fills, escrow releases and refunds, and the
//! points, volume and user activity they earn.

use std::str::FromStr;

use alloy::primitives::{Address, B256, U256};
use async_trait::async_trait;
use tokio_postgres::Transaction;

use super::calls::{Order, TokenAmount};
use crate::{
	errors::is_transient,
	services::{
		decimal::{amount_value_in_usd, to_scaled_usd, Dec},
		pending::{defer_status, flush_commitment, PendingEntity, PendingStatus},
		points::{award_points, Award},
		volume::update_volume,
	},
	store::{
		numeric, timestamp_to_date, Numeric, OrderStatus, PointsActivityType,
		ProtocolParticipantType,
	},
};

/** The referrer of an order placed without graffiti: 32 zero bytes. */
pub const DEFAULT_REFERRER: &str =
	"0x0000000000000000000000000000000000000000000000000000000000000000";
const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
const ENTITY: PendingEntity = PendingEntity::Order;

/** Token reads and prices, behind a trait so the order logic can be tested without a chain. */
#[async_trait]
pub trait Market: Send + Sync {
	/** `symbol()` and `decimals()` of a token on the chain being indexed, at the event's block. */
	async fn token_metadata(&self, token: Address) -> anyhow::Result<(String, u8)>;
	/** `fetchTokenUsdPrice`. */
	async fn usd_price(&self, symbol: &str) -> Option<Dec>;
}

/** Where a log sits, as the service records it. */
#[derive(Debug, Clone)]
pub struct TxMeta {
	pub transaction_hash: String,
	pub block_number: u64,
	/** Seconds. */
	pub timestamp: u64,
	pub log_index: u64,
}

impl TxMeta {
	fn event_id(&self) -> String {
		format!("{}.{}", self.transaction_hash, self.log_index)
	}
}

/** The host's fee token, read at the placement's block. */
pub struct FeeToken {
	/** Lowercase. */
	pub address: String,
	pub decimals: u8,
}

/** What the fill transaction's receipt adds to a fill. */
#[derive(Debug, Default, Clone)]
pub struct FillReceipt {
	pub user_op_hash: Option<String>,
	/** Aligned with the fill's outputs. */
	pub amounts_received: Option<Vec<Option<U256>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillKind {
	/** `OrderFilled`. */
	Full,
	/** `PartialFill`. */
	Partial,
}

impl FillKind {
	fn table(&self) -> &'static str {
		match self {
			FillKind::Full => "i_order_v3_fills",
			FillKind::Partial => "i_order_v3_partial_fills",
		}
	}

	fn inputs_table(&self) -> &'static str {
		match self {
			FillKind::Full => "i_order_v3_fill_input_assets",
			FillKind::Partial => "i_order_v3_partial_fill_input_assets",
		}
	}

	fn outputs_table(&self) -> &'static str {
		match self {
			FillKind::Full => "i_order_v3_fill_output_assets",
			FillKind::Partial => "i_order_v3_partial_fill_output_assets",
		}
	}

	fn parent_column(&self) -> &'static str {
		match self {
			FillKind::Full => "fill_id",
			FillKind::Partial => "partial_fill_id",
		}
	}
}

/** `"0x"` + lowercase hex. */
pub fn lower_hex(bytes: impl AsRef<[u8]>) -> String {
	format!("0x{}", hex::encode(bytes))
}

/** `bytes32ToBytes20`: a 20-byte value is returned as is, otherwise its last 20 bytes,
 * lowercase. */
pub fn bytes32_to_bytes20(value: &str) -> String {
	if value.len() == 42 {
		return value.to_string();
	}
	let bytes = hex::decode(value.trim_start_matches("0x")).unwrap_or_default();
	lower_hex(&bytes[bytes.len().saturating_sub(20)..])
}

/** `decodeChain`: chain ids stored as hex bytes read back as text. */
pub fn decode_chain(value: &str) -> String {
	match value.strip_prefix("0x") {
		Some(hex) => String::from_utf8_lossy(&hex::decode(hex).unwrap_or_default()).into_owned(),
		None => value.to_string(),
	}
}

/** The whole dollars of a non-negative `toFixed` amount: `truncated()` and `floor()`. */
fn whole_dollars(usd: &str) -> U256 {
	U256::from_str(usd.split('.').next().unwrap_or("0")).unwrap_or_default()
}

fn parse_u256(value: &Numeric) -> anyhow::Result<U256> {
	U256::from_str(value.as_str()).map_err(|e| anyhow::anyhow!("{value}: {e}"))
}

/** The order service for one chain, inside its batch's transaction. */
pub struct Gateway<'a> {
	pub db: &'a Transaction<'a>,
	/** `chainId` as SubQuery wrote it to `chain` columns: the decimal EVM chain id. */
	pub chain: &'a str,
	/** The chain's state machine id, e.g. `EVM-56`. */
	pub state_machine: &'a str,
	pub market: &'a dyn Market,
}

impl Gateway<'_> {
	/**
	 * Serialises work on one order across chains: a fill indexed on the destination while the
	 * placement is indexed on the source must not miss or double-count the early-fill backfill.
	 */
	async fn lock(&self, commitment: &str) -> anyhow::Result<()> {
		self.db
			.execute("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))", &[&commitment])
			.await?;
		Ok(())
	}

	/** `getTokenMetadata`: the zero address is ETH with 18 decimals on every chain. */
	async fn metadata(&self, token: Address) -> anyhow::Result<(String, u8)> {
		if token == Address::ZERO {
			return Ok(("ETH".into(), 18));
		}
		self.market.token_metadata(token).await
	}

	/**
	 * `getTokenValuesUSD`: the tokens' total USD value, `toFixed(18)`. A token that doesn't
	 * answer `symbol()` and `decimals()` (no code, a bytes32 symbol) is worth 0. SubQuery
	 * failed the block on it, which under rindexer's retries would stop the chain for good.
	 */
	pub async fn token_values_usd(&self, tokens: &[TokenAmount]) -> anyhow::Result<String> {
		let mut total = Dec::zero();
		for token in tokens {
			let (symbol, decimals) = match self.metadata(token.address()).await {
				Ok(metadata) => metadata,
				Err(err) if is_transient(&err) => return Err(err),
				Err(err) => {
					tracing::warn!(token = %token.address(), "unreadable token valued at 0: {err:#}");
					continue;
				},
			};
			let price = self.market.usd_price(&symbol).await.unwrap_or_else(Dec::zero);
			total = total.add(&Dec::parse(&amount_value_in_usd(token.amount, decimals, &price))?);
		}
		Ok(total.to_fixed(18))
	}

	/** `getOrCreateOrder`. `order.user` is the bytes32 from the log. */
	#[allow(clippy::too_many_arguments)]
	pub async fn get_or_create_order(
		&self,
		order: &Order,
		commitment: &str,
		referrer: &str,
		fee_token: Option<&FeeToken>,
		user_op_hash: Option<&str>,
		tx: &TxMeta,
	) -> anyhow::Result<()> {
		self.lock(commitment).await?;
		let existing: Option<OrderStatus> = self
			.db
			.query_opt(
				"SELECT status FROM app.i_order_v3s WHERE id = $1 FOR UPDATE",
				&[&commitment],
			)
			.await?
			.map(|row| row.get(0));
		let input_usd = self.token_values_usd(&order.inputs).await?;
		let user = bytes32_to_bytes20(&order.user.to_string());
		let session = order.session.to_string();
		let fee_address = fee_token.map(|f| f.address.clone());
		let fee_decimals = fee_token.map(|f| f.decimals as i32);
		let whole_usd = numeric(whole_dollars(&input_usd));
		let predispatch_call = order.predispatch_call.to_string();
		let post_dispatch_call = order.output_call.to_string();

		match existing {
			None => {
				self.db
					.execute(
						"INSERT INTO app.i_order_v3s
						   (id, \"user\", source_chain, dest_chain, commitment, deadline, nonce, fees, fee_token,
						    fee_token_decimals, session, input_u_s_d, status, predispatch_calldata,
						    post_dispatch_calldata, referrer, created_at, block_number, block_timestamp,
						    transaction_hash, user_op_hash)
						 VALUES ($1, $2, $3, $4, $1, $5, $6, $7, $8, $9, $10, $11, 'PLACED', $12, $13, $14, $15,
						    $16, $17, $18, $19)",
						&[
							&commitment,
							&user,
							&order.source,
							&order.destination,
							&numeric(order.deadline),
							&numeric(order.nonce),
							&numeric(order.fees),
							&fee_address,
							&fee_decimals,
							&session,
							&whole_usd,
							&predispatch_call,
							&post_dispatch_call,
							&referrer,
							&timestamp_to_date(tx.timestamp),
							&numeric(tx.block_number),
							&numeric(tx.timestamp),
							&tx.transaction_hash,
							&user_op_hash,
						],
					)
					.await?;
				self.save_assets(order, commitment).await?;
				flush_commitment(self.db, ENTITY, commitment).await?;
				self.backfill_early_fills(commitment).await?;
				self.award_placement(order, commitment, &user, &input_usd, tx).await?;

				let activity =
					self.get_or_create_user(&user, Some(referrer), Some(tx.timestamp)).await?;
				let placed = Dec::parse(&activity.placed_volume)?.add(&Dec::parse(&input_usd)?);
				self.db
					.execute(
						"UPDATE app.user_activity_v2s SET total_orders_placed = total_orders_placed + 1,
						   total_order_placed_volume_u_s_d = $2 WHERE id = $1",
						&[&user, &placed.to_string()],
					)
					.await?;
			},
			Some(status) => {
				self.db
					.execute(
						"UPDATE app.i_order_v3s SET \"user\" = $2, source_chain = $3, dest_chain = $4,
						   deadline = $5, nonce = $6, fees = $7,
						   fee_token = CASE WHEN $8 THEN $9 ELSE fee_token END,
						   fee_token_decimals = CASE WHEN $8 THEN $10 ELSE fee_token_decimals END,
						   user_op_hash = COALESCE($11, user_op_hash), session = $12, input_u_s_d = $13,
						   predispatch_calldata = $14, post_dispatch_calldata = $15, referrer = $16
						 WHERE id = $1",
						&[
							&commitment,
							&user,
							&order.source,
							&order.destination,
							&numeric(order.deadline),
							&numeric(order.nonce),
							&numeric(order.fees),
							&fee_token.is_some(),
							&fee_address,
							&fee_decimals,
							&user_op_hash,
							&session,
							&whole_usd,
							&predispatch_call,
							&post_dispatch_call,
							&referrer,
						],
					)
					.await?;
				self.save_assets(order, commitment).await?;
				if status != OrderStatus::Filled {
					self.award_placement(order, commitment, &user, &input_usd, tx).await?;
				}
			},
		}
		Ok(())
	}

	/** USER points and volume for a placement. */
	async fn award_placement(
		&self,
		order: &Order,
		commitment: &str,
		user: &str,
		input_usd: &str,
		tx: &TxMeta,
	) -> anyhow::Result<()> {
		let description =
			format!("Points awarded for placing orderV3 {commitment} with value {input_usd} USD");
		award_points(
			self.db,
			Award {
				address: user,
				chain: &decode_chain(&order.source),
				points: whole_dollars(input_usd),
				earner: ProtocolParticipantType::User,
				activity: PointsActivityType::OrderPlacedPoints,
				transaction_hash: &tx.transaction_hash,
				description: &description,
				timestamp: tx.timestamp,
			},
		)
		.await?;
		update_volume(self.db, "IntentGatewayV3.USER", self.state_machine, input_usd, tx.timestamp)
			.await
	}

	/** The order's predispatch, input and output asset rows. Updates keep `released` and
	 * `filled`. */
	async fn save_assets(&self, order: &Order, commitment: &str) -> anyhow::Result<()> {
		for (index, asset) in order.predispatch.iter().enumerate() {
			self.db
				.execute(
					"INSERT INTO app.i_order_v3_predispatch_assets (id, order_id, token, amount, index)
					 VALUES ($1, $2, $3, $4, $5)
					 ON CONFLICT (id) DO UPDATE SET token = EXCLUDED.token, amount = EXCLUDED.amount,
					   index = EXCLUDED.index",
					&[
						&format!("{commitment}-predispatch-{index}"),
						&commitment,
						&asset.token.to_string(),
						&numeric(asset.amount),
						&(index as i32),
					],
				)
				.await?;
		}
		for (index, asset) in order.inputs.iter().enumerate() {
			self.db
				.execute(
					"INSERT INTO app.i_order_v3_input_assets (id, order_id, token, amount, index)
					 VALUES ($1, $2, $3, $4, $5)
					 ON CONFLICT (id) DO UPDATE SET token = EXCLUDED.token, amount = EXCLUDED.amount,
					   index = EXCLUDED.index",
					&[
						&format!("{commitment}-input-{index}"),
						&commitment,
						&asset.token.to_string(),
						&numeric(asset.amount),
						&(index as i32),
					],
				)
				.await?;
		}
		let beneficiary = order.beneficiary.to_string();
		for (index, asset) in order.outputs.iter().enumerate() {
			self.db
				.execute(
					"INSERT INTO app.i_order_v3_output_assets (id, order_id, token, amount, index, beneficiary)
					 VALUES ($1, $2, $3, $4, $5, $6)
					 ON CONFLICT (id) DO UPDATE SET token = EXCLUDED.token, amount = EXCLUDED.amount,
					   index = EXCLUDED.index, beneficiary = EXCLUDED.beneficiary",
					&[
						&format!("{commitment}-output-{index}"),
						&commitment,
						&asset.token.to_string(),
						&numeric(asset.amount),
						&(index as i32),
						&beneficiary,
					],
				)
				.await?;
		}
		Ok(())
	}

	/** `updateOrderStatus`. A missing order parks the status in `pending_status_metadata`. */
	pub async fn update_order_status(
		&self,
		commitment: &str,
		status: OrderStatus,
		tx: &TxMeta,
		filler: Option<&str>,
	) -> anyhow::Result<()> {
		self.lock(commitment).await?;
		let row = self
			.db
			.query_opt(
				"SELECT \"user\", referrer, input_u_s_d, source_chain FROM app.i_order_v3s
				 WHERE id = $1 FOR UPDATE",
				&[&commitment],
			)
			.await?;
		let Some(row) = row else {
			tracing::warn!(commitment, "order not indexed yet, deferring {}", status.as_str());
			return defer_status(
				self.db,
				&PendingStatus {
					commitment,
					entity: ENTITY,
					status: status.as_str(),
					chain: self.chain,
					timestamp: numeric(tx.timestamp),
					block_number: tx.block_number.to_string(),
					block_hash: "",
					transaction_hash: &tx.transaction_hash,
					filler,
					created_at: timestamp_to_date(tx.timestamp),
				},
			)
			.await;
		};
		if status != OrderStatus::Placed {
			self.db
				.execute(
					"UPDATE app.i_order_v3s SET status = $2 WHERE id = $1",
					&[&commitment, &status],
				)
				.await?;
		}

		if status == OrderStatus::Filled && filler.is_some_and(|f| !f.is_empty()) {
			let user: String = row.get(0);
			let referrer: Option<String> = row.get(1);
			let input_usd: Numeric = row.get(2);
			let source_chain: String = row.get(3);
			let points = whole_dollars(input_usd.as_str());

			let user = bytes32_to_bytes20(&user);
			let activity = self.get_or_create_user(&user, referrer.as_deref(), None).await?;
			let filled = Dec::parse(&activity.filled_volume)?.add(&Dec::parse(input_usd.as_str())?);
			self.db
				.execute(
					"UPDATE app.user_activity_v2s SET total_filled_orders = total_filled_orders + 1,
					   total_order_filled_volume_u_s_d = $2 WHERE id = $1",
					&[&user, &filled.to_string()],
				)
				.await?;

			// The user's first referrer, which may be the zero referrer.
			if let Some(referrer) = activity.referrer.filter(|r| !r.is_empty()) {
				let description = format!(
					"Points awarded for filling orderV3 {commitment} with value {input_usd} USD"
				);
				award_points(
					self.db,
					Award {
						address: &referrer,
						chain: &decode_chain(&source_chain),
						points: points / U256::from(2),
						earner: ProtocolParticipantType::Referrer,
						activity: PointsActivityType::OrderReferredPoints,
						transaction_hash: &tx.transaction_hash,
						description: &description,
						timestamp: tx.timestamp,
					},
				)
				.await?;
			}
		}

		self.write_status(commitment, status, tx, filler).await
	}

	/** `IOrderV3StatusMetadata` `<commitment>.<status>`, overwritten by a later one. */
	async fn write_status(
		&self,
		commitment: &str,
		status: OrderStatus,
		tx: &TxMeta,
		filler: Option<&str>,
	) -> anyhow::Result<()> {
		self.db
			.execute(
				"INSERT INTO app.i_order_v3_status_metadata
				   (id, status, chain, timestamp, block_number, transaction_hash, order_id, filler, created_at)
				 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
				 ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status, chain = EXCLUDED.chain,
				   timestamp = EXCLUDED.timestamp, block_number = EXCLUDED.block_number,
				   transaction_hash = EXCLUDED.transaction_hash, order_id = EXCLUDED.order_id,
				   filler = EXCLUDED.filler, created_at = EXCLUDED.created_at",
				&[
					&format!("{commitment}.{}", status.as_str()),
					&status,
					&self.chain,
					&numeric(tx.timestamp),
					&tx.block_number.to_string(),
					&tx.transaction_hash,
					&commitment,
					&filler,
					&timestamp_to_date(tx.timestamp),
				],
			)
			.await?;
		Ok(())
	}

	/** `recordFill` and `recordPartialFill`. The fill row marks the log as applied. */
	#[allow(clippy::too_many_arguments)]
	pub async fn record_fill(
		&self,
		kind: FillKind,
		commitment: &str,
		filler: &str,
		outputs: &[TokenAmount],
		inputs: &[TokenAmount],
		tx: &TxMeta,
		receipt: &FillReceipt,
	) -> anyhow::Result<()> {
		self.lock(commitment).await?;
		if !self.order_exists(commitment).await? {
			tracing::warn!(commitment, "order not indexed yet, recording the fill by commitment");
		}
		let fill_id = tx.event_id();
		let table = kind.table();
		let parent = kind.parent_column();
		if self
			.db
			.query_opt(&format!("SELECT 1 FROM app.{table} WHERE id = $1"), &[&fill_id])
			.await?
			.is_some()
		{
			return Ok(());
		}
		self.db
			.execute(
				&format!(
					"INSERT INTO app.{table}
					   (id, order_id, chain, filler, timestamp, block_number, transaction_hash, user_op_hash, created_at)
					 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
				),
				&[
					&fill_id,
					&commitment,
					&self.chain,
					&filler,
					&numeric(tx.timestamp),
					&tx.block_number.to_string(),
					&tx.transaction_hash,
					&receipt.user_op_hash,
					&timestamp_to_date(tx.timestamp),
				],
			)
			.await?;
		for (index, input) in inputs.iter().enumerate() {
			self.db
				.execute(
					&format!(
						"INSERT INTO app.{} (id, {parent}, token, amount, index) VALUES ($1, $2, $3, $4, $5)
						 ON CONFLICT (id) DO NOTHING",
						kind.inputs_table()
					),
					&[
						&format!("{fill_id}-input-{index}"),
						&fill_id,
						&input.token.to_string(),
						&numeric(input.amount),
						&(index as i32),
					],
				)
				.await?;
		}
		for (index, output) in outputs.iter().enumerate() {
			let received = receipt
				.amounts_received
				.as_ref()
				.and_then(|amounts| amounts.get(index).copied().flatten())
				.map(numeric);
			let id = format!("{fill_id}-output-{index}");
			let token = output.token.to_string();
			let amount = numeric(output.amount);
			let index = index as i32;
			match kind {
				FillKind::Full => {
					self.db
						.execute(
							"INSERT INTO app.i_order_v3_fill_output_assets
							   (id, fill_id, token, amount, amount_received, index)
							 VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO NOTHING",
							&[&id, &fill_id, &token, &amount, &received, &index],
						)
						.await?;
				},
				FillKind::Partial => {
					// The order's beneficiary when it is indexed, else a 20-byte zero address.
					let beneficiary: String = self
						.db
						.query_opt(
							"SELECT beneficiary FROM app.i_order_v3_output_assets WHERE id = $1",
							&[&format!("{commitment}-output-{index}")],
						)
						.await?
						.map(|row| row.get(0))
						.unwrap_or_else(|| ZERO_ADDRESS.to_string());
					self.db
						.execute(
							"INSERT INTO app.i_order_v3_partial_fill_output_assets
							   (id, partial_fill_id, token, amount, amount_received, index, beneficiary)
							 VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (id) DO NOTHING",
							&[&id, &fill_id, &token, &amount, &received, &index, &beneficiary],
						)
						.await?;
				},
			}
		}
		self.accumulate_filled(commitment, outputs).await?;
		self.award_fill_rewards(commitment, filler, outputs, tx).await
	}

	async fn order_exists(&self, commitment: &str) -> anyhow::Result<bool> {
		Ok(self
			.db
			.query_opt("SELECT 1 FROM app.i_order_v3s WHERE id = $1", &[&commitment])
			.await?
			.is_some())
	}

	/** `accumulateFilled`: adds each non-zero output to the order's matching output asset. */
	async fn accumulate_filled(
		&self,
		commitment: &str,
		outputs: &[TokenAmount],
	) -> anyhow::Result<()> {
		for (index, output) in outputs.iter().enumerate() {
			if output.amount.is_zero() {
				continue;
			}
			let updated = self
				.db
				.execute(
					"UPDATE app.i_order_v3_output_assets SET filled = COALESCE(filled, 0) + $2
					 WHERE id = $1 AND lower(token) = $3",
					&[
						&format!("{commitment}-output-{index}"),
						&numeric(output.amount),
						&output.token.to_string(),
					],
				)
				.await?;
			if updated == 0 {
				tracing::warn!(
					commitment,
					index,
					"output asset missing or token mismatch, not accumulating"
				);
			}
		}
		Ok(())
	}

	/** `awardFillRewards`: the filler's volume and points for one fill slice. */
	async fn award_fill_rewards(
		&self,
		commitment: &str,
		filler: &str,
		outputs: &[TokenAmount],
		tx: &TxMeta,
	) -> anyhow::Result<()> {
		let (transaction_hash, timestamp) = (tx.transaction_hash.as_str(), tx.timestamp);
		let provided: Vec<TokenAmount> =
			outputs.iter().filter(|o| !o.amount.is_zero()).copied().collect();
		if provided.is_empty() {
			return Ok(());
		}
		let total = self.token_values_usd(&provided).await?;
		if !Dec::parse(&total)?.is_positive() {
			return Ok(());
		}
		// Seed before this fill's own update, so the seed never counts it.
		self.seed_aggregate_volume("IntentGatewayV3.FILLED", "IntentGatewayV3.FILLER.")
			.await?;
		update_volume(
			self.db,
			&format!("IntentGatewayV3.FILLER.{filler}"),
			self.state_machine,
			&total,
			timestamp,
		)
		.await?;
		update_volume(self.db, "IntentGatewayV3.FILLED", self.state_machine, &total, timestamp)
			.await?;
		if self
			.award_fill_points(commitment, filler, &total, transaction_hash, timestamp)
			.await?
		{
			return Ok(());
		}
		// The order isn't indexed yet. Keep this chain's valuation for the placement's backfill,
		// which runs on the source chain and can't read these tokens there.
		self.db
			.execute(
				"INSERT INTO indexer.deferred_fill_points
				   (id, commitment, filler, usd, transaction_hash, timestamp)
				 VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO NOTHING",
				&[
					&tx.event_id(),
					&commitment,
					&filler,
					&total,
					&transaction_hash,
					&(timestamp as i64),
				],
			)
			.await?;
		Ok(())
	}

	/** `awardFillPoints`. Returns false, awarding nothing, while the order is missing. */
	async fn award_fill_points(
		&self,
		commitment: &str,
		filler: &str,
		slice_usd: &str,
		transaction_hash: &str,
		timestamp: u64,
	) -> anyhow::Result<bool> {
		let Some(row) = self
			.db
			.query_opt("SELECT dest_chain FROM app.i_order_v3s WHERE id = $1", &[&commitment])
			.await?
		else {
			return Ok(false);
		};
		let points = whole_dollars(slice_usd);
		if points.is_zero() {
			return Ok(true);
		}
		let dest_chain: String = row.get(0);
		let description = format!(
			"Points awarded for filling orderV3 {commitment} slice worth {} USD",
			Dec::parse(slice_usd)?
		);
		award_points(
			self.db,
			Award {
				address: filler,
				chain: &decode_chain(&dest_chain),
				points,
				earner: ProtocolParticipantType::Filler,
				activity: PointsActivityType::OrderFilledPoints,
				transaction_hash,
				description: &description,
				timestamp,
			},
		)
		.await?;
		Ok(true)
	}

	/**
	 * `backfillEarlyFills`: fills recorded before the order (the destination ran ahead) get the
	 * `filled` accounting and points they skipped. Their volume was credited when they were
	 * indexed, and their points use the valuation the destination chain made then. SubQuery
	 * re-read the destination's tokens on this, the source, chain, which failed the block when
	 * a token had no code here.
	 */
	async fn backfill_early_fills(&self, commitment: &str) -> anyhow::Result<()> {
		for kind in [FillKind::Partial, FillKind::Full] {
			let fills = self
				.db
				.query(
					&format!(
						"SELECT id, filler, transaction_hash, timestamp FROM app.{} WHERE order_id = $1
						 ORDER BY id LIMIT 100",
						kind.table()
					),
					&[&commitment],
				)
				.await?;
			for fill in fills {
				let fill_id: String = fill.get(0);
				let mut outputs = vec![];
				loop {
					let Some(asset) = self
						.db
						.query_opt(
							&format!(
								"SELECT token, amount FROM app.{} WHERE id = $1",
								kind.outputs_table()
							),
							&[&format!("{fill_id}-output-{}", outputs.len())],
						)
						.await?
					else {
						break;
					};
					let token: String = asset.get(0);
					let amount: Numeric = asset.get(1);
					outputs.push(TokenAmount {
						token: B256::from_str(&token)?,
						amount: parse_u256(&amount)?,
					});
				}
				let provided: Vec<TokenAmount> =
					outputs.iter().filter(|o| !o.amount.is_zero()).copied().collect();
				if provided.is_empty() {
					continue;
				}
				self.accumulate_filled(commitment, &outputs).await?;
			}
		}
		let deferred = self
			.db
			.query(
				"DELETE FROM indexer.deferred_fill_points WHERE commitment = $1
				 RETURNING id, filler, usd, transaction_hash, timestamp",
				&[&commitment],
			)
			.await?;
		let mut deferred: Vec<_> = deferred.into_iter().collect();
		deferred.sort_by_key(|row| row.get::<_, String>(0));
		for row in deferred {
			let (filler, usd, transaction_hash): (String, String, String) =
				(row.get(1), row.get(2), row.get(3));
			let timestamp: i64 = row.get(4);
			self.award_fill_points(commitment, &filler, &usd, &transaction_hash, timestamp as u64)
				.await?;
		}
		Ok(())
	}

	/**
	 * `seedAggregateVolume`: the first time `<aggregate>.<chain>` is needed, build it and its
	 * daily rows from the component rows `<prefix>*.<chain>.<day>` already written.
	 */
	async fn seed_aggregate_volume(&self, aggregate: &str, prefix: &str) -> anyhow::Result<()> {
		let id = format!("{aggregate}.{}", self.state_machine);
		if self
			.db
			.query_opt("SELECT 1 FROM app.cumulative_volume_u_s_ds WHERE id = $1", &[&id])
			.await?
			.is_some()
		{
			return Ok(());
		}
		let suffix = format!(".{}", self.state_machine);
		self.db
			.execute(
				"WITH days AS (
				   SELECT right(id, 10) AS day, sum(last24_hours_volume_u_s_d) AS volume,
				     max(last_updated_at) AS last_updated_at
				   FROM app.daily_volume_u_s_ds
				   WHERE starts_with(id, $2::text) AND right(left(id, -11), length($3::text)) = $3::text
				     AND right(id, 10) ~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$'
				   GROUP BY 1),
				 daily AS (
				   INSERT INTO app.daily_volume_u_s_ds (id, last_updated_at, created_at, last24_hours_volume_u_s_d)
				   SELECT $1::text || '.' || day, last_updated_at, day::date::timestamp, volume FROM days
				   ON CONFLICT (id) DO UPDATE SET last_updated_at = EXCLUDED.last_updated_at,
				     created_at = EXCLUDED.created_at,
				     last24_hours_volume_u_s_d = EXCLUDED.last24_hours_volume_u_s_d)
				 INSERT INTO app.cumulative_volume_u_s_ds (id, last_updated_at, volume_u_s_d)
				 SELECT $1, COALESCE(max(last_updated_at), 0), COALESCE(sum(volume), 0) FROM days",
				&[&id, &prefix, &suffix],
			)
			.await?;
		Ok(())
	}

	/** `recordEscrowRelease`: accumulates `released`, and REDEEMED once every input is released. */
	pub async fn record_escrow_release(
		&self,
		commitment: &str,
		solver: Option<&str>,
		tokens: &[TokenAmount],
		tx: &TxMeta,
	) -> anyhow::Result<()> {
		self.lock(commitment).await?;
		let release_id = tx.event_id();
		if self
			.db
			.query_opt("SELECT 1 FROM app.i_order_v3_escrow_releases WHERE id = $1", &[&release_id])
			.await?
			.is_some()
		{
			return Ok(());
		}
		self.db
			.execute(
				"INSERT INTO app.i_order_v3_escrow_releases
				   (id, order_id, solver, chain, timestamp, block_number, transaction_hash, created_at)
				 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
				&[
					&release_id,
					&commitment,
					&solver,
					&self.chain,
					&numeric(tx.timestamp),
					&tx.block_number.to_string(),
					&tx.transaction_hash,
					&timestamp_to_date(tx.timestamp),
				],
			)
			.await?;
		for (index, token) in tokens.iter().enumerate() {
			self.db
				.execute(
					"INSERT INTO app.i_order_v3_escrow_release_tokens (id, release_id, token, amount, index)
					 VALUES ($1, $2, $3, $4, $5)
					 ON CONFLICT (id) DO UPDATE SET release_id = EXCLUDED.release_id,
					   token = EXCLUDED.token, amount = EXCLUDED.amount, index = EXCLUDED.index",
					&[
						&format!("{release_id}-token-{index}"),
						&release_id,
						&token.token.to_string(),
						&numeric(token.amount),
						&(index as i32),
					],
				)
				.await?;
		}

		let mut inputs: Vec<(String, String, U256, Option<U256>)> = vec![];
		loop {
			let id = format!("{commitment}-input-{}", inputs.len());
			let Some(row) = self
				.db
				.query_opt(
					"SELECT token, amount, released FROM app.i_order_v3_input_assets WHERE id = $1 FOR UPDATE",
					&[&id],
				)
				.await?
			else {
				break;
			};
			let amount: Numeric = row.get(1);
			let released: Option<Numeric> = row.get(2);
			inputs.push((
				id,
				row.get(0),
				parse_u256(&amount)?,
				released.as_ref().map(parse_u256).transpose()?,
			));
		}
		if inputs.is_empty() {
			tracing::warn!(
				commitment,
				"order has no input assets yet, not accumulating the release"
			);
			return Ok(());
		}
		for (index, token) in tokens.iter().enumerate() {
			if token.amount.is_zero() {
				continue;
			}
			match inputs.get_mut(index) {
				Some((id, asset_token, _, released))
					if asset_token.to_lowercase() == token.token.to_string() =>
				{
					let total = released.unwrap_or_default() + token.amount;
					*released = Some(total);
					self.db
						.execute(
							"UPDATE app.i_order_v3_input_assets SET released = $2 WHERE id = $1",
							&[&*id, &numeric(total)],
						)
						.await?;
				},
				_ => tracing::warn!(
					commitment,
					index,
					"input asset missing or token mismatch, not accumulating"
				),
			}
		}
		if inputs
			.iter()
			.all(|(_, _, amount, released)| released.unwrap_or_default() >= *amount)
		{
			self.update_order_status(commitment, OrderStatus::Redeemed, tx, solver).await?;
		}
		Ok(())
	}

	/** `recordEscrowRefund`. The first write of each row is kept. */
	pub async fn record_escrow_refund(
		&self,
		commitment: &str,
		tokens: &[TokenAmount],
		tx: &TxMeta,
	) -> anyhow::Result<()> {
		let refund_id = tx.event_id();
		self.db
			.execute(
				"INSERT INTO app.i_order_v3_escrow_refunds
				   (id, order_id, chain, timestamp, block_number, transaction_hash, created_at)
				 VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (id) DO NOTHING",
				&[
					&refund_id,
					&commitment,
					&self.chain,
					&numeric(tx.timestamp),
					&tx.block_number.to_string(),
					&tx.transaction_hash,
					&timestamp_to_date(tx.timestamp),
				],
			)
			.await?;
		for (index, token) in tokens.iter().enumerate() {
			self.db
				.execute(
					"INSERT INTO app.i_order_v3_escrow_refund_tokens (id, refund_id, token, amount, index)
					 VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO NOTHING",
					&[
						&format!("{refund_id}-token-{index}"),
						&refund_id,
						&token.token.to_string(),
						&numeric(token.amount),
						&(index as i32),
					],
				)
				.await?;
		}
		Ok(())
	}

	/** `recordProtocolFeeRefund`. Never reads the order; the first write is kept. */
	pub async fn record_protocol_fee_refund(
		&self,
		commitment: &str,
		token: &str,
		amount: U256,
		tx: &TxMeta,
	) -> anyhow::Result<()> {
		self.db
			.execute(
				"INSERT INTO app.i_order_v3_protocol_fee_refunds
				   (id, order_id, chain, token, amount, timestamp, block_number, transaction_hash, created_at)
				 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT (id) DO NOTHING",
				&[
					&tx.event_id(),
					&commitment,
					&self.chain,
					&token,
					&numeric(amount),
					&numeric(tx.timestamp),
					&tx.block_number.to_string(),
					&tx.transaction_hash,
					&timestamp_to_date(tx.timestamp),
				],
			)
			.await?;
		Ok(())
	}

	/**
	 * `recordOrderCancellation`: the cancellation row and a CANCELLED status, which waits in
	 * `pending_status_metadata` while the order is missing. The order row is never touched.
	 */
	pub async fn record_order_cancellation(
		&self,
		commitment: &str,
		canceller: &str,
		tx: &TxMeta,
	) -> anyhow::Result<()> {
		self.lock(commitment).await?;
		self.db
			.execute(
				"INSERT INTO app.i_order_v3_cancellations
				   (id, order_id, chain, canceller, timestamp, block_number, transaction_hash, created_at)
				 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT (id) DO NOTHING",
				&[
					&tx.event_id(),
					&commitment,
					&self.chain,
					&canceller,
					&numeric(tx.timestamp),
					&tx.block_number.to_string(),
					&tx.transaction_hash,
					&timestamp_to_date(tx.timestamp),
				],
			)
			.await?;
		if self.order_exists(commitment).await? {
			return self.write_status(commitment, OrderStatus::Cancelled, tx, None).await;
		}
		defer_status(
			self.db,
			&PendingStatus {
				commitment,
				entity: ENTITY,
				status: OrderStatus::Cancelled.as_str(),
				chain: self.chain,
				timestamp: numeric(tx.timestamp),
				block_number: tx.block_number.to_string(),
				block_hash: "",
				transaction_hash: &tx.transaction_hash,
				filler: None,
				created_at: timestamp_to_date(tx.timestamp),
			},
		)
		.await
	}

	/**
	 * `recordOrderVolume`: raw amounts per chain and token, and their USD total per chain. A
	 * token whose metadata can't be read skips the USD total, as the TypeScript's rejected
	 * `Promise.all` did, while the other tokens' amounts are still added.
	 */
	pub async fn record_order_volume(
		&self,
		volume_type: &str,
		tokens: &[TokenAmount],
		timestamp: u64,
	) -> anyhow::Result<()> {
		let mut by_token: Vec<(Address, U256)> = vec![];
		for token in tokens {
			let address = token.address();
			match by_token.iter_mut().find(|(a, _)| *a == address) {
				Some((_, amount)) => *amount += token.amount,
				None => by_token.push((address, token.amount)),
			}
		}
		let mut usd = Dec::zero();
		let mut failed = false;
		for (address, amount) in by_token {
			let token_address = lower_hex(address);
			let id = format!("{}-{token_address}-{volume_type}", self.state_machine);
			let existing = self
				.db
				.query_opt(
					"SELECT token_symbol, decimals FROM app.intent_gateway_token_volumes WHERE id = $1 FOR UPDATE",
					&[&id],
				)
				.await?;
			let (symbol, decimals) = match existing {
				Some(row) => {
					self.db
						.execute(
							"UPDATE app.intent_gateway_token_volumes SET amount = amount + $2, last_updated_at = $3
							 WHERE id = $1",
							&[&id, &numeric(amount), &numeric(timestamp)],
						)
						.await?;
					(row.get::<_, String>(0), row.get::<_, i32>(1) as u8)
				},
				None => {
					let (symbol, decimals) = match self.metadata(address).await {
						Ok(metadata) => metadata,
						Err(err) => {
							tracing::error!(
								"recording {volume_type} volume: {token_address} metadata failed: {err:#}"
							);
							failed = true;
							continue;
						},
					};
					self.db
						.execute(
							"INSERT INTO app.intent_gateway_token_volumes
							   (id, chain, token_address, token_symbol, decimals, volume_type, amount, last_updated_at)
							 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
							 ON CONFLICT (id) DO UPDATE SET amount = intent_gateway_token_volumes.amount + EXCLUDED.amount,
							   last_updated_at = EXCLUDED.last_updated_at",
							&[
								&id,
								&self.state_machine,
								&token_address,
								&symbol,
								&(decimals as i32),
								&volume_type,
								&numeric(amount),
								&numeric(timestamp),
							],
						)
						.await?;
					(symbol, decimals)
				},
			};
			match self.market.usd_price(&symbol).await {
				Some(price) =>
					usd = usd.add(&Dec::parse(&amount_value_in_usd(amount, decimals, &price))?),
				None => tracing::warn!(
					"no USD price for {symbol} ({token_address}) on {}; raw amount kept",
					self.state_machine
				),
			}
		}
		if failed || usd.is_zero() {
			return Ok(());
		}
		self.db
			.execute(
				"INSERT INTO app.cumulative_intent_gateway_volume_u_s_ds
				   (id, chain, volume_type, volume_u_s_d, last_updated_at)
				 VALUES ($1, $2, $3, $4, $5)
				 ON CONFLICT (id) DO UPDATE SET
				   volume_u_s_d = cumulative_intent_gateway_volume_u_s_ds.volume_u_s_d + EXCLUDED.volume_u_s_d,
				   last_updated_at = EXCLUDED.last_updated_at",
				&[
					&format!("{}-{volume_type}", self.state_machine),
					&self.state_machine,
					&volume_type,
					&to_scaled_usd(&usd.to_fixed(18))?,
					&numeric(timestamp),
				],
			)
			.await?;
		Ok(())
	}

	/** `getOrCreateUser`: the user's row, taking `referrer` only while the row has none. */
	async fn get_or_create_user(
		&self,
		address: &str,
		referrer: Option<&str>,
		timestamp: Option<u64>,
	) -> anyhow::Result<UserActivity> {
		self.db
			.execute(
				"INSERT INTO app.user_activity_v2s
				   (id, referrer, total_orders_placed, total_filled_orders, total_order_placed_volume_u_s_d,
				    total_order_filled_volume_u_s_d, created_at)
				 VALUES ($1, $2, 0, 0, '0', '0', $3) ON CONFLICT (id) DO NOTHING",
				&[&address, &referrer, &timestamp_to_date(timestamp.unwrap_or(0))],
			)
			.await?;
		let row = self
			.db
			.query_one(
				"SELECT referrer, total_order_placed_volume_u_s_d, total_order_filled_volume_u_s_d
				 FROM app.user_activity_v2s WHERE id = $1 FOR UPDATE",
				&[&address],
			)
			.await?;
		let mut referrer_now: Option<String> = row.get(0);
		if let Some(referrer) = referrer.filter(|r| !r.is_empty()) {
			if referrer_now.as_deref().is_none_or(str::is_empty) {
				self.db
					.execute(
						"UPDATE app.user_activity_v2s SET referrer = $2 WHERE id = $1",
						&[&address, &referrer],
					)
					.await?;
				referrer_now = Some(referrer.to_string());
			}
		}
		Ok(UserActivity {
			referrer: referrer_now,
			placed_volume: row.get(1),
			filled_volume: row.get(2),
		})
	}
}

struct UserActivity {
	referrer: Option<String>,
	placed_volume: String,
	filled_volume: String,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn address_forms() {
		let word = "0x0000000000000000000000003C43aD0F71BB11ba2cc9466C2C8614bbb2589d3d";
		assert_eq!(bytes32_to_bytes20(word), "0x3c43ad0f71bb11ba2cc9466c2c8614bbb2589d3d");
		let short = "0x3C43aD0F71BB11ba2cc9466C2C8614bbb2589d3d";
		assert_eq!(bytes32_to_bytes20(short), short);
		assert_eq!(decode_chain("0x45564d2d3536"), "EVM-56");
		assert_eq!(decode_chain("EVM-56"), "EVM-56");
		assert_eq!(whole_dollars("8256.869500000000000000"), U256::from(8256));
		assert_eq!(whole_dollars("0.990000000000000000"), U256::ZERO);
		// Above u64::MAX, as a $1-priced fake token can reach.
		assert_eq!(whole_dollars("50000000000000000000.5").to_string(), "50000000000000000000");
	}
}
