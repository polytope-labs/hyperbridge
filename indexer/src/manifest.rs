use std::{
	collections::{BTreeMap, HashMap},
	path::Path,
	sync::Arc,
};

use serde_json::{json, Value};

use crate::{
	config::Config,
	registry::{stream_name, ChainRoutes, STREAM_EVENT},
};

/**
 * Builds the rindexer manifest: one stream per chain over every address its handlers watch, with
 * the chain's start block and reorg-safe distance. rindexer's table creation and reorg rollback
 * are off: handlers write the `app` tables themselves and only see blocks past the safe distance.
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
