use std::{collections::BTreeMap, path::Path};

use serde_json::{json, Value};

use crate::{
	config::Config,
	registry::{contract_name, Source, Subscription},
};

/**
 * Builds the rindexer manifest. Each source gets one contract entry per network, so each network
 * keeps its own start block and reorg-safe distance. rindexer's table creation and reorg rollback
 * are off: handlers write the `app` tables themselves and only see blocks past the safe distance.
 */
pub fn build(
	config: &Config,
	rpc_urls: &BTreeMap<String, String>,
	subscriptions: &[Subscription],
	abi_root: &Path,
) -> anyhow::Result<Value> {
	let networks: Vec<Value> = config
		.chains
		.iter()
		.map(|(name, chain)| {
			let mut network = json!({
				"name": name,
				"chain_id": chain.chain_id,
				"rpc": rpc_urls[name],
				"reorg_handling": { "enabled": false },
			});
			if let Some(range) = chain.max_block_range {
				network["max_block_range"] = json!(range);
			}
			network
		})
		.collect();

	let mut events_by_source: BTreeMap<String, (Source, Vec<&'static str>)> = BTreeMap::new();
	for subscription in subscriptions {
		let entry = events_by_source
			.entry(subscription.source.name())
			.or_insert_with(|| (subscription.source, vec![]));
		entry.1.push(subscription.event_name());
	}

	let mut contracts = vec![];
	for (network, chain) in &config.chains {
		for (source, events) in events_by_source.values() {
			let addresses = source.addresses(chain);
			if addresses.is_empty() {
				continue;
			}
			let addresses: Vec<String> = addresses.iter().map(|a| a.to_string()).collect();
			contracts.push(json!({
				"name": contract_name(source, network),
				"details": [{
					"network": network,
					"address": addresses,
					"start_block": chain.start_block.to_string(),
				}],
				"abi": abi_root.join(source.abi()).to_string_lossy(),
				"include_events": events,
				"index_event_in_order": events,
				"reorg_safe_distance": chain.reorg_safe_distance,
			}));
		}
	}

	Ok(json!({
		"name": "HyperbridgeIndexer",
		"project_type": "rust",
		"timestamps": true,
		"networks": networks,
		"storage": { "postgres": { "enabled": true, "disable_create_tables": true } },
		"contracts": contracts,
	}))
}
