//! The shared services against the real `app` schema. Set `INDEXER_TEST_DATABASE_URL` to run.

mod common;

use common::{database, text};
use hyperbridge_indexer::{
	config::Environment,
	services::{
		points::{award_points, Award},
		prices::{token_price, Orderbook},
		relayers::{
			add_chain_stats, add_relayer_stats, touch_relayer_activity, ChainStatsDelta,
			RelayerStatsDelta,
		},
		solvers::{queue_solver, Discovery},
		volume::update_volume,
	},
	store::{numeric, PointsActivityType, ProtocolParticipantType, SolverDiscoveryTrigger},
};

#[tokio::test]
async fn points_add_up_and_the_log_keeps_the_last_award() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	for points in [50, 25] {
		award_points(
			&db,
			Award {
				address: "0xabc",
				chain: "EVM-56",
				points: alloy::primitives::U256::from(points),
				earner: ProtocolParticipantType::Relayer,
				activity: PointsActivityType::RewardPointsEarned,
				transaction_hash: "0xtx",
				description: "delivered",
				timestamp: 1_700_000_000,
			},
		)
		.await
		.unwrap();
	}
	let points =
		text(&db, "SELECT points::text FROM app.rewards WHERE id = '0xabc-EVM-56-RELAYER'").await;
	assert_eq!(points.as_deref(), Some("75"));
	let log = text(
		&db,
		"SELECT points::text || ' ' || created_at::text FROM app.rewards_activity_logs WHERE id = '0xabc-RELAYER-0xtx'",
	)
	.await;
	assert_eq!(log.as_deref(), Some("25 2023-11-14 22:13:20"));
}

#[tokio::test]
async fn cumulative_volume_drops_a_second_update_in_the_same_block() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	update_volume(&db, "Transfer.USDC", "EVM-56", "25", 1_700_000_000)
		.await
		.unwrap();
	update_volume(&db, "Transfer.USDC", "EVM-56", "5", 1_700_000_000).await.unwrap();
	update_volume(&db, "Transfer.USDC", "EVM-56", "1.5", 1_700_000_012)
		.await
		.unwrap();
	let cumulative = text(
		&db,
		"SELECT volume_u_s_d::text || ' ' || last_updated_at::text FROM app.cumulative_volume_u_s_ds WHERE id = 'Transfer.USDC.EVM-56'",
	)
	.await;
	assert_eq!(cumulative.as_deref(), Some("26500000000000000000 1700000012"));
	let daily = text(
		&db,
		"SELECT last24_hours_volume_u_s_d::text || ' ' || created_at::text FROM app.daily_volume_u_s_ds WHERE id = 'Transfer.USDC.EVM-56.2023-11-14'",
	)
	.await;
	assert_eq!(daily.as_deref(), Some("31500000000000000000 2023-11-14 00:00:00"));
}

#[tokio::test]
async fn relayer_and_chain_stats_accumulate() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let delta = RelayerStatsDelta {
		messages_delivered: numeric(1),
		gas_used: numeric(21_000),
		..Default::default()
	};
	add_relayer_stats(&db, "0xrelayer", "EVM-1", &delta).await.unwrap();
	add_relayer_stats(
		&db,
		"0xrelayer",
		"EVM-1",
		&RelayerStatsDelta { withdrawn: numeric(7), ..delta },
	)
	.await
	.unwrap();
	touch_relayer_activity(&db, "0xrelayer", 1_700_000_000).await.unwrap();
	let stats = text(
		&db,
		"SELECT number_of_successful_messages_delivered || ' ' || gas_used_for_successful_messages || ' ' || cumulative_withdrawn_amount
		 FROM app.relayer_stats_per_chain_v2s WHERE id = '0xrelayer-EVM-1'",
	)
	.await;
	assert_eq!(stats.as_deref(), Some("2 42000 7"));
	add_chain_stats(
		&db,
		"EVM-1",
		&ChainStatsDelta { messages_sent: numeric(1), ..Default::default() },
	)
	.await
	.unwrap();
	add_chain_stats(
		&db,
		"EVM-1",
		&ChainStatsDelta { messages_sent: numeric(1), ..Default::default() },
	)
	.await
	.unwrap();
	let sent = text(
		&db,
		"SELECT number_of_messages_sent::text FROM app.hyper_bridge_stats WHERE id = 'EVM-1'",
	)
	.await;
	assert_eq!(sent.as_deref(), Some("2"));
}

#[tokio::test]
async fn the_first_discovery_of_a_solver_wins() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	for (by, block) in [(SolverDiscoveryTrigger::Fill, 10), (SolverDiscoveryTrigger::Watchlist, 20)]
	{
		let fill_transaction = (by == SolverDiscoveryTrigger::Fill).then_some("0xfill");
		queue_solver(
			&db,
			"EVM-56",
			"0xABCD",
			Discovery { by, fill_transaction, block_number: block, block_timestamp: 1_700_000_000 },
		)
		.await
		.unwrap();
	}
	let row = text(
		&db,
		"SELECT status || ' ' || discovered_by || ' ' || discovery_block FROM app.tracked_solvers WHERE id = 'EVM-56-0xabcd'",
	)
	.await;
	assert_eq!(row.as_deref(), Some("PENDING FILL 10"));
}

#[tokio::test]
async fn token_prices_are_zero_on_testnets_and_stables_are_stored_at_one() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let orderbook = Orderbook::new(None, reqwest::Client::new());
	let testnet = token_price(&orderbook, Environment::Testnet, &db, "USDC", 1_700_000_000)
		.await
		.unwrap();
	assert!(testnet.is_zero());
	let usdc = token_price(&orderbook, Environment::Mainnet, &db, "USDC", 1_700_000_000)
		.await
		.unwrap();
	assert_eq!(usdc.to_string(), "1");
	let stored =
		text(&db, "SELECT price || ' ' || last_updated_at FROM app.token_prices WHERE id = 'USDC'")
			.await;
	assert_eq!(stored.as_deref(), Some("1 1700000000000"));
	let unpriced = token_price(&orderbook, Environment::Mainnet, &db, "WETH", 1_700_000_000)
		.await
		.unwrap();
	assert!(unpriced.is_zero());
}

#[tokio::test]
async fn pending_statuses_wait_for_their_parent() {
	use hyperbridge_indexer::{
		services::pending::{
			defer_status, flush_commitment, flush_ready, PendingEntity, PendingStatus,
		},
		store::timestamp_secs,
	};
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let pending = |commitment: &'static str| PendingStatus {
		commitment,
		entity: PendingEntity::Request,
		status: "DESTINATION",
		chain: "EVM-1",
		timestamp: numeric(1_700_000_000u64),
		block_number: "5".into(),
		block_hash: "0xblock",
		transaction_hash: "0xtx",
		filler: None,
		created_at: timestamp_secs(1_700_000_000),
	};
	defer_status(&db, &pending("0xparent")).await.unwrap();
	defer_status(&db, &pending("0xorphan")).await.unwrap();
	assert_eq!(flush_ready(&db, 10).await.unwrap(), 0, "no parent yet");

	db.execute(
		"INSERT INTO app.request_v2s (id, commitment, chain, \"from\", \"to\", nonce, body, fee, created_at)
		 VALUES ('0xparent', '0xparent', 'EVM-56', '0xa', '0xb', 1, '0x', 0, now())",
		&[],
	)
	.await
	.unwrap();
	assert_eq!(flush_commitment(&db, PendingEntity::Request, "0xparent").await.unwrap(), 1);
	let status = text(
		&db,
		"SELECT status || ' ' || request_id FROM app.request_status_metadata WHERE id = '0xparent.DESTINATION'",
	)
	.await;
	assert_eq!(status.as_deref(), Some("DESTINATION 0xparent"));
	let left =
		text(&db, "SELECT string_agg(commitment, ',') FROM app.pending_status_metadata").await;
	assert_eq!(left.as_deref(), Some("0xorphan"));
}
