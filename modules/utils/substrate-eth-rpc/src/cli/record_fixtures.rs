use std::{collections::BTreeMap, path::Path};

use alloy_primitives::Bytes;
use anyhow::{anyhow, bail, Context};

use crate::{
	config::ChainConfig,
	substrate::{
		fixture::{FixtureFinalized, FixtureRuntimeVersion},
		system_events_key, timestamp_now_key, FixtureBlock, LiveRpc, RuntimeVersion, SubstrateRpc,
	},
};

pub fn parse_blocks(specs: &[String]) -> anyhow::Result<Vec<u64>> {
	let mut blocks = Vec::new();
	for spec in specs.iter().flat_map(|spec| spec.split(',')) {
		let spec = spec.trim();
		if spec.is_empty() {
			continue;
		}
		match spec.split_once('-') {
			Some((start, end)) => {
				let start = parse_number(start)?;
				let end = parse_number(end)?;
				if start > end {
					bail!("invalid block range {spec:?}: start is after end");
				}
				blocks.extend(start..=end);
			},
			None => blocks.push(parse_number(spec)?),
		}
	}
	blocks.sort_unstable();
	blocks.dedup();
	if blocks.is_empty() {
		bail!("no blocks given");
	}
	Ok(blocks)
}

fn parse_number(raw: &str) -> anyhow::Result<u64> {
	raw.trim().parse().map_err(|err| anyhow!("invalid block number {raw:?}: {err}"))
}

pub async fn record(chain: &ChainConfig, blocks: &[u64], out: &Path) -> anyhow::Result<()> {
	let rpc = LiveRpc::connect(&chain.rpc_urls())
		.await
		.with_context(|| format!("chain {}: connect", chain.name))?;

	let finalized_hash = rpc.finalized_head().await?;
	let finalized = rpc
		.header(finalized_hash)
		.await?
		.ok_or_else(|| anyhow!("finalized header {finalized_hash} missing"))?
		.number;
	if let Some(unfinalized) = blocks.iter().find(|number| **number > finalized) {
		bail!("block {unfinalized} is beyond finalized head {finalized}");
	}

	let blocks_dir = out.join("blocks");
	let metadata_dir = out.join("metadata");
	std::fs::create_dir_all(&blocks_dir)
		.with_context(|| format!("create {}", blocks_dir.display()))?;
	std::fs::create_dir_all(&metadata_dir)
		.with_context(|| format!("create {}", metadata_dir.display()))?;

	let events_key = system_events_key();
	let timestamp_key = timestamp_now_key();
	for &number in blocks {
		let hash = rpc
			.block_hash(number)
			.await?
			.ok_or_else(|| anyhow!("block {number} not found"))?;
		let raw = rpc.block(hash).await?.ok_or_else(|| anyhow!("block {number} body not found"))?;
		let spec_at = if number == 0 { hash } else { raw.parent_hash };
		let (version, parent_version, events, timestamp) = tokio::try_join!(
			rpc.runtime_version(hash),
			rpc.runtime_version(spec_at),
			rpc.storage(&events_key, hash),
			rpc.storage(&timestamp_key, hash),
		)?;

		let mut storage = BTreeMap::new();
		storage.insert(
			Bytes::from(events_key.clone()),
			Bytes::from(events.ok_or_else(|| anyhow!("block {number}: System.Events missing"))?),
		);
		storage.insert(
			Bytes::from(timestamp_key.clone()),
			Bytes::from(timestamp.ok_or_else(|| anyhow!("block {number}: Timestamp.Now missing"))?),
		);

		for (spec_version, at) in
			[(version.spec_version, hash), (parent_version.spec_version, spec_at)]
		{
			let metadata_path = metadata_dir.join(format!("{spec_version}.scale"));
			if !metadata_path.exists() {
				let metadata = rpc.metadata(at).await?;
				write(&metadata_path, &metadata)?;
				tracing::info!(spec_version, "recorded metadata");
			}
		}

		let block = FixtureBlock {
			hash,
			parent_hash: raw.parent_hash,
			number,
			state_root: raw.state_root,
			extrinsics_root: raw.extrinsics_root,
			extrinsics: raw.extrinsics.into_iter().map(Bytes::from).collect(),
			storage,
			child_storage: BTreeMap::new(),
			runtime_version: fixture_version(version),
			parent_runtime_version: Some(fixture_version(parent_version)),
		};
		let json = serde_json::to_string_pretty(&block)?;
		write(&blocks_dir.join(format!("{number}.json")), json.as_bytes())?;
		tracing::info!(number, %hash, "recorded block");
	}

	let finalized_path = out.join("finalized.json");
	let recorded_max = blocks.iter().copied().max().unwrap_or_default();
	let previous = match std::fs::read_to_string(&finalized_path) {
		Ok(raw) =>
			serde_json::from_str::<FixtureFinalized>(&raw)
				.with_context(|| format!("parse {}", finalized_path.display()))?
				.number,
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => 0,
		Err(err) => return Err(err).with_context(|| format!("read {}", finalized_path.display())),
	};
	let finalized = FixtureFinalized { number: previous.max(recorded_max) };
	write(&finalized_path, serde_json::to_string_pretty(&finalized)?.as_bytes())?;
	Ok(())
}

fn fixture_version(version: RuntimeVersion) -> FixtureRuntimeVersion {
	FixtureRuntimeVersion {
		spec_name: version.spec_name,
		spec_version: version.spec_version,
		transaction_version: version.transaction_version,
	}
}

fn write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
	std::fs::write(path, bytes).with_context(|| format!("write {}", path.display()))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn parse(specs: &[&str]) -> anyhow::Result<Vec<u64>> {
		parse_blocks(&specs.iter().map(|spec| spec.to_string()).collect::<Vec<_>>())
	}

	#[test]
	fn parses_numbers_and_ranges() {
		assert_eq!(parse(&["100", "105-108"]).unwrap(), vec![100, 105, 106, 107, 108]);
		assert_eq!(parse(&["7,3-5, 4"]).unwrap(), vec![3, 4, 5, 7]);
		assert_eq!(parse(&["9-9"]).unwrap(), vec![9]);
	}

	#[test]
	fn rejects_bad_input() {
		assert!(parse(&[]).is_err());
		assert!(parse(&[""]).is_err());
		assert!(parse(&["10-5"]).is_err());
		assert!(parse(&["abc"]).is_err());
		assert!(parse(&["1-2-3"]).is_err());
	}
}
