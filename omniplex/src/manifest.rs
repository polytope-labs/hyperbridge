use std::{
	collections::{BTreeMap, HashMap},
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::anyhow;
use rindexer::manifest::{core::Manifest, yaml::read_manifest};
use serde_json::{json, Value};

use crate::{
	config::{ChainConfig, Config},
	registry::{stream_abi, stream_name, ChainRoutes, STREAM_EVENT},
};

/**
 * Writes the rindexer manifest and the stream ABI it names, and reads the manifest back as
 * rindexer will. They go in a folder of `dir` named after the chains in `rpc_urls`: rindexer
 * reads the manifest again when it starts, so processes that share `dir` must not overwrite
 * each other's. Only this user can read them: the manifest holds RPC URLs, which can carry
 * keys.
 */
pub fn write(
	dir: &Path,
	config: &Config,
	rpc_urls: &BTreeMap<String, String>,
	cursors: &BTreeMap<String, u64>,
	routes: &HashMap<String, Arc<ChainRoutes>>,
) -> anyhow::Result<(PathBuf, Manifest)> {
	let dir = dir.join(rpc_urls.keys().map(String::as_str).collect::<Vec<_>>().join("+"));
	let abi = write_private(&dir, "stream.abi.json", &stream_abi())?;
	let manifest = build(config, rpc_urls, cursors, routes, &abi);
	// Written as JSON, which is valid YAML. serde_yaml writes numbers as maps when the build turns
	// on serde_json's `arbitrary_precision`, as a build alongside the runtimes does.
	let path = write_private(&dir, "rindexer.yaml", &serde_json::to_string_pretty(&manifest)?)?;
	let manifest = read_manifest(&path).map_err(|e| anyhow!("rindexer manifest: {e}"))?;
	Ok((path, manifest))
}

fn write_private(dir: &Path, name: &str, contents: &str) -> anyhow::Result<PathBuf> {
	use std::{fs, os::unix::fs::PermissionsExt};
	fs::create_dir_all(dir)?;
	fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
	let path = dir.join(name);
	fs::write(&path, contents)?;
	fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
	Ok(path)
}

/**
 * Builds the rindexer manifest: one stream per chain over every address its handlers watch,
 * with the chain's start block and reorg-safe distance. rindexer's table creation and reorg
 * rollback are off: handlers write the `app` tables themselves and only see blocks past the
 * safe distance.
 */
pub fn build(
	config: &Config,
	rpc_urls: &BTreeMap<String, String>,
	cursors: &BTreeMap<String, u64>,
	routes: &HashMap<String, Arc<ChainRoutes>>,
	stream_abi: &Path,
) -> Value {
	let networks: Vec<Value> = config
		.chains
		.iter()
		.filter(|(name, _)| rpc_urls.contains_key(*name))
		.map(|(name, chain)| {
			let mut network = json!({
				"name": name,
				"chain_id": chain.chain_id,
				"rpc": rpc_urls[name],
				"disable_logs_bloom_checks": true,
				"reorg_handling": { "enabled": false },
			});
			if let Some(range) = chain.max_block_range {
				network["max_block_range"] = json!(range);
			}
			network
		})
		.collect();

	let contracts: Vec<Value> = config
		.chains
		.iter()
		.filter_map(|(network, chain)| {
			let addresses = routes.get(network)?.addresses();
			if addresses.is_empty() {
				return None;
			}
			Some(json!({
				"name": stream_name(network),
				"details": [{
					"network": network,
					"address": addresses.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
					"start_block": start_block(chain, cursors.get(network)).to_string(),
				}],
				"abi": stream_abi.to_string_lossy(),
				"include_events": [STREAM_EVENT],
				"index_event_in_order": [STREAM_EVENT],
				"reorg_safe_distance": chain.reorg_safe_distance,
			}))
		})
		.collect();

	json!({
		"name": "Omniplex",
		"project_type": "rust",
		"timestamps": true,
		"networks": networks,
		"storage": { "postgres": { "enabled": true, "disable_create_tables": true } },
		"contracts": contracts,
	})
}

/**
 * Where rindexer starts a chain it has no progress for: the config's start block, or the
 * chain's cursor block when that is later, as after a SubQuery import. The cursor skips the
 * logs of its own block, and rindexer refuses a start past the node's head, which SubQuery's
 * last block can be. With progress of its own, rindexer resumes from that instead.
 */
fn start_block(chain: &ChainConfig, cursor: Option<&u64>) -> u64 {
	cursor.map_or(chain.start_block, |block| chain.start_block.max(*block))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::handlers;

	/** No database needed, so this runs in every build, including those that turn on
	 * serde_json's `arbitrary_precision`. */
	#[test]
	fn every_chain_reads_back_as_rindexer_parses_it() {
		let handlers = handlers::all();
		for env in ["mainnet", "testnet"] {
			let root = Path::new(env!("CARGO_MANIFEST_DIR"));
			let config = Config::load(&root.join(format!("config/{env}.toml"))).unwrap();
			let routes = config
				.chains
				.iter()
				.map(|(name, chain)| {
					let routes = ChainRoutes::new(chain, &handlers.subscriptions, &handlers.hooks);
					(name.clone(), Arc::new(routes))
				})
				.collect();
			let rpc_urls = config
				.chains
				.keys()
				.map(|name| (name.clone(), "http://127.0.0.1:9".into()))
				.collect();
			let dir =
				std::env::temp_dir().join(format!("indexer-manifest-{env}-{}", std::process::id()));
			let written = write(&dir, &config, &rpc_urls, &BTreeMap::new(), &routes);
			let _ = std::fs::remove_dir_all(&dir);
			let (_, manifest) = written.unwrap();
			assert_eq!(manifest.networks.len(), config.chains.len(), "{env}");
			for network in &manifest.networks {
				let chain = &config.chains[&network.name];
				assert_eq!(network.chain_id, chain.chain_id, "{env} {}", network.name);
			}
		}
	}

	/** Processes for different chains can share a run dir: each gets its own manifest. */
	#[test]
	fn each_set_of_chains_gets_its_own_manifest() {
		let handlers = handlers::all();
		let root = Path::new(env!("CARGO_MANIFEST_DIR"));
		let config = Config::load(&root.join("config/mainnet.toml")).unwrap();
		let dir =
			std::env::temp_dir().join(format!("indexer-manifest-sets-{}", std::process::id()));
		let write_for = |names: &[&str]| {
			let routes = names
				.iter()
				.map(|name| {
					let chain = &config.chains[*name];
					let routes = ChainRoutes::new(chain, &handlers.subscriptions, &handlers.hooks);
					(name.to_string(), Arc::new(routes))
				})
				.collect();
			let rpc_urls = names
				.iter()
				.map(|name| (name.to_string(), "http://127.0.0.1:9".into()))
				.collect();
			write(&dir, &config, &rpc_urls, &BTreeMap::new(), &routes)
		};
		let bsc = write_for(&["bsc-mainnet"]);
		let others = write_for(&["base-mainnet", "ethereum-mainnet"]);
		let _ = std::fs::remove_dir_all(&dir);
		let ((bsc_path, bsc), (others_path, others)) = (bsc.unwrap(), others.unwrap());
		assert_ne!(bsc_path, others_path);
		let names = |manifest: &Manifest| {
			let mut names: Vec<_> = manifest.networks.iter().map(|n| n.name.clone()).collect();
			names.sort();
			names
		};
		assert_eq!(names(&bsc), ["bsc-mainnet"]);
		assert_eq!(names(&others), ["base-mainnet", "ethereum-mainnet"]);
	}

	/** A chain whose cursor is past its start block, as after a SubQuery import, starts at the
	 * cursor. */
	#[test]
	fn a_chain_starts_at_its_cursor() {
		let root = Path::new(env!("CARGO_MANIFEST_DIR"));
		let config = Config::load(&root.join("config/mainnet.toml")).unwrap();
		let chain = &config.chains["bsc-mainnet"];
		assert_eq!(start_block(chain, None), chain.start_block);
		assert_eq!(start_block(chain, Some(&(chain.start_block + 41))), chain.start_block + 41);
		assert_eq!(start_block(chain, Some(&(chain.start_block - 1))), chain.start_block);
	}
}
