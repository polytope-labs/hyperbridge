mod common;

use alloy_primitives::Address;
use common::{log_filter, BIFROST, NEXUS};
use serde_json::json;
use substrate_eth_rpc::pallet_address;

const SERVER_ERROR: i32 = -32000;

#[tokio::test]
async fn range_without_matching_events_is_empty() {
	let harness = NEXUS.start().await;
	let relayer = pallet_address("relayer");
	let logs = harness
		.logs(log_filter(12483540, 12483542, json!({ "address": relayer })))
		.await;
	assert!(logs.is_empty());

	let unmapped = Address::repeat_byte(0x11);
	let logs = harness
		.logs(log_filter(12483540, 12483542, json!({ "address": unmapped })))
		.await;
	assert!(logs.is_empty());
}

#[tokio::test]
async fn bifrost_only_serves_configured_pallets() {
	let harness = BIFROST.start().await;
	let balances = pallet_address("balances");
	let logs = harness
		.logs(log_filter(12861850, 12861850, json!({ "address": balances })))
		.await;
	assert!(logs.is_empty());
	let any = harness.logs(log_filter(12861850, 12861850, json!({}))).await;
	assert!(any.iter().all(|log| log.address() == pallet_address("ismp")));
}

#[tokio::test]
async fn to_block_beyond_finalized_is_an_error() {
	let harness = NEXUS.start().await;
	let head = NEXUS.finalized();
	let err = harness.logs_error(log_filter(head, head + 1, json!({}))).await;
	assert_eq!(err.code(), SERVER_ERROR);
	assert_eq!(
		err.message(),
		format!("toBlock {:#x} is beyond finalized head {head:#x}", head + 1)
	);

	let harness = BIFROST.start().await;
	let head = BIFROST.finalized();
	let err = harness.logs_error(log_filter(head + 1, head + 5, json!({}))).await;
	assert_eq!(err.code(), SERVER_ERROR);
	assert!(err.message().contains("beyond finalized head"), "{}", err.message());
}
