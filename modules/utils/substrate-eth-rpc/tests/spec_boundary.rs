mod common;

use std::sync::Arc;

use alloy_primitives::B256;
use alloy_rpc_types_eth::Block;
use common::{event_filter, event_spec, log_filter, NEXUS};
use jsonrpsee::{core::client::ClientT, rpc_params};
use serde_json::json;
use substrate_eth_rpc::{blocks::BlockService, pallet_address, substrate::FixtureRpc};

const OLD_SPEC_BLOCK: u64 = 11526510;
const NEW_SPEC_BLOCK: u64 = 12300900;

struct Upgrade {
	first_new_block: u64,
	old_spec: u32,
	new_spec: u32,
}

const UPGRADES: &[Upgrade] = &[
	Upgrade { first_new_block: 11755982, old_spec: 8400, new_spec: 8500 },
	Upgrade { first_new_block: 12019344, old_spec: 8500, new_spec: 8600 },
	Upgrade { first_new_block: 12252354, old_spec: 8600, new_spec: 8700 },
];

#[tokio::test]
async fn blocks_on_both_sides_of_a_runtime_upgrade_decode() {
	let old = NEXUS.block(OLD_SPEC_BLOCK).runtime_version;
	let new = NEXUS.block(NEW_SPEC_BLOCK).runtime_version;
	assert_eq!(old.spec_name, new.spec_name);
	assert!(old.spec_version < new.spec_version, "{} vs {}", old.spec_version, new.spec_version);

	let harness = NEXUS.start().await;
	let spec = event_spec("ismp", "StateMachineUpdated");
	let ismp = pallet_address("ismp");
	for number in [NEW_SPEC_BLOCK, OLD_SPEC_BLOCK, NEW_SPEC_BLOCK] {
		let logs = harness.logs(event_filter(number, number, ismp, spec.topic0())).await;
		assert!(!logs.is_empty(), "block {number}");
		assert!(logs.iter().all(|log| log.block_number == Some(number)));
	}
}

#[tokio::test]
async fn upgrade_blocks_decode_with_the_runtime_that_produced_their_events() {
	let harness = NEXUS.start().await;
	let service = BlockService::new(
		Arc::new(FixtureRpc::load(&NEXUS.fixtures()).expect("load fixtures")),
		NEXUS.config(),
	)
	.expect("block service");

	for upgrade in UPGRADES {
		let (last_old, first_new) = (upgrade.first_new_block - 1, upgrade.first_new_block);
		let before = NEXUS.block(last_old);
		let at = NEXUS.block(first_new);
		assert_eq!(at.parent_hash, before.hash);
		assert_eq!(before.runtime_version.spec_version, upgrade.old_spec);
		assert_eq!(at.runtime_version.spec_version, upgrade.new_spec);
		assert_eq!(
			at.parent_runtime_version.as_ref().map(|version| version.spec_version),
			Some(upgrade.old_spec)
		);

		let decoded = service.block_by_number(first_new).await.unwrap().expect("block exists");
		assert_eq!(decoded.spec_version, upgrade.old_spec, "block {first_new}");
		let decoded = service.block_by_number(last_old).await.unwrap().expect("block exists");
		assert_eq!(decoded.spec_version, upgrade.old_spec, "block {last_old}");

		let logs = harness.logs(log_filter(last_old, first_new, json!({}))).await;
		assert!(logs.iter().all(|log| {
			matches!(log.block_number, Some(number) if number == last_old || number == first_new)
		}));

		let block: Option<Block<B256>> = harness
			.client
			.request("eth_getBlockByNumber", rpc_params![format!("{first_new:#x}"), false])
			.await
			.expect("eth_getBlockByNumber");
		let block = block.expect("block exists");
		assert_eq!(block.header.hash, at.hash);
		assert_eq!(block.header.parent_hash, before.hash);
		assert_eq!(block.header.number, first_new);
	}
}
