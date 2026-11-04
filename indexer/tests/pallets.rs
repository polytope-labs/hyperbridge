//! The Hyperbridge pallet handlers' writes against the real `app` schema. Set
//! `INDEXER_TEST_DATABASE_URL` to run.

mod common;

use alloy::primitives::{B256, U256};
use common::{database, text};
use hyperbridge_indexer::handlers::pallets::{
	bandwidth::{
		apply_tier_set, record_consumption, record_credit, record_eviction, set_active_counts,
		Credit,
	},
	relayer::record_accumulated_fees,
	rewards::{
		record_collator_reward, record_relayer_reward, record_treasury_transfer, RelayerReward,
	},
	supply::{write_supply, Supply},
};

/** 2026-11-03T13:06:36Z, a Nexus block. */
const MS: u64 = 1_793_711_196_000;

/** The last reward in an extrinsic keeps SubQuery's id; the earlier ones, which SubQuery
 * overwrote, move to numbered ids in order. */
#[tokio::test]
async fn relayer_rewards_sum_and_keep_every_transaction_in_one_extrinsic() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	for amount in [100, 50, 250] {
		let reward = RelayerReward {
			relayer: "14ERRjLB".into(),
			amount,
			chain: "POLKADOT-3367",
			block_number: 12_597_592,
			timestamp_ms: MS,
			extrinsic: Some((2, B256::repeat_byte(0xab))),
		};
		record_relayer_reward(&db, &reward).await.unwrap();
	}
	let totals = text(
		&db,
		"SELECT total_reward_amount || ' ' || total_consensus_reward_amount || ' ' ||
		   (total_messaging_reward_amount IS NULL) || ' ' || (reputation_asset_balance IS NULL)
		 FROM app.hyperbridge_relayer_rewards WHERE id = '14ERRjLB'",
	)
	.await;
	assert_eq!(totals.as_deref(), Some("400 400 true true"));
	let daily = text(
		&db,
		"SELECT daily_reward_amount || ' ' || date FROM app.daily_treasury_relayer_rewards WHERE id = '2026-11-03'",
	)
	.await;
	assert_eq!(daily.as_deref(), Some("400 2026-11-03 00:00:00"));
	let earlier = text(
		&db,
		"SELECT string_agg(id || '=' || amount, ',' ORDER BY id) FROM app.relayer_reward_transactions
		 WHERE id LIKE '14ERRjLB-12597592-2-%'",
	)
	.await;
	assert_eq!(earlier.as_deref(), Some("14ERRjLB-12597592-2-1=100,14ERRjLB-12597592-2-2=50"));
	let transaction = text(
		&db,
		"SELECT amount || ' ' || chain || ' ' || reward_type || ' ' || block_timestamp || ' ' ||
		   extrinsic_hash || ' ' || created_at
		 FROM app.relayer_reward_transactions WHERE id = '14ERRjLB-12597592-2'",
	)
	.await;
	assert_eq!(
		transaction.as_deref(),
		Some(
			"250 POLKADOT-3367 CONSENSUS_REWARD 1793711196000 \
			 0xabababababababababababababababababababababababababababababababab 2026-11-03 13:06:36"
		)
	);

	// Outside an extrinsic SubQuery saw no extrinsic: index 0 and no hash.
	let reward = RelayerReward {
		relayer: "14ERRjLB".into(),
		amount: 1,
		chain: "POLKADOT-3367",
		block_number: 12_597_593,
		timestamp_ms: MS + 6000,
		extrinsic: None,
	};
	record_relayer_reward(&db, &reward).await.unwrap();
	let hash = text(
		&db,
		"SELECT coalesce(extrinsic_hash, 'null') FROM app.relayer_reward_transactions WHERE id = '14ERRjLB-12597593-0'",
	)
	.await;
	assert_eq!(hash.as_deref(), Some("null"));
}

#[tokio::test]
async fn collator_rewards_count_blocks() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	record_collator_reward(&db, "12Nt6vSY", 40, 12_597_592, MS).await.unwrap();
	record_collator_reward(&db, "12Nt6vSY", 60, 12_597_593, MS + 6000)
		.await
		.unwrap();
	let totals = text(
		&db,
		"SELECT total_reward_amount || ' ' || total_blocks_authored || ' ' || last_rewarded_at
		 FROM app.hyperbridge_collator_rewards WHERE id = '12Nt6vSY'",
	)
	.await;
	assert_eq!(totals.as_deref(), Some("100 2 2026-11-03 13:06:42"));
	let daily = text(
		&db,
		"SELECT daily_reward_amount::text FROM app.daily_treasury_collator_rewards WHERE id = '2026-11-03'",
	)
	.await;
	assert_eq!(daily.as_deref(), Some("100"));
	let transaction = text(
		&db,
		"SELECT amount || ' ' || block_timestamp || ' ' || created_at
		 FROM app.hyperbridge_collator_reward_transactions WHERE id = '12Nt6vSY-12597592'",
	)
	.await;
	assert_eq!(transaction.as_deref(), Some("40 1793711196000 2026-11-03 13:06:36"));
}

#[tokio::test]
async fn treasury_totals_accumulate_and_the_balance_is_overwritten() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	record_treasury_transfer(&db, true, 30, 1_000, MS).await.unwrap();
	record_treasury_transfer(&db, true, 20, 950, MS + 6000).await.unwrap();
	record_treasury_transfer(&db, false, 5, 955, MS + 12_000).await.unwrap();
	let row = text(
		&db,
		"SELECT total_amount_transferred_in || ' ' || total_amount_transferred_out || ' ' ||
		   total_balance || ' ' || last_updated_at
		 FROM app.treasuries WHERE id = '13UVJyLkyUpEiXBx5p776dHQoBuuk3Y5PYp5Aa89rYWePWA3'",
	)
	.await;
	assert_eq!(row.as_deref(), Some("5 50 955 2026-11-03 13:06:48"));
}

#[tokio::test]
async fn accumulated_fees_credit_the_relayer() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let relayer = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";
	for fees in [7u64, 3] {
		record_accumulated_fees(&db, relayer, "EVM-1", U256::from(fees), MS)
			.await
			.unwrap();
	}
	let fees = text(
		&db,
		&format!(
			"SELECT lifetime_fees || ' ' || relayer || ' ' || chain_id || ' ' || last_updated_at
			 FROM app.accumulated_fees WHERE id = '{relayer}-EVM-1'"
		),
	)
	.await;
	assert_eq!(fees.as_deref(), Some(format!("10 {relayer} EVM-1 2026-11-03 13:06:36").as_str()));
	let stats = text(
		&db,
		&format!(
			"SELECT fees_earned || ' ' || cumulative_withdrawn_amount
			 FROM app.relayer_stats_per_chain_v2s WHERE id = '{relayer}-EVM-1'"
		),
	)
	.await;
	assert_eq!(stats.as_deref(), Some("10 0"));
	let activity = text(
		&db,
		&format!("SELECT last_updated_at::text FROM app.relayer_activities WHERE id = '{relayer}'"),
	)
	.await;
	assert_eq!(activity.as_deref(), Some("1793711196000"));
}

fn credit<'a>(app_hex: &'a str, tier: u8, block_number: u64, timestamp_ms: u64) -> Credit<'a> {
	Credit {
		chain: "EVM-1",
		app_hex,
		tier,
		bytes: 102_400,
		expires_at_secs: timestamp_ms / 1000 + 2_592_000,
		paid_from: "EVM-56",
		forced: false,
		block_number,
		log_index: 14,
		timestamp_ms,
		extrinsic_hash: None,
	}
}

#[tokio::test]
async fn bandwidth_credits_evictions_and_active_counts() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let app = "5762e54a92f331226dec592a4cebcf35579ffdc2";
	let app_id = format!("EVM-1-{app}");
	record_credit(&db, &credit(app, 1, 100, MS)).await.unwrap();
	record_credit(&db, &credit(app, 1, 101, MS + 6000)).await.unwrap();
	record_credit(&db, &credit(app, 2, 102, MS + 12_000)).await.unwrap();

	let sub = text(
		&db,
		&format!(
			"SELECT tier || ' ' || bytes || ' ' || expires_at || ' ' || purchased_at || ' ' || paid_from
			   || ' ' || forced || ' ' || coalesce(extrinsic_hash, 'null')
			 FROM app.bandwidth_subscriptions WHERE id = '{app_id}-100-14'"
		),
	)
	.await;
	assert_eq!(sub.as_deref(), Some("1 102400 1796303196 1793711196000 EVM-56 false null"));

	// The oldest unevicted subscription of the tier is marked, then the next one.
	record_eviction(&db, "EVM-1", app, 1, 50, MS + 18_000).await.unwrap();
	record_eviction(&db, "EVM-1", app, 1, 60, MS + 24_000).await.unwrap();
	let evicted = text(
		&db,
		&format!(
			"SELECT string_agg(id || ':' || coalesce(evicted_lost_bytes::text, '-'), ',' ORDER BY id)
			 FROM app.bandwidth_subscriptions WHERE app_id = '{app_id}'"
		),
	)
	.await;
	assert_eq!(
		evicted.as_deref(),
		Some(format!("{app_id}-100-14:50,{app_id}-101-14:60,{app_id}-102-14:-").as_str())
	);
	// Nothing left to mark: the counters still move.
	record_eviction(&db, "EVM-1", app, 1, 5, MS + 30_000).await.unwrap();

	// Storage holds one tier-2 and two tier-4 subscriptions: tier 1 drops to 0, tier 4 is new.
	set_active_counts(&db, "EVM-1", app, &[2, 4, 4], MS + 30_000).await.unwrap();
	let app_row = text(
		&db,
		&format!(
			"SELECT first_seen_at || ' ' || last_activity_at || ' ' || lifetime_subscriptions || ' ' ||
			   lifetime_bytes_credited || ' ' || lifetime_bytes_evicted || ' ' || active_subscriptions
			 FROM app.bandwidth_apps WHERE id = '{app_id}'"
		),
	)
	.await;
	assert_eq!(app_row.as_deref(), Some("1793711196000 1793711226000 3 307200 115 3"));
	let tiers = text(
		&db,
		&format!(
			"SELECT string_agg(tier || ':' || lifetime_subscriptions || '/' || lifetime_bytes_evicted || '/'
			   || active_subscriptions || '/' || last_purchased_at, ',' ORDER BY tier)
			 FROM app.bandwidth_app_tier_stats WHERE app_id = '{app_id}'"
		),
	)
	.await;
	assert_eq!(
		tiers.as_deref(),
		Some("1:2/115/0/1793711202000,2:1/0/1/1793711208000,4:0/0/2/1793711226000")
	);

	// Re-processing a credit resets its eviction marks.
	record_credit(&db, &credit(app, 1, 100, MS)).await.unwrap();
	let marks = text(
		&db,
		&format!(
			"SELECT (evicted_at IS NULL)::text FROM app.bandwidth_subscriptions WHERE id = '{app_id}-100-14'"
		),
	)
	.await;
	assert_eq!(marks.as_deref(), Some("true"));
}

#[tokio::test]
async fn bandwidth_consumption_fills_daily_buckets() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let app = "aa";
	record_consumption(&db, "EVM-1", app, 500, MS).await.unwrap();
	record_consumption(&db, "EVM-1", app, 364, MS + 6000).await.unwrap();
	record_consumption(&db, "EVM-1", app, 1, MS + 86_400_000).await.unwrap();
	let buckets = text(
		&db,
		"SELECT string_agg(id || ':' || bytes_consumed || '@' || date, ',' ORDER BY id)
		 FROM app.bandwidth_app_daily_consumptions WHERE app_id = 'EVM-1-aa'",
	)
	.await;
	assert_eq!(
		buckets.as_deref(),
		Some(
			"EVM-1-aa-2026-11-03:864@2026-11-03 00:00:00,EVM-1-aa-2026-11-04:1@2026-11-04 00:00:00"
		)
	);
	let app_row = text(
		&db,
		"SELECT first_seen_at || ' ' || lifetime_bytes_consumed || ' ' || lifetime_subscriptions
		 FROM app.bandwidth_apps WHERE id = 'EVM-1-aa'",
	)
	.await;
	assert_eq!(app_row.as_deref(), Some("1793711196000 865 0"));
}

#[tokio::test]
async fn tier_set_activates_and_revokes() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	apply_tier_set(&db, 3, Some((1_048_576, 2_592_000)), 1_779_761_718_000)
		.await
		.unwrap();
	let tier = text(
		&db,
		"SELECT tier || ' ' || bytes || ' ' || duration_secs || ' ' || is_active || ' ' || last_updated_at
		 FROM app.bandwidth_tiers WHERE id = '3'",
	)
	.await;
	assert_eq!(tier.as_deref(), Some("3 1048576 2592000 true 1779761718000"));
	apply_tier_set(&db, 3, None, 1_779_761_724_000).await.unwrap();
	let tier = text(
		&db,
		"SELECT coalesce(bytes::text, 'null') || ' ' || coalesce(duration_secs::text, 'null') || ' ' ||
		   is_active || ' ' || last_updated_at
		 FROM app.bandwidth_tiers WHERE id = '3'",
	)
	.await;
	assert_eq!(tier.as_deref(), Some("null null false 1779761724000"));
}

#[tokio::test]
async fn token_supply_keeps_its_creation_time() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let first =
		Supply { total: 999_993_796, circulating: 34_993_921, timestamp_ms: 1_778_703_918_000 };
	write_supply(&db, &first).await.unwrap();
	let later =
		Supply { total: 999_993_797, circulating: 34_993_000, timestamp_ms: 1_793_703_978_000 };
	write_supply(&db, &later).await.unwrap();
	let row = text(
		&db,
		"SELECT total_supply || ' ' || circulating_supply || ' ' || last_updated_at || ' ' || created_at
		 FROM app.bridge_token_supplies WHERE id = 'hyperbridge-token-supply'",
	)
	.await;
	assert_eq!(row.as_deref(), Some("999993797 34993000 1793703978000 2026-05-13 20:25:18"));
}
