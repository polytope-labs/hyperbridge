use std::{
	collections::{BTreeMap, HashMap},
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::anyhow;
use rindexer::manifest::{core::Manifest, yaml::read_manifest};
use serde_json::{json, Value};

use crate::{
	config::Config,
	registry::{stream_abi, stream_name, ChainRoutes, STREAM_EVENT},
};

/**
 * Writes the rindexer manifest and the stream ABI it names to `dir`, and reads the manifest
 * back as rindexer will. Only this user can read them: the manifest holds RPC URLs, which can
 * carry keys.
 */
pub fn write(
	dir: &Path,
	config: &Config,
	rpc_urls: &BTreeMap<String, String>,
	routes: &HashMap<String, Arc<ChainRoutes>>,
) -> anyhow::Result<(PathBuf, Manifest)> {
	let abi = write_private(dir, "stream.abi.json", &stream_abi())?;
	let manifest = build(config, rpc_urls, routes, &abi);
	// Written as JSON, which is valid YAML. serde_yaml writes numbers as maps when the build turns
	// on serde_json's `arbitrary_precision`, as a build alongside the runtimes does.
	let path = write_private(dir, "rindexer.yaml", &serde_json::to_string_pretty(&manifest)?)?;
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
					"start_block": chain.start_block.to_string(),
				}],
				"abi": stream_abi.to_string_lossy(),
				"include_events": [STREAM_EVENT],
				"index_event_in_order": [STREAM_EVENT],
				"reorg_safe_distance": chain.reorg_safe_distance,
			}))
		})
		.collect();

	json!({
		"name": "HyperbridgeIndexer",
		"project_type": "rust",
		"timestamps": true,
		"networks": networks,
		"storage": { "postgres": { "enabled": true, "disable_create_tables": true } },
		"contracts": contracts,
	})
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
			let written = write(&dir, &config, &rpc_urls, &routes);
			let _ = std::fs::remove_dir_all(&dir);
			let (_, manifest) = written.unwrap();
			assert_eq!(manifest.networks.len(), config.chains.len(), "{env}");
			for network in &manifest.networks {
				let chain = &config.chains[&network.name];
				assert_eq!(network.chain_id, chain.chain_id, "{env} {}", network.name);
			}
		}
	}
}
