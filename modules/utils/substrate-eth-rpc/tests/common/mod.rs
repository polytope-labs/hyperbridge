#![allow(dead_code)]

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use alloy_primitives::{Address, B256};
use alloy_rpc_types_eth::Log;
use jsonrpsee::{
	core::{
		client::{ClientT, Error as ClientError},
		params::ArrayParams,
	},
	http_client::{HttpClient, HttpClientBuilder},
	rpc_params,
	server::ServerHandle,
	types::ErrorObjectOwned,
};
use serde_json::json;
use substrate_eth_rpc::{
	blocks::BlockService,
	mapping::{all_pallets, EventSpec},
	server::start_with_services,
	substrate::{FixtureBlock, FixtureRpc},
	ChainConfig, Hasher,
};

pub const NEXUS: Chain = Chain {
	dir: "hyperbridge-nexus",
	eth_chain_id: 1000003367,
	hasher: Hasher::Keccak,
	pallets: &[
		"ismp",
		"consensusIncentives",
		"balances",
		"relayer",
		"collatorManager",
		"bandwidth",
		"intentsCoprocessor",
	],
};

pub const GARGANTUA: Chain = Chain {
	dir: "gargantua",
	eth_chain_id: 1000004009,
	hasher: Hasher::Keccak,
	pallets: NEXUS.pallets,
};

pub struct Chain {
	pub dir: &'static str,
	pub eth_chain_id: u64,
	pub hasher: Hasher,
	pub pallets: &'static [&'static str],
}

impl Chain {
	pub fn fixtures(&self) -> PathBuf {
		PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(self.dir)
	}

	pub fn config(&self) -> ChainConfig {
		ChainConfig {
			name: self.dir.into(),
			rpc: String::new(),
			eth_chain_id: self.eth_chain_id,
			hasher: self.hasher,
			pallets: self.pallets.iter().map(|pallet| pallet.to_string()).collect(),
			max_block_range: 1000,
			cache_blocks: 64,
			upstream_concurrency: 8,
			upstream_retries: 5,
			upstream_backoff_ms: 250,
			rate_limit_cooldown_ms: 10_000,
			range_deadline_ms: 60_000,
		}
	}

	pub fn block(&self, number: u64) -> FixtureBlock {
		let path = self.fixtures().join("blocks").join(format!("{number}.json"));
		let raw = std::fs::read_to_string(&path)
			.unwrap_or_else(|err| panic!("{}: {err}", path.display()));
		serde_json::from_str(&raw).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
	}

	pub fn finalized(&self) -> u64 {
		let path = self.fixtures().join("finalized.json");
		let raw = std::fs::read_to_string(&path)
			.unwrap_or_else(|err| panic!("{}: {err}", path.display()));
		let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
		value["number"].as_u64().expect("finalized.json number")
	}

	pub async fn start(&self) -> Harness {
		let rpc = Arc::new(FixtureRpc::load(&self.fixtures()).expect("load fixtures"));
		let service =
			Arc::new(BlockService::new(rpc.clone(), self.config()).expect("block service"));
		let listen: SocketAddr = "127.0.0.1:0".parse().unwrap();
		let (addr, handle) = start_with_services(listen, vec![(self.dir.to_string(), service)])
			.await
			.expect("start server");
		let client = HttpClientBuilder::default()
			.build(format!("http://{addr}/{}", self.dir))
			.expect("http client");
		Harness { client, rpc, _handle: handle }
	}
}

pub struct Harness {
	pub client: HttpClient,
	pub rpc: Arc<FixtureRpc>,
	_handle: ServerHandle,
}

impl Harness {
	pub async fn logs(&self, params: ArrayParams) -> Vec<Log> {
		self.client.request("eth_getLogs", params).await.expect("eth_getLogs")
	}

	pub async fn logs_error(&self, params: ArrayParams) -> ErrorObjectOwned {
		match self.client.request::<Vec<Log>, _>("eth_getLogs", params).await {
			Err(ClientError::Call(err)) => err,
			other => panic!("expected a JSON-RPC error, got {other:?}"),
		}
	}
}

pub fn log_filter(from: u64, to: u64, extra: serde_json::Value) -> ArrayParams {
	let mut filter = json!({ "fromBlock": format!("{from:#x}"), "toBlock": format!("{to:#x}") });
	filter.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
	rpc_params![filter]
}

pub fn event_filter(from: u64, to: u64, address: Address, topic0: B256) -> ArrayParams {
	log_filter(from, to, json!({ "address": address, "topics": [topic0] }))
}

pub fn event_spec(pallet: &str, event: &str) -> EventSpec {
	all_pallets()
		.into_iter()
		.find(|spec| spec.key == pallet)
		.unwrap_or_else(|| panic!("unknown pallet {pallet}"))
		.events
		.into_iter()
		.find(|spec| spec.name == event)
		.unwrap_or_else(|| panic!("unknown event {pallet}.{event}"))
}
