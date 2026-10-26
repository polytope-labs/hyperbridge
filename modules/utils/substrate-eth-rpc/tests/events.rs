mod common;

use std::collections::BTreeSet;

use alloy_dyn_abi::DynSolType;
use alloy_primitives::B256;
use alloy_rpc_types_eth::Block;
use common::{event_filter, event_spec, Chain, Harness, BIFROST, NEXUS};
use jsonrpsee::{core::client::ClientT, rpc_params};
use substrate_eth_rpc::pallet_address;

struct Case {
	block: u64,
	pallet: &'static str,
	event: &'static str,
	commitments: &'static [&'static str],
}

const fn case(block: u64, pallet: &'static str, event: &'static str) -> Case {
	Case { block, pallet, event, commitments: &[] }
}

const fn ismp(block: u64, event: &'static str, commitments: &'static [&'static str]) -> Case {
	Case { block, pallet: "ismp", event, commitments }
}

const NEXUS_CASES: &[Case] = &[
	ismp(
		12300900,
		"Request",
		&["0x177b8c7eb42f44d3cdfc6a31021c42f2be4dee88bc0a563c1f34084089cf84b7"],
	),
	ismp(
		12469148,
		"Request",
		&["0x5c9f7bdd9cd53cdedb0fdb57c465283e10e7907b4a36101617ede4f48b0a6d60"],
	),
	ismp(
		12332133,
		"Response",
		&["0xa875d3d9aa959b3ec0fe63acc68072900f9b9a5ebb8dcaadd47ba0b19796c373"],
	),
	ismp(
		12469148,
		"PostRequestHandled",
		&["0x5c9f7bdd9cd53cdedb0fdb57c465283e10e7907b4a36101617ede4f48b0a6d60"],
	),
	ismp(
		11526510,
		"PostRequestHandled",
		&["0xf8afda40e7db7ffbd9290bbe87cd9486ebb60c8fc28d6b8d69c8e8356303073e"],
	),
	ismp(
		12332133,
		"GetRequestHandled",
		&["0x0f72b9decbe47c09b2acb86aecc2d7da6a17491abdda9310429007d58006fd7e"],
	),
	ismp(12483540, "StateMachineUpdated", &[]),
	ismp(12483542, "StateMachineUpdated", &[]),
	case(12483541, "consensusIncentives", "RelayerRewarded"),
	case(12483541, "balances", "Transfer"),
	case(12483541, "collatorManager", "CollatorRewarded"),
	case(11526510, "bandwidth", "BandwidthCredited"),
	case(12469337, "intentsCoprocessor", "BidPlaced"),
];

const BIFROST_CASES: &[Case] = &[ismp(
	12861850,
	"Request",
	&[
		"0x07131679de0c524d9afd2341c6b53809567f253f7806438608f0af01b540e373",
		"0x220296df7769198987de6e3c115c4ec2491925f875dcff20fa066c76a1a95d4f",
		"0x787a184a39e25e6de799f2f46ad587272c5916f4e7b3cf23e9c342fbc94c248e",
	],
)];

async fn check_case(harness: &Harness, chain: &Chain, case: &Case) {
	let spec = event_spec(case.pallet, case.event);
	let address = pallet_address(case.pallet);
	let topic0 = spec.topic0();
	let label = format!("{} block {} {}.{}", chain.dir, case.block, case.pallet, case.event);

	let logs = harness.logs(event_filter(case.block, case.block, address, topic0)).await;
	assert!(!logs.is_empty(), "{label}: no logs");

	let indexed = spec.fields.iter().filter(|field| field.indexed).count();
	let data_type = DynSolType::Tuple(
		spec.fields
			.iter()
			.filter(|field| !field.indexed)
			.map(|field| field.ty.dyn_type())
			.collect(),
	);
	let block = chain.block(case.block);
	for log in &logs {
		assert_eq!(log.address(), address, "{label}");
		assert_eq!(log.topics().len(), indexed + 1, "{label}: topic count");
		assert_eq!(log.topics()[0], topic0, "{label}");
		assert_eq!(log.block_number, Some(case.block), "{label}");
		assert_eq!(log.block_hash, Some(block.hash), "{label}");
		assert!(log.block_timestamp.is_some(), "{label}: blockTimestamp missing");
		assert!(log.transaction_hash.is_some(), "{label}: transactionHash missing");
		assert!(log.transaction_index.is_some(), "{label}: transactionIndex missing");
		assert!(log.log_index.is_some(), "{label}: logIndex missing");
		let decoded = data_type
			.abi_decode_params(&log.data().data)
			.unwrap_or_else(|err| panic!("{label}: data does not decode as {data_type}: {err}"));
		let values = decoded.as_tuple().expect("tuple");
		assert_eq!(values.len(), spec.fields.len() - indexed, "{label}: field count");
	}

	if !case.commitments.is_empty() {
		let expected: BTreeSet<B256> =
			case.commitments.iter().map(|hex| hex.parse().unwrap()).collect();
		let actual: BTreeSet<B256> = logs.iter().map(|log| log.topics()[1]).collect();
		assert_eq!(actual, expected, "{label}: commitment topics");
	}
}

async fn check_block(harness: &Harness, chain: &Chain, number: u64) {
	let fixture = chain.block(number);
	let block: Block = harness
		.client
		.request("eth_getBlockByNumber", rpc_params![format!("{number:#x}"), false])
		.await
		.expect("eth_getBlockByNumber");
	assert_eq!(block.header.hash, fixture.hash);
	assert_eq!(block.header.parent_hash, fixture.parent_hash);
	assert_eq!(block.header.number, fixture.number);
	assert_eq!(block.header.transactions_root, fixture.extrinsics_root);
	assert_eq!(block.header.state_root, fixture.state_root);
	assert_eq!(block.transactions.hashes().count(), fixture.extrinsics.len());

	let timestamp_key = substrate_eth_rpc::substrate::timestamp_now_key();
	let raw_timestamp = fixture.storage.get(&timestamp_key[..]).expect("Timestamp.Now recorded");
	let timestamp_ms = u64::from_le_bytes(raw_timestamp[..8].try_into().unwrap());
	assert_eq!(block.header.timestamp, timestamp_ms / 1000);
	assert_eq!(block.header.timestamp.to_string().len(), 10, "timestamp must be in seconds");

	let raw: serde_json::Value = harness
		.client
		.request("eth_getBlockByNumber", rpc_params![format!("{number:#x}"), false])
		.await
		.unwrap();
	assert_eq!(raw["number"], format!("{number:#x}"));
	assert_eq!(raw["timestamp"], format!("{:#x}", timestamp_ms / 1000));
	assert_eq!(raw["hash"], format!("{}", fixture.hash));
}

async fn check_log_shape(harness: &Harness, chain: &Chain, case: &Case) {
	let spec = event_spec(case.pallet, case.event);
	let raw: Vec<serde_json::Value> = harness
		.client
		.request(
			"eth_getLogs",
			event_filter(case.block, case.block, pallet_address(case.pallet), spec.topic0()),
		)
		.await
		.unwrap();
	assert!(!raw.is_empty(), "{} block {}", chain.dir, case.block);
	for log in &raw {
		assert_eq!(log["transactionHash"].as_str().unwrap().len(), 66);
		assert_eq!(log["blockHash"].as_str().unwrap().len(), 66);
		assert!(log["blockTimestamp"].as_str().unwrap().starts_with("0x"));
		assert!(log["logIndex"].as_str().unwrap().starts_with("0x"));
		assert!(log["transactionIndex"].as_str().unwrap().starts_with("0x"));
		assert_eq!(log["removed"], false);
	}
}

#[tokio::test]
async fn nexus_events_decode_against_mapping() {
	let harness = NEXUS.start().await;
	for case in NEXUS_CASES {
		check_case(&harness, &NEXUS, case).await;
	}
	check_log_shape(&harness, &NEXUS, &NEXUS_CASES[0]).await;
}

#[tokio::test]
async fn bifrost_events_decode_against_mapping() {
	let harness = BIFROST.start().await;
	for case in BIFROST_CASES {
		check_case(&harness, &BIFROST, case).await;
	}
	check_log_shape(&harness, &BIFROST, &BIFROST_CASES[0]).await;
}

#[tokio::test]
async fn nexus_block_matches_fixture() {
	let harness = NEXUS.start().await;
	check_block(&harness, &NEXUS, 12300900).await;
}

#[tokio::test]
async fn bifrost_block_matches_fixture() {
	let harness = BIFROST.start().await;
	check_block(&harness, &BIFROST, 12861850).await;
}
