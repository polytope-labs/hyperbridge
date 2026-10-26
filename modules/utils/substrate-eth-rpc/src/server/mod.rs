pub mod error;
pub mod eth;
pub mod router;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use jsonrpsee::server::ServerHandle;

use crate::{blocks::BlockService, config::Config, substrate::LiveRpc};

pub use eth::{to_rpc_block, to_rpc_log, ChainRpc};

pub async fn run(config: Config) -> anyhow::Result<()> {
	let mut services = Vec::with_capacity(config.chains.len());
	for chain in config.chains {
		let rpc = LiveRpc::connect(&chain.rpc_urls())
			.await
			.with_context(|| format!("chain {}: connect", chain.name))?;
		let name = chain.name.clone();
		let service = BlockService::new(Arc::new(rpc), chain)
			.with_context(|| format!("chain {name}: block service"))?;
		services.push((name, Arc::new(service)));
	}

	let (_, handle) = start_with_services(config.listen, services).await?;
	tokio::select! {
		_ = handle.clone().stopped() => {},
		_ = tokio::signal::ctrl_c() => {
			tracing::info!("shutting down");
			let _ = handle.stop();
		},
	}
	handle.stopped().await;
	Ok(())
}

pub async fn start_with_services(
	listen: SocketAddr,
	services: Vec<(String, Arc<BlockService>)>,
) -> anyhow::Result<(SocketAddr, ServerHandle)> {
	let chains = services
		.into_iter()
		.map(|(name, service)| (name, ChainRpc::new(service).into_module().into()))
		.collect::<Vec<_>>();
	let names: Vec<String> = chains.iter().map(|(name, _)| name.clone()).collect();
	let (addr, handle) = router::serve(listen, chains).await?;
	tracing::info!(%addr, "listening");
	for name in names {
		tracing::info!(path = %format!("/{name}"), chain = %name, "serving chain");
	}
	Ok((addr, handle))
}

#[cfg(test)]
mod tests {
	use std::collections::{BTreeMap, HashMap};

	use alloy_primitives::{hex, keccak256, Address, Bytes, B256};
	use alloy_rpc_types_eth::{Block, Log};
	use alloy_sol_types::{sol_data, SolCall, SolType};
	use codec::Encode;
	use jsonrpsee::{
		core::{
			client::{ClientT, Error as ClientError},
			params::{ArrayParams, BatchRequestBuilder},
		},
		http_client::{HttpClient, HttpClientBuilder},
		rpc_params,
		types::ErrorObjectOwned,
	};
	use serde_json::json;

	use super::{eth::SubstrateStorage, *};
	use crate::{
		config::ChainConfig,
		decode::testdata::NEXUS,
		mapping::pallets_for,
		substrate::{
			fixture::FixtureRuntimeVersion, system_events_key, timestamp_now_key, FixtureBlock,
			FixtureRpc,
		},
		types::{pallet_address, storage_address, Hasher},
	};

	const METADATA: &[u8] = NEXUS.metadata;
	const EVENTS: &[u8] = NEXUS.events;

	const SPEC_VERSION: u32 = 8700;
	const TIMESTAMP_MS: u64 = 1_700_000_123_456;
	const FINALIZED: u64 = 102;
	const CHAIN_ID: u64 = 1000003367;

	fn hash_of(number: u64) -> B256 {
		B256::repeat_byte(number as u8)
	}

	fn fixture_block(number: u64, events: &[u8]) -> FixtureBlock {
		let mut storage = BTreeMap::new();
		storage.insert(Bytes::from(system_events_key()), Bytes::from(events.to_vec()));
		storage.insert(Bytes::from(timestamp_now_key()), Bytes::from(TIMESTAMP_MS.encode()));
		FixtureBlock {
			hash: hash_of(number),
			parent_hash: hash_of(number - 1),
			number,
			state_root: B256::repeat_byte(0x33),
			extrinsics_root: B256::repeat_byte(0x44),
			extrinsics: NEXUS.extrinsics().into_iter().map(Bytes::from).collect(),
			storage,
			child_storage: BTreeMap::new(),
			runtime_version: FixtureRuntimeVersion {
				spec_name: "nexus".into(),
				spec_version: SPEC_VERSION,
				transaction_version: 1,
			},
			parent_runtime_version: None,
		}
	}

	fn fixture() -> Arc<FixtureRpc> {
		let empty = Vec::<u8>::new().encode();
		let blocks = vec![
			fixture_block(100, EVENTS),
			fixture_block(101, &empty),
			fixture_block(102, EVENTS),
		];
		let metadata = HashMap::from([(SPEC_VERSION, METADATA.to_vec())]);
		Arc::new(FixtureRpc::from_blocks(blocks, metadata, FINALIZED))
	}

	fn chain(max_block_range: u64) -> ChainConfig {
		ChainConfig {
			name: "nexus".into(),
			rpc: String::new(),
			eth_chain_id: CHAIN_ID,
			hasher: Hasher::Keccak,
			pallets: vec!["ismp".into(), "consensusIncentives".into()],
			max_block_range,
			cache_blocks: 64,
		}
	}

	struct Harness {
		rpc: Arc<FixtureRpc>,
		addr: SocketAddr,
		_handle: ServerHandle,
	}

	impl Harness {
		async fn start(max_block_range: u64) -> Self {
			let rpc = fixture();
			let service = Arc::new(BlockService::new(rpc.clone(), chain(max_block_range)).unwrap());
			let listen: SocketAddr = "127.0.0.1:0".parse().unwrap();
			let (addr, handle) =
				start_with_services(listen, vec![("nexus".into(), service)]).await.unwrap();
			Self { rpc, addr, _handle: handle }
		}

		fn client_at(&self, path: &str) -> HttpClient {
			HttpClientBuilder::default()
				.build(format!("http://{}{path}", self.addr))
				.unwrap()
		}

		fn client(&self) -> HttpClient {
			self.client_at("/nexus")
		}
	}

	fn state_machine_updated_topic() -> B256 {
		keccak256(b"StateMachineUpdated((string,bytes4),uint64)")
	}

	fn ismp() -> Address {
		pallet_address("ismp")
	}

	fn log_filter(from: &str, to: &str, extra: serde_json::Value) -> ArrayParams {
		let mut filter = json!({ "fromBlock": from, "toBlock": to });
		filter.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
		rpc_params![filter]
	}

	async fn get_logs(client: &HttpClient, params: ArrayParams) -> Vec<Log> {
		client.request("eth_getLogs", params).await.unwrap()
	}

	async fn call_error(
		client: &HttpClient,
		method: &str,
		params: ArrayParams,
	) -> ErrorObjectOwned {
		match client.request::<serde_json::Value, _>(method, params).await {
			Err(ClientError::Call(err)) => err,
			other => panic!("expected a JSON-RPC error, got {other:?}"),
		}
	}

	#[tokio::test]
	async fn chain_identity_and_head() {
		let harness = Harness::start(1000).await;
		let client = harness.client();

		let chain_id: String = client.request("eth_chainId", rpc_params![]).await.unwrap();
		assert_eq!(chain_id, format!("{CHAIN_ID:#x}"));
		let version: String = client.request("net_version", rpc_params![]).await.unwrap();
		assert_eq!(version, CHAIN_ID.to_string());
		let head: String = client.request("eth_blockNumber", rpc_params![]).await.unwrap();
		assert_eq!(head, "0x66");
	}

	#[tokio::test]
	async fn get_block_by_number_and_hash() {
		let harness = Harness::start(1000).await;
		let client = harness.client();

		let block: Block = client
			.request("eth_getBlockByNumber", rpc_params!["latest", false])
			.await
			.unwrap();
		assert_eq!(block.header.number, 102);
		assert_eq!(block.header.hash, hash_of(102));
		assert_eq!(block.header.parent_hash, hash_of(101));
		assert_eq!(block.header.timestamp, TIMESTAMP_MS / 1000);
		assert_eq!(block.header.transactions_root, B256::repeat_byte(0x44));
		assert_eq!(block.header.state_root, B256::repeat_byte(0x33));
		assert!(!block.header.logs_bloom.is_zero());
		assert_eq!(block.transactions.hashes().count(), 4);
		assert!(block.uncles.is_empty());

		let raw: serde_json::Value = client
			.request("eth_getBlockByNumber", rpc_params!["0x65", false])
			.await
			.unwrap();
		assert_eq!(raw["number"], "0x65");
		assert_eq!(raw["timestamp"], format!("{:#x}", TIMESTAMP_MS / 1000));
		assert_eq!(raw["gasLimit"], "0x0");
		assert_eq!(raw["miner"], format!("{}", Address::ZERO));
		assert!(raw.get("baseFeePerGas").is_none());
		assert!(raw.get("size").is_none());
		assert!(raw.get("totalDifficulty").is_none());
		let empty: Block = serde_json::from_value(raw).unwrap();
		assert!(empty.header.logs_bloom.is_zero());

		let missing: Option<Block> = client
			.request("eth_getBlockByNumber", rpc_params!["0x67", false])
			.await
			.unwrap();
		assert!(missing.is_none());

		let by_hash: Block = client
			.request("eth_getBlockByHash", rpc_params![hash_of(100), false])
			.await
			.unwrap();
		assert_eq!(by_hash.header.number, 100);
		let unknown: Option<Block> = client
			.request("eth_getBlockByHash", rpc_params![B256::repeat_byte(0xff), false])
			.await
			.unwrap();
		assert!(unknown.is_none());
	}

	#[tokio::test]
	async fn batch_preserves_order() {
		let harness = Harness::start(1000).await;
		let client = harness.client();

		let mut batch = BatchRequestBuilder::new();
		for tag in ["0x66", "0x64", "0x65"] {
			batch.insert("eth_getBlockByNumber", rpc_params![tag, false]).unwrap();
		}
		let responses = client.batch_request::<Block>(batch).await.unwrap();
		let numbers: Vec<u64> =
			responses.into_ok().unwrap().map(|block| block.header.number).collect();
		assert_eq!(numbers, vec![102, 100, 101]);
	}

	#[tokio::test]
	async fn get_logs_filters_by_address_and_topics() {
		let harness = Harness::start(1000).await;
		let client = harness.client();
		let topic0 = state_machine_updated_topic();

		let logs =
			get_logs(&client, log_filter("0x64", "0x66", json!({ "address": ismp() }))).await;
		let updates: Vec<&Log> = logs.iter().filter(|log| log.topics()[0] == topic0).collect();
		let in_block =
			|number: u64| updates.iter().filter(|log| log.block_number == Some(number)).count();
		assert!(in_block(100) > 0);
		assert_eq!(in_block(100), in_block(102));
		assert_eq!(in_block(100) + in_block(102), updates.len());
		assert!(updates.windows(2).all(|pair| pair[0].block_number <= pair[1].block_number));
		for log in &logs {
			assert_eq!(log.address(), ismp());
			assert_eq!(log.block_timestamp, Some(TIMESTAMP_MS / 1000));
			assert!(!log.removed);
			assert!(log.transaction_hash.is_some());
			assert!(log.log_index.is_some());
		}

		let as_array =
			get_logs(&client, log_filter("0x64", "0x66", json!({ "address": [ismp()] }))).await;
		assert_eq!(as_array.len(), logs.len());

		let incentives = pallet_address("consensusIncentives");
		let rewards =
			get_logs(&client, log_filter("0x64", "0x66", json!({ "address": incentives }))).await;
		assert!(!rewards.is_empty());
		assert!(rewards.iter().all(|log| log.address() == incentives));
		let rewarded = pallets_for(&["consensusIncentives".into()]).unwrap()[0].events[0].topic0();
		assert!(rewards.iter().any(|log| log.topics()[0] == rewarded));

		let by_topic =
			get_logs(&client, log_filter("0x64", "0x66", json!({ "topics": [topic0] }))).await;
		assert_eq!(by_topic.len(), updates.len());
		assert!(by_topic.iter().all(|log| log.topics()[0] == topic0));

		let none = get_logs(
			&client,
			log_filter("0x64", "0x66", json!({ "topics": [B256::repeat_byte(0xaa)] })),
		)
		.await;
		assert!(none.is_empty());

		let unknown_address = get_logs(
			&client,
			log_filter("0x64", "0x66", json!({ "address": Address::repeat_byte(0x11) })),
		)
		.await;
		assert!(unknown_address.is_empty());

		let empty_block = get_logs(&client, log_filter("0x65", "0x65", json!({}))).await;
		assert!(empty_block.is_empty());

		let tagged = get_logs(
			&client,
			log_filter("finalized", "latest", json!({ "address": ismp(), "topics": [topic0] })),
		)
		.await;
		assert_eq!(tagged.len(), in_block(102));
		assert!(tagged.iter().all(|log| log.block_number == Some(102)));
	}

	#[tokio::test]
	async fn get_logs_log_shape() {
		let harness = Harness::start(1000).await;
		let client = harness.client();
		let topic0 = state_machine_updated_topic();

		let raw: Vec<serde_json::Value> = client
			.request(
				"eth_getLogs",
				log_filter("0x64", "0x64", json!({ "address": ismp(), "topics": [topic0] })),
			)
			.await
			.unwrap();
		assert!(raw.iter().all(|log| log["blockNumber"] == "0x64"));
		let log = raw.iter().find(|log| log["logIndex"] == "0x8").expect("log index 8");
		assert_eq!(log["address"].as_str().unwrap().parse::<Address>().unwrap(), ismp());
		assert_eq!(log["address"], hex::encode_prefixed(ismp()));
		assert_eq!(log["topics"][0], format!("{topic0}"));
		assert_eq!(log["blockHash"], format!("{}", hash_of(100)));
		assert_eq!(log["blockTimestamp"], format!("{:#x}", TIMESTAMP_MS / 1000));
		assert_eq!(log["transactionIndex"], "0x2");
		assert_eq!(log["removed"], false);
		assert!(log["data"].as_str().unwrap().starts_with("0x"));
		assert!(log["transactionHash"].as_str().unwrap().starts_with("0x"));
	}

	#[tokio::test]
	async fn get_logs_is_all_or_nothing_and_recovers() {
		let harness = Harness::start(1000).await;
		let client = harness.client();
		let params = || {
			log_filter(
				"0x64",
				"0x66",
				json!({ "address": ismp(), "topics": [state_machine_updated_topic()] }),
			)
		};

		harness.rpc.fail_at(101);
		let err = call_error(&client, "eth_getLogs", params()).await;
		assert_eq!(err.code(), error::SERVER_ERROR);
		assert!(err.message().contains("block 101"), "{}", err.message());

		harness.rpc.recover();
		let logs = get_logs(&client, params()).await;
		let in_block =
			|number: u64| logs.iter().filter(|log| log.block_number == Some(number)).count();
		assert!(in_block(100) > 0);
		assert_eq!(in_block(100), in_block(102));
		assert_eq!(in_block(100) + in_block(102), logs.len());
	}

	#[tokio::test]
	async fn get_logs_range_errors() {
		let harness = Harness::start(2).await;
		let client = harness.client();

		let err = call_error(&client, "eth_getLogs", log_filter("0x64", "0x66", json!({}))).await;
		assert_eq!(err.code(), error::SERVER_ERROR);
		assert_eq!(err.message(), "this block range should work: [0x64, 0x65]");

		let err = call_error(&client, "eth_getLogs", log_filter("0x66", "0x67", json!({}))).await;
		assert_eq!(err.code(), error::SERVER_ERROR);
		assert_eq!(err.message(), "toBlock 0x67 is beyond finalized head 0x66");

		let err = call_error(&client, "eth_getLogs", log_filter("0x65", "0x64", json!({}))).await;
		assert_eq!(err.code(), error::INVALID_PARAMS);

		let err = call_error(
			&client,
			"eth_getLogs",
			rpc_params![json!({ "blockHash": hash_of(100), "address": ismp() })],
		)
		.await;
		assert_eq!(err.code(), error::INVALID_PARAMS);
		assert_eq!(err.message(), "blockHash filter not supported");
	}

	#[tokio::test]
	async fn eth_call_reads_storage() {
		let harness = Harness::start(1000).await;
		let client = harness.client();

		let data =
			SubstrateStorage::getStorageCall { key: timestamp_now_key().into() }.abi_encode();
		let ret: Bytes = client
			.request(
				"eth_call",
				rpc_params![json!({ "to": storage_address(), "data": Bytes::from(data) }), "0x64"],
			)
			.await
			.unwrap();
		assert_eq!(sol_data::Bytes::abi_decode(&ret).unwrap(), TIMESTAMP_MS.encode());

		let data =
			SubstrateStorage::getStorageCall { key: b"missing".to_vec().into() }.abi_encode();
		let ret: Bytes = client
			.request(
				"eth_call",
				rpc_params![
					json!({ "to": storage_address(), "input": Bytes::from(data) }),
					"latest"
				],
			)
			.await
			.unwrap();
		assert!(sol_data::Bytes::abi_decode(&ret).unwrap().is_empty());

		let data = SubstrateStorage::getChildStorageCall {
			childKey: b"child".to_vec().into(),
			key: b"key".to_vec().into(),
		}
		.abi_encode();
		let ret: Bytes = client
			.request(
				"eth_call",
				rpc_params![json!({ "to": storage_address(), "data": Bytes::from(data) })],
			)
			.await
			.unwrap();
		assert!(sol_data::Bytes::abi_decode(&ret).unwrap().is_empty());
	}

	#[tokio::test]
	async fn eth_call_rejects_other_targets_and_selectors() {
		let harness = Harness::start(1000).await;
		let client = harness.client();

		let data =
			SubstrateStorage::getStorageCall { key: timestamp_now_key().into() }.abi_encode();
		let err = call_error(
			&client,
			"eth_call",
			rpc_params![json!({ "to": ismp(), "data": Bytes::from(data) }), "0x64"],
		)
		.await;
		assert_eq!(err.code(), error::INVALID_PARAMS);
		assert_eq!(err.message(), "only the storage address is callable");

		let err = call_error(
			&client,
			"eth_call",
			rpc_params![json!({ "to": storage_address(), "data": "0xdeadbeef" }), "0x64"],
		)
		.await;
		assert_eq!(err.code(), error::INVALID_PARAMS);

		let data =
			SubstrateStorage::getStorageCall { key: timestamp_now_key().into() }.abi_encode();
		let err = call_error(
			&client,
			"eth_call",
			rpc_params![json!({ "to": storage_address(), "data": Bytes::from(data) }), "0x67"],
		)
		.await;
		assert_eq!(err.code(), error::SERVER_ERROR);
		assert_eq!(err.message(), "block 103 not found");
	}

	#[tokio::test]
	async fn unknown_methods_and_paths() {
		let harness = Harness::start(1000).await;

		let err = call_error(&harness.client(), "zks_L1ChainId", rpc_params![]).await;
		assert_eq!(err.code(), -32601);

		let err = harness
			.client_at("/nope")
			.request::<String, _>("eth_chainId", rpc_params![])
			.await
			.unwrap_err();
		assert!(matches!(err, ClientError::Transport(_)), "{err}");
		assert!(err.to_string().contains("404"), "{err}");

		let root: String =
			harness.client_at("/").request("eth_chainId", rpc_params![]).await.unwrap();
		assert_eq!(root, format!("{CHAIN_ID:#x}"));
	}
}
