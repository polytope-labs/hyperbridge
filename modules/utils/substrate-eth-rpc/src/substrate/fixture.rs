use std::{
	collections::{BTreeMap, HashMap, HashSet},
	path::Path,
	sync::Mutex,
};

use alloy_primitives::Bytes;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{BlockHeader, RawBlock, RpcError, RuntimeVersion, SubstrateRpc};
use crate::types::H256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureBlock {
	pub hash: H256,
	pub parent_hash: H256,
	pub number: u64,
	pub state_root: H256,
	pub extrinsics_root: H256,
	pub extrinsics: Vec<Bytes>,
	#[serde(default)]
	pub storage: BTreeMap<Bytes, Bytes>,
	#[serde(default)]
	pub child_storage: BTreeMap<Bytes, BTreeMap<Bytes, Bytes>>,
	pub runtime_version: FixtureRuntimeVersion,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub parent_runtime_version: Option<FixtureRuntimeVersion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureRuntimeVersion {
	pub spec_name: String,
	pub spec_version: u32,
	pub transaction_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureFinalized {
	pub number: u64,
}

impl FixtureBlock {
	fn header(&self) -> BlockHeader {
		BlockHeader {
			hash: self.hash,
			number: self.number,
			parent_hash: self.parent_hash,
			state_root: self.state_root,
			extrinsics_root: self.extrinsics_root,
		}
	}

	fn raw(&self) -> RawBlock {
		RawBlock {
			hash: self.hash,
			number: self.number,
			parent_hash: self.parent_hash,
			state_root: self.state_root,
			extrinsics_root: self.extrinsics_root,
			extrinsics: self.extrinsics.iter().map(|ext| ext.to_vec()).collect(),
		}
	}
}

pub struct FixtureRpc {
	blocks: HashMap<u64, FixtureBlock>,
	numbers: HashMap<H256, u64>,
	children: HashMap<H256, u64>,
	metadata: HashMap<u32, Vec<u8>>,
	finalized: u64,
	failing: Mutex<HashSet<u64>>,
}

impl FixtureRpc {
	pub fn load(dir: &Path) -> Result<Self, RpcError> {
		let fixture = |err: std::io::Error, path: &Path| {
			RpcError::Fixture(format!("{}: {err}", path.display()))
		};
		let parse = |err: serde_json::Error, path: &Path| {
			RpcError::Fixture(format!("{}: {err}", path.display()))
		};

		let mut blocks = Vec::new();
		let blocks_dir = dir.join("blocks");
		for entry in std::fs::read_dir(&blocks_dir).map_err(|err| fixture(err, &blocks_dir))? {
			let path = entry.map_err(|err| fixture(err, &blocks_dir))?.path();
			if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
				continue;
			}
			let raw = std::fs::read_to_string(&path).map_err(|err| fixture(err, &path))?;
			let block: FixtureBlock =
				serde_json::from_str(&raw).map_err(|err| parse(err, &path))?;
			blocks.push(block);
		}

		let mut metadata = HashMap::new();
		let metadata_dir = dir.join("metadata");
		if metadata_dir.is_dir() {
			for entry in
				std::fs::read_dir(&metadata_dir).map_err(|err| fixture(err, &metadata_dir))?
			{
				let path = entry.map_err(|err| fixture(err, &metadata_dir))?.path();
				if path.extension().and_then(|ext| ext.to_str()) != Some("scale") {
					continue;
				}
				let spec_version = path
					.file_stem()
					.and_then(|stem| stem.to_str())
					.and_then(|stem| stem.parse::<u32>().ok())
					.ok_or_else(|| {
						RpcError::Fixture(format!(
							"{}: metadata file name must be <specVersion>.scale",
							path.display()
						))
					})?;
				let bytes = std::fs::read(&path).map_err(|err| fixture(err, &path))?;
				metadata.insert(spec_version, bytes);
			}
		}

		let finalized_path = dir.join("finalized.json");
		let raw = std::fs::read_to_string(&finalized_path)
			.map_err(|err| fixture(err, &finalized_path))?;
		let finalized: FixtureFinalized =
			serde_json::from_str(&raw).map_err(|err| parse(err, &finalized_path))?;

		Ok(Self::from_blocks(blocks, metadata, finalized.number))
	}

	pub fn from_blocks(
		blocks: Vec<FixtureBlock>,
		metadata: HashMap<u32, Vec<u8>>,
		finalized: u64,
	) -> Self {
		let numbers = blocks.iter().map(|block| (block.hash, block.number)).collect();
		let children = blocks.iter().map(|block| (block.parent_hash, block.number)).collect();
		let blocks = blocks.into_iter().map(|block| (block.number, block)).collect();
		Self { blocks, numbers, children, metadata, finalized, failing: Mutex::new(HashSet::new()) }
	}

	pub fn fail_at(&self, number: u64) {
		self.failing.lock().expect("fixture lock").insert(number);
	}

	pub fn recover(&self) {
		self.failing.lock().expect("fixture lock").clear();
	}

	fn check_failure(&self, number: u64) -> Result<(), RpcError> {
		if self.failing.lock().expect("fixture lock").contains(&number) {
			return Err(RpcError::Transport(format!("injected failure at block {number}")));
		}
		Ok(())
	}

	fn block_at(&self, hash: H256) -> Result<&FixtureBlock, RpcError> {
		let number = *self.numbers.get(&hash).ok_or(RpcError::HashNotFound(hash))?;
		self.check_failure(number)?;
		Ok(&self.blocks[&number])
	}

	fn block_if_known(&self, hash: H256) -> Result<Option<&FixtureBlock>, RpcError> {
		match self.block_at(hash) {
			Ok(block) => Ok(Some(block)),
			Err(RpcError::HashNotFound(_)) => Ok(None),
			Err(err) => Err(err),
		}
	}

	// Parents of recorded blocks are usually not recorded themselves, so a parent hash answers
	// with the child's `parentRuntimeVersion`.
	fn runtime_version_at(&self, at: H256) -> Result<&FixtureRuntimeVersion, RpcError> {
		if let Some(block) = self.block_if_known(at)? {
			return Ok(&block.runtime_version);
		}
		let number = *self.children.get(&at).ok_or(RpcError::HashNotFound(at))?;
		self.check_failure(number)?;
		let block = &self.blocks[&number];
		Ok(block.parent_runtime_version.as_ref().unwrap_or(&block.runtime_version))
	}
}

#[async_trait]
impl SubstrateRpc for FixtureRpc {
	async fn finalized_head(&self) -> Result<H256, RpcError> {
		self.blocks
			.get(&self.finalized)
			.map(|block| block.hash)
			.ok_or_else(|| RpcError::Fixture(format!("finalized block {} missing", self.finalized)))
	}

	async fn block_hash(&self, number: u64) -> Result<Option<H256>, RpcError> {
		self.check_failure(number)?;
		Ok(self.blocks.get(&number).map(|block| block.hash))
	}

	async fn header(&self, hash: H256) -> Result<Option<BlockHeader>, RpcError> {
		Ok(self.block_if_known(hash)?.map(FixtureBlock::header))
	}

	async fn block(&self, hash: H256) -> Result<Option<RawBlock>, RpcError> {
		Ok(self.block_if_known(hash)?.map(FixtureBlock::raw))
	}

	async fn storage(&self, key: &[u8], at: H256) -> Result<Option<Vec<u8>>, RpcError> {
		let block = self.block_at(at)?;
		Ok(block.storage.get(key).map(|value| value.to_vec()))
	}

	async fn child_storage(
		&self,
		child_key: &[u8],
		key: &[u8],
		at: H256,
	) -> Result<Option<Vec<u8>>, RpcError> {
		let block = self.block_at(at)?;
		Ok(block
			.child_storage
			.get(child_key)
			.and_then(|child| child.get(key))
			.map(|value| value.to_vec()))
	}

	async fn runtime_version(&self, at: H256) -> Result<RuntimeVersion, RpcError> {
		let version = self.runtime_version_at(at)?;
		Ok(RuntimeVersion {
			spec_name: version.spec_name.clone(),
			spec_version: version.spec_version,
			transaction_version: version.transaction_version,
		})
	}

	async fn metadata(&self, at: H256) -> Result<Vec<u8>, RpcError> {
		let spec_version = self.runtime_version_at(at)?.spec_version;
		self.metadata.get(&spec_version).cloned().ok_or_else(|| {
			RpcError::Fixture(format!("metadata for spec version {spec_version} missing"))
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::substrate::{system_events_key, timestamp_now_key};
	use alloy_primitives::{b256, bytes};

	const HASH_1: H256 = b256!("0101010101010101010101010101010101010101010101010101010101010101");
	const HASH_2: H256 = b256!("0202020202020202020202020202020202020202020202020202020202020202");
	const UNKNOWN: H256 = b256!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");

	fn version() -> FixtureRuntimeVersion {
		FixtureRuntimeVersion { spec_name: "nexus".into(), spec_version: 7, transaction_version: 1 }
	}

	fn block(number: u64, hash: H256, parent: H256) -> FixtureBlock {
		let mut storage = BTreeMap::new();
		storage.insert(Bytes::from(timestamp_now_key()), bytes!("0102030405060708"));
		let mut child = BTreeMap::new();
		child.insert(bytes!("aa"), bytes!("bb"));
		let mut child_storage = BTreeMap::new();
		child_storage.insert(bytes!("0a0b"), child);
		FixtureBlock {
			hash,
			parent_hash: parent,
			number,
			state_root: H256::repeat_byte(0x33),
			extrinsics_root: H256::repeat_byte(0x44),
			extrinsics: vec![bytes!("0401"), bytes!("0402")],
			storage,
			child_storage,
			runtime_version: version(),
			parent_runtime_version: None,
		}
	}

	fn fixture() -> FixtureRpc {
		let blocks = vec![block(1, HASH_1, H256::ZERO), block(2, HASH_2, HASH_1)];
		let metadata = HashMap::from([(7u32, b"meta".to_vec())]);
		FixtureRpc::from_blocks(blocks, metadata, 2)
	}

	#[tokio::test]
	async fn round_trips_every_method() {
		let rpc = fixture();
		assert_eq!(rpc.finalized_head().await.unwrap(), HASH_2);
		assert_eq!(rpc.block_hash(1).await.unwrap(), Some(HASH_1));
		assert_eq!(rpc.block_hash(3).await.unwrap(), None);

		let raw = rpc.block(HASH_2).await.unwrap().unwrap();
		assert_eq!(raw.number, 2);
		assert_eq!(raw.parent_hash, HASH_1);
		assert_eq!(raw.extrinsics, vec![vec![0x04, 0x01], vec![0x04, 0x02]]);
		assert_eq!(rpc.block(UNKNOWN).await.unwrap(), None);

		assert_eq!(
			rpc.storage(&timestamp_now_key(), HASH_1).await.unwrap(),
			Some(vec![1, 2, 3, 4, 5, 6, 7, 8])
		);
		assert_eq!(rpc.storage(&system_events_key(), HASH_1).await.unwrap(), None);
		assert!(matches!(
			rpc.storage(&timestamp_now_key(), UNKNOWN).await,
			Err(RpcError::HashNotFound(_))
		));

		assert_eq!(
			rpc.child_storage(&[0x0a, 0x0b], &[0xaa], HASH_1).await.unwrap(),
			Some(vec![0xbb])
		);
		assert_eq!(rpc.child_storage(&[0x0a, 0x0b], &[0xcc], HASH_1).await.unwrap(), None);
		assert_eq!(rpc.child_storage(&[0x00], &[0xaa], HASH_1).await.unwrap(), None);

		let version = rpc.runtime_version(HASH_1).await.unwrap();
		assert_eq!(version.spec_name, "nexus");
		assert_eq!(version.spec_version, 7);
		assert_eq!(version.transaction_version, 1);

		assert_eq!(rpc.metadata(HASH_2).await.unwrap(), b"meta".to_vec());
		assert_eq!(rpc.block_by_number(2).await.unwrap(), raw);
	}

	#[tokio::test]
	async fn parent_hash_answers_with_parent_runtime_version() {
		let mut upgraded = block(2, HASH_2, HASH_1);
		upgraded.runtime_version.spec_version = 8;
		upgraded.parent_runtime_version = Some(version());
		let blocks = vec![upgraded];
		let metadata = HashMap::from([(7u32, b"old".to_vec()), (8u32, b"new".to_vec())]);
		let rpc = FixtureRpc::from_blocks(blocks, metadata, 2);

		assert_eq!(rpc.runtime_version(HASH_2).await.unwrap().spec_version, 8);
		assert_eq!(rpc.metadata(HASH_2).await.unwrap(), b"new".to_vec());
		assert_eq!(rpc.runtime_version(HASH_1).await.unwrap().spec_version, 7);
		assert_eq!(rpc.metadata(HASH_1).await.unwrap(), b"old".to_vec());
		assert!(matches!(rpc.runtime_version(UNKNOWN).await, Err(RpcError::HashNotFound(_))));
		assert!(rpc.block(HASH_1).await.unwrap().is_none());

		rpc.fail_at(2);
		assert!(rpc.runtime_version(HASH_1).await.is_err());
	}

	#[tokio::test]
	async fn parent_hash_falls_back_to_own_runtime_version() {
		let rpc = fixture();
		assert_eq!(rpc.runtime_version(H256::ZERO).await.unwrap().spec_version, 7);
		assert_eq!(rpc.metadata(H256::ZERO).await.unwrap(), b"meta".to_vec());
	}

	#[tokio::test]
	async fn injected_failure_is_scoped_and_recoverable() {
		let rpc = fixture();
		rpc.fail_at(1);

		let err = rpc.block(HASH_1).await.unwrap_err();
		assert!(matches!(err, RpcError::Transport(ref msg) if msg.contains("block 1")), "{err}");
		assert!(rpc.block_hash(1).await.is_err());
		assert!(rpc.storage(&timestamp_now_key(), HASH_1).await.is_err());
		assert!(rpc.runtime_version(HASH_1).await.is_err());
		assert!(rpc.metadata(HASH_1).await.is_err());
		assert!(rpc.block(HASH_2).await.unwrap().is_some());

		rpc.recover();
		assert!(rpc.block(HASH_1).await.unwrap().is_some());
	}

	#[tokio::test]
	async fn header_matches_block_and_respects_fail_at() {
		let rpc = fixture();
		let header = rpc.header(HASH_2).await.unwrap().unwrap();
		let raw = rpc.block(HASH_2).await.unwrap().unwrap();
		assert_eq!(header.hash, HASH_2);
		assert_eq!(header.number, raw.number);
		assert_eq!(header.parent_hash, raw.parent_hash);
		assert_eq!(header.state_root, raw.state_root);
		assert_eq!(header.extrinsics_root, raw.extrinsics_root);
		assert_eq!(rpc.header(UNKNOWN).await.unwrap(), None);

		rpc.fail_at(2);
		assert!(matches!(rpc.header(HASH_2).await, Err(RpcError::Transport(_))));
		assert!(rpc.header(HASH_1).await.unwrap().is_some());

		rpc.recover();
		assert_eq!(rpc.header(HASH_2).await.unwrap(), Some(header));
	}

	#[tokio::test]
	async fn batch_blocks_surfaces_failures() {
		let rpc = fixture();
		let blocks = rpc.batch_blocks(1..3).await.unwrap();
		assert_eq!(blocks.iter().map(|block| block.number).collect::<Vec<_>>(), vec![1, 2]);

		rpc.fail_at(2);
		assert!(rpc.batch_blocks(1..3).await.is_err());
		assert!(matches!(rpc.batch_blocks(1..4).await, Err(RpcError::Transport(_))));

		rpc.recover();
		assert!(matches!(rpc.batch_blocks(1..4).await, Err(RpcError::BlockNotFound(3))));
	}

	#[test]
	fn fixture_block_json_round_trip() {
		let json = r#"{
			"hash": "0x0101010101010101010101010101010101010101010101010101010101010101",
			"parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"number": 1,
			"stateRoot": "0x3333333333333333333333333333333333333333333333333333333333333333",
			"extrinsicsRoot": "0x4444444444444444444444444444444444444444444444444444444444444444",
			"extrinsics": ["0x0401", "0x0402"],
			"storage": {
				"0xf0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb": "0x0102030405060708"
			},
			"childStorage": { "0x0a0b": { "0xaa": "0xbb" } },
			"runtimeVersion": { "specName": "nexus", "specVersion": 7, "transactionVersion": 1 }
		}"#;
		let parsed: FixtureBlock = serde_json::from_str(json).unwrap();
		assert_eq!(parsed, block(1, HASH_1, H256::ZERO));

		let reencoded = serde_json::to_string(&parsed).unwrap();
		let reparsed: FixtureBlock = serde_json::from_str(&reencoded).unwrap();
		assert_eq!(reparsed, parsed);

		let value: serde_json::Value = serde_json::from_str(&reencoded).unwrap();
		assert_eq!(value["childStorage"]["0x0a0b"]["0xaa"], "0xbb");
		assert_eq!(value["runtimeVersion"]["specVersion"], 7);
		assert!(value.get("parentRuntimeVersion").is_none());

		let mut with_parent = parsed.clone();
		with_parent.parent_runtime_version = Some(version());
		let value: serde_json::Value =
			serde_json::from_str(&serde_json::to_string(&with_parent).unwrap()).unwrap();
		assert_eq!(value["parentRuntimeVersion"]["specVersion"], 7);
	}

	#[test]
	fn missing_maps_default_to_empty() {
		let json = r#"{
			"hash": "0x0101010101010101010101010101010101010101010101010101010101010101",
			"parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"number": 1,
			"stateRoot": "0x3333333333333333333333333333333333333333333333333333333333333333",
			"extrinsicsRoot": "0x4444444444444444444444444444444444444444444444444444444444444444",
			"extrinsics": [],
			"runtimeVersion": { "specName": "nexus", "specVersion": 7, "transactionVersion": 1 }
		}"#;
		let parsed: FixtureBlock = serde_json::from_str(json).unwrap();
		assert!(parsed.storage.is_empty());
		assert!(parsed.child_storage.is_empty());
		assert!(parsed.parent_runtime_version.is_none());
	}
}
