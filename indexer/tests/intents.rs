//! The intent gateway's order service against the real `app` schema. Set
//! `INDEXER_TEST_DATABASE_URL` to run. `INDEXER_TEST_NEXUS_RPC` (a Nexus node's HTTP RPC) runs
//! the live bid test.

mod common;

use std::collections::HashMap;

use alloy::primitives::{Address, Bytes, B256, U256};
use async_trait::async_trait;
use common::{database, text};
use hyperbridge_indexer::{
	handlers::intents::{
		bids::extract_user_op,
		calls::{Order, TokenAmount},
		dust::{record_dust, Dust},
		orders::{FeeToken, FillKind, FillReceipt, Gateway, Market, TxMeta},
	},
	services::decimal::Dec,
	store::OrderStatus,
	substrate::SubstrateRpc,
};
use tokio_postgres::Transaction;

const USDC: Address = alloy::primitives::address!("833589fcd6edb6e08f4c7c32d4f71b54bda02913");
const WETH: Address = alloy::primitives::address!("4200000000000000000000000000000000000006");
const CNGN: Address = alloy::primitives::address!("46c85152bfe9f96829aa94755d9f915f9b10ef5f");
const USER: Address = alloy::primitives::address!("3c43ad0f71bb11ba2cc9466c2c8614bbb2589d3d");
const BENEFICIARY: Address =
	alloy::primitives::address!("da64d52fa817be25a7e492bb470b621adc3d40c2");
const FILLER: &str = "0xce319986ca4D5d0893751a628D0db3Dc8fc91d62";
const REFERRER: &str = "0x4879706572465800000000000000000000000000000000000000000000000000";

/** USDC at $1 with 6 decimals, WETH at $3 with no decimals, cNGN unpriced. */
struct TestMarket {
	tokens: HashMap<Address, (&'static str, u8)>,
	prices: HashMap<&'static str, &'static str>,
}

impl TestMarket {
	fn new() -> Self {
		Self {
			tokens: HashMap::from([(USDC, ("USDC", 6)), (WETH, ("WETH", 0)), (CNGN, ("cNGN", 6))]),
			prices: HashMap::from([("USDC", "1"), ("WETH", "3")]),
		}
	}
}

#[async_trait]
impl Market for TestMarket {
	async fn token_metadata(&self, token: Address) -> anyhow::Result<(String, u8)> {
		let (symbol, decimals) =
			self.tokens.get(&token).ok_or_else(|| anyhow::anyhow!("{token} has no code"))?;
		Ok((symbol.to_string(), *decimals))
	}

	async fn usd_price(&self, symbol: &str) -> Option<Dec> {
		self.prices.get(symbol).map(|p| Dec::parse(p).unwrap())
	}
}

fn gateway<'a>(db: &'a Transaction<'a>, market: &'a TestMarket) -> Gateway<'a> {
	Gateway { db, chain: "8453", state_machine: "EVM-8453", market }
}

fn token(address: Address, amount: u64) -> TokenAmount {
	TokenAmount { token: address.into_word(), amount: U256::from(amount) }
}

fn order(inputs: Vec<TokenAmount>, outputs: Vec<TokenAmount>, nonce: u64) -> Order {
	Order {
		user: USER.into_word(),
		source: "EVM-8453".into(),
		destination: "EVM-8453".into(),
		deadline: U256::from(53459566u64),
		nonce: U256::from(nonce),
		fees: U256::from(15086),
		session: "0xF162C33f96613888C86c1F95ab66F93A3Ed72303".parse().unwrap(),
		predispatch: vec![],
		predispatch_call: Bytes::new(),
		inputs,
		beneficiary: BENEFICIARY.into_word(),
		outputs,
		output_call: Bytes::new(),
	}
}

fn tx(byte: u8, block_number: u64, log_index: u64) -> TxMeta {
	TxMeta {
		transaction_hash: B256::repeat_byte(byte).to_string(),
		block_number,
		timestamp: 1_793_708_000 + block_number,
		log_index,
	}
}

async fn place(g: &Gateway<'_>, order: &Order, tx: &TxMeta) -> String {
	let commitment = order.commitment().unwrap().to_string();
	let fee = FeeToken { address: format!("{:#x}", USDC), decimals: 6 };
	g.get_or_create_order(order, &commitment, REFERRER, Some(&fee), Some("0xop"), tx)
		.await
		.unwrap();
	g.update_order_status(&commitment, OrderStatus::Placed, tx, None).await.unwrap();
	commitment
}

async fn one(db: &Transaction<'_>, sql: &str) -> Option<String> {
	text(db, sql).await
}

/**
 * Lets rows reference an order that isn't indexed yet. The schema's foreign keys are DEFERRABLE
 * but immediate, so in production such a row fails its batch, which retries until the order is
 * indexed; these tests exercise the code paths for rows that arrive first.
 */
async fn defer_foreign_keys(db: &Transaction<'_>) {
	db.batch_execute("SET CONSTRAINTS ALL DEFERRED").await.unwrap();
}

#[tokio::test]
async fn placement_writes_the_order_points_volume_and_user() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let market = TestMarket::new();
	let g = gateway(&db, &market);
	let order = order(vec![token(USDC, 8_256_869_500)], vec![token(CNGN, 11_311_911_214_988)], 1);
	let placed = tx(0xaa, 100, 3);
	let c = place(&g, &order, &placed).await;

	let row = one(
		&db,
		&format!(
			"SELECT concat_ws(' ', \"user\", input_u_s_d, status, session, fee_token, fee_token_decimals,
			   referrer, block_number, block_timestamp, user_op_hash, created_at)
			 FROM app.i_order_v3s WHERE id = '{c}'"
		),
	)
	.await;
	assert_eq!(
		row.as_deref(),
		Some(
			"0x3c43ad0f71bb11ba2cc9466c2c8614bbb2589d3d 8256 PLACED 0xF162C33f96613888C86c1F95ab66F93A3Ed72303 \
			 0x833589fcd6edb6e08f4c7c32d4f71b54bda02913 6 0x4879706572465800000000000000000000000000000000000000000000000000 \
			 100 1793708100 0xop 2026-11-03 12:15:00"
		)
	);
	let output = one(
		&db,
		&format!("SELECT token || ' ' || beneficiary FROM app.i_order_v3_output_assets WHERE id = '{c}-output-0'"),
	)
	.await;
	assert_eq!(
		output.as_deref(),
		Some(
			"0x00000000000000000000000046c85152bfe9f96829aa94755d9f915f9b10ef5f \
			 0x000000000000000000000000da64d52fa817be25a7e492bb470b621adc3d40c2"
		)
	);
	let status = one(
		&db,
		&format!(
			"SELECT chain || ' ' || block_number || ' ' || coalesce(filler, 'null')
			 FROM app.i_order_v3_status_metadata WHERE id = '{c}.PLACED'"
		),
	)
	.await;
	assert_eq!(status.as_deref(), Some("8453 100 null"));
	let points = one(
		&db,
		"SELECT points::text FROM app.rewards WHERE id = '0x3c43ad0f71bb11ba2cc9466c2c8614bbb2589d3d-EVM-8453-USER'",
	)
	.await;
	assert_eq!(points.as_deref(), Some("8256"));
	let log = one(
		&db,
		&format!(
			"SELECT description FROM app.rewards_activity_logs
			 WHERE id = '0x3c43ad0f71bb11ba2cc9466c2c8614bbb2589d3d-USER-{}'",
			placed.transaction_hash
		),
	)
	.await;
	assert_eq!(
		log,
		Some(format!(
			"Points awarded for placing orderV3 {c} with value 8256.869500000000000000 USD"
		))
	);
	let volume = one(
		&db,
		"SELECT volume_u_s_d::text FROM app.cumulative_volume_u_s_ds WHERE id = 'IntentGatewayV3.USER.EVM-8453'",
	)
	.await;
	assert_eq!(volume.as_deref(), Some("8256869500000000000000"));
	let user = one(
		&db,
		"SELECT concat_ws(' ', referrer, total_orders_placed, total_order_placed_volume_u_s_d, created_at)
		 FROM app.user_activity_v2s WHERE id = '0x3c43ad0f71bb11ba2cc9466c2c8614bbb2589d3d'",
	)
	.await;
	assert_eq!(user, Some(format!("{REFERRER} 1 8256.8695 2026-11-03 12:15:00")));
}

#[tokio::test]
async fn a_filled_order_credits_the_user_and_the_first_referrer_and_replays_keep_status() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let market = TestMarket::new();
	let g = gateway(&db, &market);
	let order = order(vec![token(USDC, 8_256_869_500)], vec![token(CNGN, 1)], 2);
	let c = place(&g, &order, &tx(0xaa, 100, 3)).await;
	g.update_order_status(&c, OrderStatus::Filled, &tx(0xbb, 110, 5), Some(FILLER))
		.await
		.unwrap();

	let user = one(
		&db,
		"SELECT total_filled_orders || ' ' || total_order_filled_volume_u_s_d FROM app.user_activity_v2s
		 WHERE id = '0x3c43ad0f71bb11ba2cc9466c2c8614bbb2589d3d'",
	)
	.await;
	assert_eq!(user.as_deref(), Some("1 8256"));
	let referred = one(
		&db,
		&format!("SELECT points::text FROM app.rewards WHERE id = '{REFERRER}-EVM-8453-REFERRER'"),
	)
	.await;
	assert_eq!(referred.as_deref(), Some("4128"));

	// A replay of the placement without receipt or fee token keeps what is known.
	g.get_or_create_order(&order, &c, REFERRER, None, None, &tx(0xaa, 100, 3))
		.await
		.unwrap();
	let row = one(
		&db,
		&format!("SELECT concat_ws(' ', status, fee_token, user_op_hash) FROM app.i_order_v3s WHERE id = '{c}'"),
	)
	.await;
	assert_eq!(row.as_deref(), Some("FILLED 0x833589fcd6edb6e08f4c7c32d4f71b54bda02913 0xop"));
	// FILLED orders don't earn the USER points again.
	let points = one(
		&db,
		"SELECT points::text FROM app.rewards WHERE id = '0x3c43ad0f71bb11ba2cc9466c2c8614bbb2589d3d-EVM-8453-USER'",
	)
	.await;
	assert_eq!(points.as_deref(), Some("8256"));
}

#[tokio::test]
async fn an_early_fill_is_backfilled_when_the_order_arrives() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	defer_foreign_keys(&db).await;
	let market = TestMarket::new();
	let g = gateway(&db, &market);
	let order = order(vec![token(USDC, 5_000_000)], vec![token(WETH, 1000)], 3);
	let c = order.commitment().unwrap().to_string();
	let filled = tx(0xcc, 90, 9);
	let receipt = FillReceipt {
		user_op_hash: Some("0xfillop".into()),
		amounts_received: Some(vec![Some(U256::from(1005))]),
	};
	g.record_fill(FillKind::Full, &c, FILLER, &[token(WETH, 1000)], &[], &filled, &receipt)
		.await
		.unwrap();
	g.update_order_status(&c, OrderStatus::Filled, &filled, Some(FILLER))
		.await
		.unwrap();

	let fill_id = format!("{}.9", filled.transaction_hash);
	let output = one(
		&db,
		&format!(
			"SELECT amount || ' ' || amount_received FROM app.i_order_v3_fill_output_assets WHERE id = '{fill_id}-output-0'"
		),
	)
	.await;
	assert_eq!(output.as_deref(), Some("1000 1005"));
	let pending = one(
		&db,
		&format!("SELECT filler || ' ' || block_hash || '.' FROM app.pending_status_metadata WHERE id = '{c}.IOrderV3.FILLED'"),
	)
	.await;
	assert_eq!(pending, Some(format!("{FILLER} .")));
	let filler_volume = one(
		&db,
		&format!(
			"SELECT volume_u_s_d::text FROM app.cumulative_volume_u_s_ds WHERE id = 'IntentGatewayV3.FILLER.{FILLER}.EVM-8453'"
		),
	)
	.await;
	assert_eq!(filler_volume.as_deref(), Some("3000000000000000000000"));
	// The aggregate was seeded empty before this fill, then credited with it.
	let filled_volume =
		one(&db, "SELECT volume_u_s_d::text FROM app.cumulative_volume_u_s_ds WHERE id = 'IntentGatewayV3.FILLED.EVM-8453'")
			.await;
	assert_eq!(filled_volume.as_deref(), Some("3000000000000000000000"));
	let no_points = one(
		&db,
		&format!("SELECT points::text FROM app.rewards WHERE id = '{FILLER}-EVM-8453-FILLER'"),
	)
	.await;
	assert_eq!(no_points, None);

	place(&g, &order, &tx(0xaa, 100, 3)).await;
	let output_filled = one(
		&db,
		&format!("SELECT filled::text FROM app.i_order_v3_output_assets WHERE id = '{c}-output-0'"),
	)
	.await;
	assert_eq!(output_filled.as_deref(), Some("1000"));
	let points = one(
		&db,
		&format!("SELECT points::text FROM app.rewards WHERE id = '{FILLER}-EVM-8453-FILLER'"),
	)
	.await;
	assert_eq!(points.as_deref(), Some("3000"));
	// The pending FILLED becomes metadata, but the order row stays PLACED.
	let status = one(
		&db,
		&format!(
			"SELECT o.status || ' ' || m.filler FROM app.i_order_v3s o
			 JOIN app.i_order_v3_status_metadata m ON m.id = o.id || '.FILLED' WHERE o.id = '{c}'"
		),
	)
	.await;
	assert_eq!(status, Some(format!("PLACED {FILLER}")));
	assert_eq!(
		one(&db, &format!("SELECT id FROM app.pending_status_metadata WHERE commitment = '{c}'"))
			.await,
		None
	);
}

#[tokio::test]
async fn partial_fills_take_the_order_beneficiary_or_a_zero_address() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	defer_foreign_keys(&db).await;
	let market = TestMarket::new();
	let g = gateway(&db, &market);
	let order = order(vec![token(USDC, 5_000_000)], vec![token(WETH, 1000)], 4);
	let c = order.commitment().unwrap().to_string();
	let early = tx(0xd1, 90, 1);
	g.record_fill(
		FillKind::Partial,
		&c,
		FILLER,
		&[token(WETH, 400)],
		&[],
		&early,
		&FillReceipt::default(),
	)
	.await
	.unwrap();
	place(&g, &order, &tx(0xaa, 100, 3)).await;
	let late = tx(0xd2, 120, 1);
	g.record_fill(
		FillKind::Partial,
		&c,
		FILLER,
		&[token(WETH, 600)],
		&[],
		&late,
		&FillReceipt::default(),
	)
	.await
	.unwrap();
	let beneficiaries = one(
		&db,
		"SELECT string_agg(beneficiary, ' ' ORDER BY id) FROM app.i_order_v3_partial_fill_output_assets",
	)
	.await;
	assert_eq!(
		beneficiaries.as_deref(),
		Some(
			"0x0000000000000000000000000000000000000000 \
			 0x000000000000000000000000da64d52fa817be25a7e492bb470b621adc3d40c2"
		)
	);
	let filled = one(
		&db,
		&format!("SELECT filled::text FROM app.i_order_v3_output_assets WHERE id = '{c}-output-0'"),
	)
	.await;
	assert_eq!(filled.as_deref(), Some("1000"));
	// Partial fills don't change the order's status.
	let status =
		one(&db, &format!("SELECT status::text FROM app.i_order_v3s WHERE id = '{c}'")).await;
	assert_eq!(status.as_deref(), Some("PLACED"));
	let points = one(
		&db,
		&format!("SELECT points::text FROM app.rewards WHERE id = '{FILLER}-EVM-8453-FILLER'"),
	)
	.await;
	assert_eq!(points.as_deref(), Some("3000"));
}

#[tokio::test]
async fn releases_accumulate_until_the_order_is_redeemed() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let market = TestMarket::new();
	let g = gateway(&db, &market);
	let order = order(vec![token(USDC, 100)], vec![token(WETH, 1)], 5);
	let c = place(&g, &order, &tx(0xaa, 100, 3)).await;
	g.record_escrow_release(&c, Some(FILLER), &[token(USDC, 40)], &tx(0xe1, 110, 2))
		.await
		.unwrap();
	let released = one(
		&db,
		&format!("SELECT released::text FROM app.i_order_v3_input_assets WHERE id = '{c}-input-0'"),
	)
	.await;
	assert_eq!(released.as_deref(), Some("40"));
	assert_eq!(
		one(
			&db,
			&format!("SELECT id FROM app.i_order_v3_status_metadata WHERE id = '{c}.REDEEMED'")
		)
		.await,
		None
	);

	g.record_escrow_release(&c, Some(FILLER), &[token(USDC, 60)], &tx(0xe2, 120, 2))
		.await
		.unwrap();
	// The same log again is skipped.
	g.record_escrow_release(&c, Some(FILLER), &[token(USDC, 60)], &tx(0xe2, 120, 2))
		.await
		.unwrap();
	let row = one(
		&db,
		&format!(
			"SELECT a.released || ' ' || o.status || ' ' || m.filler FROM app.i_order_v3_input_assets a
			 JOIN app.i_order_v3s o ON o.id = a.order_id JOIN app.i_order_v3_status_metadata m ON m.id = o.id || '.REDEEMED'
			 WHERE a.id = '{c}-input-0'"
		),
	)
	.await;
	assert_eq!(row, Some(format!("100 REDEEMED {FILLER}")));
	let tokens = one(&db, "SELECT count(*)::text FROM app.i_order_v3_escrow_release_tokens").await;
	assert_eq!(tokens.as_deref(), Some("2"));
}

#[tokio::test]
async fn cancellations_write_metadata_never_the_order_row() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	defer_foreign_keys(&db).await;
	let market = TestMarket::new();
	let g = gateway(&db, &market);
	let canceller = "0x7f929bC0C702370d5E94e00240ba3E6b1e69f605";
	let order = order(vec![token(USDC, 100)], vec![token(WETH, 1)], 6);
	let c = order.commitment().unwrap().to_string();
	let event = TxMeta {
		transaction_hash: B256::repeat_byte(0xf1).to_string(),
		block_number: 123,
		timestamp: 456,
		log_index: 7,
	};

	// Before the order: the cancellation row, and a pending status.
	g.record_order_cancellation(&c, canceller, &event).await.unwrap();
	let pending = one(
		&db,
		&format!("SELECT chain || ' ' || status || ' ' || block_number FROM app.pending_status_metadata WHERE id = '{c}.IOrderV3.CANCELLED'"),
	)
	.await;
	assert_eq!(pending.as_deref(), Some("8453 CANCELLED 123"));
	place(&g, &order, &tx(0xaa, 100, 3)).await;
	assert_eq!(
		one(&db, &format!("SELECT id FROM app.pending_status_metadata WHERE commitment = '{c}'"))
			.await,
		None
	);

	g.record_order_cancellation(&c, canceller, &event).await.unwrap();
	g.record_order_cancellation(&c, canceller, &TxMeta { log_index: 8, ..event.clone() })
		.await
		.unwrap();
	let rows = one(
		&db,
		&format!(
			"SELECT count(*) || ' ' || min(canceller) || ' ' || min(chain) || ' ' || min(timestamp) FROM app.i_order_v3_cancellations
			 WHERE order_id = '{c}'"
		),
	)
	.await;
	assert_eq!(rows, Some(format!("2 {canceller} 8453 456")));
	let status =
		one(&db, &format!("SELECT status::text FROM app.i_order_v3s WHERE id = '{c}'")).await;
	assert_eq!(status.as_deref(), Some("PLACED"));

	// The same-chain refund completes the order.
	g.record_escrow_refund(&c, &[token(USDC, 100)], &event).await.unwrap();
	g.update_order_status(&c, OrderStatus::Refunded, &event, None).await.unwrap();
	let statuses = one(
		&db,
		&format!(
			"SELECT o.status || ' ' || string_agg(m.status::text, ',' ORDER BY m.id) FROM app.i_order_v3s o
			 JOIN app.i_order_v3_status_metadata m ON m.order_id = o.id WHERE o.id = '{c}' GROUP BY o.status"
		),
	)
	.await;
	assert_eq!(statuses.as_deref(), Some("REFUNDED CANCELLED,PLACED,REFUNDED"));
}

#[tokio::test]
async fn protocol_fee_refunds_keep_their_first_write_without_an_order() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	defer_foreign_keys(&db).await;
	let market = TestMarket::new();
	let g = gateway(&db, &market);
	let commitment = B256::repeat_byte(0x99).to_string();
	let event = TxMeta {
		transaction_hash: B256::repeat_byte(0xf2).to_string(),
		block_number: 7,
		timestamp: 9,
		log_index: 4,
	};
	let token = "0x46C85152bFe9f96829aA94755D9f915F9B10EF5F";
	g.record_protocol_fee_refund(&commitment, token, U256::from(5), &event)
		.await
		.unwrap();
	g.record_protocol_fee_refund(&commitment, token, U256::from(6), &event)
		.await
		.unwrap();
	let row = one(
		&db,
		&format!(
			"SELECT concat_ws(' ', order_id, chain, token, amount, block_number) FROM app.i_order_v3_protocol_fee_refunds
			 WHERE id = '{}.4'",
			event.transaction_hash
		),
	)
	.await;
	assert_eq!(row, Some(format!("{commitment} 8453 {token} 5 7")));
}

#[tokio::test]
async fn dust_adds_amounts_and_priced_usd() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	record_dust(
		&db,
		Dust::Collected,
		"EVM-8453",
		Address::ZERO,
		U256::from(60),
		1_000,
		"eth",
		"60.000000000000000000",
	)
	.await
	.unwrap();
	let row = one(
		&db,
		"SELECT d.token_symbol || ' ' || d.amount || ' ' || c.amount_u_s_d || ' ' || c.last_updated_at
		 FROM app.protocol_dust_collecteds d JOIN app.cumulative_dust_collected_per_chains c ON c.id = d.chain
		 WHERE d.id = 'EVM-8453-0x0000000000000000000000000000000000000000'",
	)
	.await;
	assert_eq!(row.as_deref(), Some("eth 60 60000000000000000000 1000"));

	// Unpriced dust keeps its amount out of the USD total.
	record_dust(&db, Dust::Collected, "EVM-8453", Address::ZERO, U256::from(5), 2_000, "eth", "0")
		.await
		.unwrap();
	record_dust(
		&db,
		Dust::Swept,
		"EVM-8453",
		USDC,
		U256::from(7),
		2_000,
		"USDC",
		"0.000007000000000000",
	)
	.await
	.unwrap();
	let collected = one(
		&db,
		"SELECT d.amount || ' ' || c.amount_u_s_d FROM app.protocol_dust_collecteds d
		 JOIN app.cumulative_dust_collected_per_chains c ON c.id = d.chain",
	)
	.await;
	assert_eq!(collected.as_deref(), Some("65 60000000000000000000"));
	let swept = one(
		&db,
		"SELECT d.id || ' ' || c.amount_u_s_d FROM app.protocol_dust_swepts d
		 JOIN app.cumulative_dust_swept_per_chains c ON c.id = d.chain",
	)
	.await;
	assert_eq!(
		swept.as_deref(),
		Some("EVM-8453-0x833589fcd6edb6e08f4c7c32d4f71b54bda02913 7000000000000")
	);
}

#[tokio::test]
async fn order_volume_prices_stables_and_keeps_unpriced_amounts() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let market = TestMarket::new();
	let g = Gateway { db: &db, chain: "137", state_machine: "EVM-137", market: &market };
	g.record_order_volume("PLACED", &[token(USDC, 100_000_000)], 2).await.unwrap();
	g.record_order_volume("PLACED", &[token(USDC, 10_000_000), token(USDC, 5_000_000)], 3)
		.await
		.unwrap();
	let row = one(
		&db,
		"SELECT t.token_symbol || ' ' || t.amount || ' ' || c.volume_u_s_d || ' ' || c.last_updated_at
		 FROM app.intent_gateway_token_volumes t JOIN app.cumulative_intent_gateway_volume_u_s_ds c ON c.id = 'EVM-137-PLACED'
		 WHERE t.id = 'EVM-137-0x833589fcd6edb6e08f4c7c32d4f71b54bda02913-PLACED'",
	)
	.await;
	assert_eq!(row.as_deref(), Some("USDC 115000000 115000000000000000000 3"));

	g.record_order_volume("FILLED", &[token(CNGN, 3_000_000_000)], 4).await.unwrap();
	let cngn = one(
		&db,
		"SELECT amount::text FROM app.intent_gateway_token_volumes WHERE id = 'EVM-137-0x46c85152bfe9f96829aa94755d9f915f9b10ef5f-FILLED'",
	)
	.await;
	assert_eq!(cngn.as_deref(), Some("3000000000"));
	assert_eq!(one(&db, "SELECT id FROM app.cumulative_intent_gateway_volume_u_s_ds WHERE id = 'EVM-137-FILLED'").await, None);

	// A token without readable metadata skips the USD total but not the other tokens' amounts.
	let unknown = Address::repeat_byte(0x77);
	g.record_order_volume("PLACED", &[token(USDC, 1_000_000), token(unknown, 1)], 5)
		.await
		.unwrap();
	let after = one(
		&db,
		"SELECT t.amount || ' ' || c.volume_u_s_d FROM app.intent_gateway_token_volumes t
		 JOIN app.cumulative_intent_gateway_volume_u_s_ds c ON c.id = 'EVM-137-PLACED'
		 WHERE t.id = 'EVM-137-0x833589fcd6edb6e08f4c7c32d4f71b54bda02913-PLACED'",
	)
	.await;
	assert_eq!(after.as_deref(), Some("116000000 115000000000000000000"));
}

#[tokio::test]
async fn bid_payload_matches_the_live_indexer() {
	let Ok(url) = std::env::var("INDEXER_TEST_NEXUS_RPC") else { return };
	let rpc = SubstrateRpc::new(&url, reqwest::Client::new());
	// Nexus block 12597100, event 7: filler_bids row
	// 0x1e5aac55…-12hJWxswyBApyLeoiwzyLDScSrK3MLe5KAKVSdMd7MJLmdjc-12597100-7.
	let hash: String = rpc.call("chain_getBlockHash", serde_json::json!([12597100])).await.unwrap();
	let block: serde_json::Value =
		rpc.call("chain_getBlock", serde_json::json!([hash])).await.unwrap();
	let extrinsic = block["block"]["extrinsics"][2].as_str().unwrap();
	let extrinsic = hex::decode(extrinsic.trim_start_matches("0x")).unwrap();
	assert_eq!(
		alloy::primitives::keccak256(&extrinsic).to_string(),
		"0xe3d0f21fc38ef7ea9e6300ea230a3a0d7d324870c0c4ba028a212b8ac8686920"
	);
	let commitment: B256 = "0x1e5aac55b5069b58d6a053f332b737a26d4b7d3837cdcecc5a4485f460927307"
		.parse()
		.unwrap();
	let bid: B256 = "0xc00aee40ddec7fc8dcfe182b676c930c546e563c5aae2e558138b6dd5192a20e"
		.parse()
		.unwrap();
	let user_op = hex::encode(extract_user_op(&extrinsic, &commitment, &bid).unwrap());
	assert_eq!(user_op.len(), 6612);
	assert!(user_op.starts_with("ce319986ca4d5d0893751a628d0db3dc8fc91d621c1945e2739b22af62eb0bf9"));
	assert!(user_op.ends_with("dc0c3381bc50420375b382c6054eb85559c19f8fb57ac774c2577150d47bed1b"));
}
