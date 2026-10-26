mod common;

use common::{event_filter, event_spec, NEXUS};
use substrate_eth_rpc::pallet_address;

const FROM: u64 = 12483540;
const MIDDLE: u64 = 12483541;
const TO: u64 = 12483542;
const SERVER_ERROR: i32 = -32000;

async fn failing_block_poisons_the_whole_range(failing: u64) {
	let harness = NEXUS.start().await;
	let spec = event_spec("ismp", "StateMachineUpdated");
	let params = || event_filter(FROM, TO, pallet_address("ismp"), spec.topic0());

	harness.rpc.fail_at(failing);
	let err = harness.logs_error(params()).await;
	assert_eq!(err.code(), SERVER_ERROR);
	assert!(err.message().contains(&format!("block {failing}")), "{}", err.message());

	harness.rpc.recover();
	let logs = harness.logs(params()).await;
	let in_block = |number: u64| logs.iter().filter(|log| log.block_number == Some(number)).count();
	assert!(in_block(FROM) > 0);
	assert!(in_block(TO) > 0);
	assert_eq!(in_block(FROM) + in_block(MIDDLE) + in_block(TO), logs.len());
	assert!(logs.windows(2).all(|pair| pair[0].block_number <= pair[1].block_number));
}

#[tokio::test]
async fn middle_block_failure() {
	failing_block_poisons_the_whole_range(MIDDLE).await;
}

#[tokio::test]
async fn first_block_failure() {
	failing_block_poisons_the_whole_range(FROM).await;
}

#[tokio::test]
async fn last_block_failure() {
	failing_block_poisons_the_whole_range(TO).await;
}
