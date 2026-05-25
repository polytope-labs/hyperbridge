// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Collator side validation of the operator's tesseract toml.
//!
//! Two entry points. [`preflight`] inspects the raw toml and is safe to run
//! before chain init, so the binary can fail fast on a bad config without
//! touching the network. [`validate`] runs the full check on a parsed
//! [`HyperbridgeConfig`] and is invoked from [`crate::spawn`] once the
//! local RPC server is up.
//!
//! Rules layered on top of the relayer schema: the signer must be set,
//! every supported L2 in [`tesseract_evm::registry`] must be configured,
//! every L2 needs at least two distinct host `rpc_urls` so the byzantine
//! handler has independent providers to quorum across, and every L2 must
//! carry a `[<chain>.consensus]` sub-table with the matching consensus
//! kind so the L1 rollup-claim fisherman has the rollup-core / dispute-game
//! factory address it needs to watch.

use std::collections::HashSet;

use anyhow::{anyhow, Context};
use ismp::host::StateMachine;
use tesseract::config::HyperbridgeConfig;
use tesseract_config::AnyConfig;
use tesseract_consensus_config::AnyConfig as ConsensusConfig;
use tesseract_evm::registry::{
	expected_consensus_kind, is_supported_l2, SUPPORTED_L2_CHAIN_IDS_MAINNET,
	SUPPORTED_L2_CHAIN_IDS_TESTNET,
};
use toml::{Table, Value};
use url::Url;

const MIN_RPC_URLS_PER_L2: usize = 2;

/// Validate the operator's tesseract toml without parsing it through
/// [`HyperbridgeConfig::parse_conf`]. The full parser dials each RPC to
/// resolve chain metadata, and on a collator `[hyperbridge].rpc_ws`
/// usually points at the same node, which has not opened its RPC port yet
/// at this point in startup. Reading the raw toml lets us still catch the
/// common operator mistakes (missing file, malformed toml, blank signer,
/// partial L2 coverage) without any network I/O.
pub fn preflight(toml_str: &str) -> anyhow::Result<()> {
	let table: Table = toml_str.parse().context("tesseract config is not valid TOML")?;

	let hyperbridge = table
		.get("hyperbridge")
		.and_then(Value::as_table)
		.ok_or_else(|| anyhow!("missing [hyperbridge] table in tesseract config"))?;
	let signer = hyperbridge.get("signer").and_then(Value::as_str).ok_or_else(|| {
		anyhow!(
			"[hyperbridge].signer is required for the fisherman; set it to the seed/URI of the account that should sign veto extrinsics"
		)
	})?;
	if signer.trim().is_empty() {
		return Err(anyhow!(
			"[hyperbridge].signer is empty; set it to the seed/URI of the account that should sign veto extrinsics"
		));
	}

	let mut configured_l2s: Vec<u64> = Vec::new();
	for (name, raw) in &table {
		if name == "hyperbridge" || name == "relayer" {
			continue;
		}
		let Some(chain_table) = raw.as_table() else { continue };
		// Substrate chains have rpc_ws, EVM chains have rpc_urls. The
		// fisherman only cares about EVM L2s, so skip anything else.
		let Some(rpc_urls) = chain_table.get("rpc_urls").and_then(Value::as_array) else {
			continue;
		};

		// Without an explicit state_machine in the toml the chain id is
		// only known after resolve, so defer to the post resolve check.
		let Some(chain_id) = chain_table
			.get("state_machine")
			.and_then(Value::as_str)
			.and_then(parse_evm_chain_id)
		else {
			continue;
		};

		if !is_supported_l2(chain_id) {
			continue;
		}

		let urls: Vec<String> = rpc_urls
			.iter()
			.map(|v| {
				v.as_str()
					.map(str::to_string)
					.ok_or_else(|| anyhow!("[{name}].rpc_urls entries must be strings"))
			})
			.collect::<anyhow::Result<_>>()?;
		if urls.len() < MIN_RPC_URLS_PER_L2 {
			return Err(anyhow!(
				"L2 chain (chain_id {chain_id}) has only {} rpc_urls, need at least {} different RPC providers for quorum",
				urls.len(),
				MIN_RPC_URLS_PER_L2,
			));
		}
		ensure_distinct_hosts(chain_id, &urls)?;
		ensure_consensus_section_raw(name, chain_table, chain_id)?;
		configured_l2s.push(chain_id);
	}

	require_complete_set(&configured_l2s, SUPPORTED_L2_CHAIN_IDS_MAINNET, "mainnet")?;
	require_complete_set(&configured_l2s, SUPPORTED_L2_CHAIN_IDS_TESTNET, "testnet")?;
	Ok(())
}

/// Pull the chain id out of an `"EVM-<chain_id>"` state machine string.
/// Returns `None` for any other variant (Polkadot, Kusama, Substrate, and
/// so on) since the fisherman only cares about EVM L2s.
fn parse_evm_chain_id(state_machine: &str) -> Option<u64> {
	state_machine.strip_prefix("EVM-")?.parse().ok()
}

/// Enforce the collator side rules on a parsed [`HyperbridgeConfig`].
/// Returns the first violation as `Err`. The signer must be set in
/// `[hyperbridge].signer`; operators provide it explicitly and it is not
/// sourced from the local keystore.
pub fn validate(config: &HyperbridgeConfig) -> anyhow::Result<()> {
	if config.hyperbridge.substrate.signer.as_deref().map_or(true, str::is_empty) {
		return Err(anyhow!(
			"[hyperbridge].signer is required for the fisherman; set it to the seed/URI of the account that should sign veto extrinsics"
		));
	}

	let mut configured_l2s: Vec<u64> = Vec::new();
	for (state_machine, per_chain) in &config.chains {
		let AnyConfig::Evm(evm) = &per_chain.messaging else { continue };
		let StateMachine::Evm(chain_id) = state_machine else { continue };
		if !is_supported_l2(*chain_id as u64) {
			continue;
		}
		if evm.rpc_urls.len() < MIN_RPC_URLS_PER_L2 {
			return Err(anyhow!(
				"L2 chain (chain_id {chain_id}) has only {} rpc_urls, need at least {} different RPC providers for quorum",
				evm.rpc_urls.len(),
				MIN_RPC_URLS_PER_L2,
			));
		}
		ensure_distinct_hosts(*chain_id as u64, &evm.rpc_urls)?;
		ensure_consensus_section(*chain_id as u64, per_chain.consensus.as_ref())?;

		configured_l2s.push(*chain_id as u64);
	}

	require_complete_set(&configured_l2s, SUPPORTED_L2_CHAIN_IDS_MAINNET, "mainnet")?;
	require_complete_set(&configured_l2s, SUPPORTED_L2_CHAIN_IDS_TESTNET, "testnet")?;
	Ok(())
}

/// Reject if any two URLs in `urls` resolve to the same host. Same host
/// means the same provider as far as the byzantine quorum is concerned,
/// so allowing duplicates would silently shrink the effective fan out.
fn ensure_distinct_hosts(chain_id: u64, urls: &[String]) -> anyhow::Result<()> {
	let mut seen: HashSet<String> = HashSet::with_capacity(urls.len());
	for url in urls {
		let host = rpc_host(url).ok_or_else(|| {
			anyhow!("L2 chain (chain_id {chain_id}) has rpc_url {url:?} with no parseable host")
		})?;
		if !seen.insert(host.clone()) {
			return Err(anyhow!(
				"L2 chain (chain_id {chain_id}) lists multiple rpc_urls on host {host:?}; quorum requires distinct providers"
			));
		}
	}
	Ok(())
}

/// Lower cased host portion of an RPC URL. Two URLs with the same host
/// are treated as the same provider; different paths or API keys on the
/// same vendor (two Alchemy endpoints, say) do not add quorum value, so
/// the byzantine handler should not be tricked into thinking they do.
fn rpc_host(url: &str) -> Option<String> {
	Url::parse(url).ok()?.host_str().map(str::to_ascii_lowercase)
}

/// Enforce that the operator has wired the matching consensus client for `chain_id`. The
/// fisherman reads the existing `op-host` / `arb-host` configs (`OpConfig.host` /
/// `ArbConfig.host`) for the rollup-core / dispute-game-factory addresses; without it the L1
/// rollup-claim watcher has nothing to monitor for this chain.
fn ensure_consensus_section(
	chain_id: u64,
	consensus: Option<&ConsensusConfig>,
) -> anyhow::Result<()> {
	let Some(expected) = expected_consensus_kind(chain_id) else {
		// Unknown L2 (not in the supported registry). The caller already filtered to known
		// L2s; this branch is just defensive.
		return Ok(());
	};
	let Some(consensus) = consensus else {
		return Err(anyhow!(
			"L2 chain (chain_id {chain_id}) has no [<chain>.consensus] section; \
			fisherman requires a {expected:?} consensus block for each L2 so it can read the \
			rollup-core / dispute-game factory address"
		));
	};
	let actual = consensus_kind_str(consensus);
	if actual != expected {
		return Err(anyhow!(
			"L2 chain (chain_id {chain_id}) has [<chain>.consensus] type {actual:?}, \
			expected {expected:?}"
		));
	}
	Ok(())
}

/// String tag for a [`ConsensusConfig`] variant, matching the serde `type = "..."` discriminant.
fn consensus_kind_str(consensus: &ConsensusConfig) -> &'static str {
	match consensus {
		ConsensusConfig::Sepolia { .. } => "sepolia",
		ConsensusConfig::Ethereum { .. } => "ethereum",
		ConsensusConfig::ArbitrumOrbit { .. } => "arbitrum_orbit",
		ConsensusConfig::OpStack { .. } => "op_stack",
		ConsensusConfig::BscTestnet { .. } => "bsc_testnet",
		ConsensusConfig::Bsc { .. } => "bsc",
		ConsensusConfig::Chiado { .. } => "chiado",
		ConsensusConfig::Gnosis { .. } => "gnosis",
		ConsensusConfig::Grandpa { .. } => "grandpa",
		ConsensusConfig::Parachain { .. } => "parachain",
		ConsensusConfig::Polygon { .. } => "polygon",
		ConsensusConfig::Tendermint { .. } => "tendermint",
		ConsensusConfig::EvmHost { .. } => "evm_host",
		ConsensusConfig::Pharos { .. } => "pharos",
	}
}

/// Preflight version of [`ensure_consensus_section`] that operates on the raw TOML table for
/// a chain. Reads `[<chain>.consensus]` and checks its `type` matches the expected kind.
fn ensure_consensus_section_raw(
	chain_name: &str,
	chain_table: &Table,
	chain_id: u64,
) -> anyhow::Result<()> {
	let Some(expected) = expected_consensus_kind(chain_id) else {
		return Ok(());
	};
	let consensus = chain_table.get("consensus").and_then(Value::as_table).ok_or_else(|| {
		anyhow!(
			"L2 chain {chain_name:?} (chain_id {chain_id}) has no [{chain_name}.consensus] \
			sub-table; fisherman requires a {expected:?} consensus block for each L2"
		)
	})?;
	let actual = consensus.get("type").and_then(Value::as_str).ok_or_else(|| {
		anyhow!(
			"[{chain_name}.consensus] is missing the `type` field; expected type = {expected:?}"
		)
	})?;
	if actual != expected {
		return Err(anyhow!(
			"[{chain_name}.consensus].type is {actual:?}, expected {expected:?}"
		));
	}
	Ok(())
}

/// If any chain in `set` is configured, all of them must be configured. A
/// completely empty intersection is fine (operator runs the other set).
fn require_complete_set(configured: &[u64], set: &[u64], label: &str) -> anyhow::Result<()> {
	let any_present = set.iter().any(|c| configured.contains(c));
	if !any_present {
		return Ok(());
	}
	let missing: Vec<u64> = set.iter().copied().filter(|c| !configured.contains(c)).collect();
	if !missing.is_empty() {
		return Err(anyhow!(
			"{label} L2 coverage is partial, missing chain_ids {missing:?}. Running fisherman requires all supported L2s to be configured"
		));
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::{ensure_distinct_hosts, rpc_host};

	#[test]
	fn rpc_host_strips_scheme_path_query_and_fragment() {
		assert_eq!(rpc_host("https://eth.example/v2/key"), Some("eth.example".into()));
		assert_eq!(rpc_host("wss://eth.example?token=k"), Some("eth.example".into()));
		assert_eq!(rpc_host("https://eth.example#frag"), Some("eth.example".into()));
		assert_eq!(rpc_host("https://eth.example/"), Some("eth.example".into()));
	}

	#[test]
	fn rpc_host_strips_userinfo_and_port() {
		assert_eq!(rpc_host("https://user:pass@eth.example:8545/"), Some("eth.example".into()));
		assert_eq!(rpc_host("https://eth.example:443"), Some("eth.example".into()));
	}

	#[test]
	fn rpc_host_lowercases() {
		assert_eq!(rpc_host("https://Eth.Example.COM/v2"), Some("eth.example.com".into()));
	}

	#[test]
	fn rpc_host_handles_ipv6_literal() {
		// `url::Url::host_str` returns IPv6 literals with their brackets; that's
		// fine for our use-case since we only compare hosts for equality.
		assert_eq!(rpc_host("https://[::1]:8545/rpc"), Some("[::1]".into()));
		assert_eq!(rpc_host("https://[2001:db8::1]/"), Some("[2001:db8::1]".into()));
	}

	#[test]
	fn rpc_host_rejects_url_without_scheme() {
		// A scheme-less endpoint would fail downstream when the byzantine
		// handler tries to construct a provider, so reject it here.
		assert_eq!(rpc_host("eth.example/v2"), None);
	}

	#[test]
	fn rpc_host_rejects_malformed_urls() {
		assert_eq!(rpc_host(""), None);
		assert_eq!(rpc_host("not a url"), None);
		assert_eq!(rpc_host("ftp://"), None);
	}

	#[test]
	fn ensure_distinct_hosts_accepts_distinct_providers() {
		let urls = vec![
			"https://eth-mainnet.g.alchemy.com/v2/key1".into(),
			"https://mainnet.infura.io/v3/key2".into(),
			"https://rpc.ankr.com/eth".into(),
		];
		ensure_distinct_hosts(42161, &urls).unwrap();
	}

	#[test]
	fn ensure_distinct_hosts_rejects_same_host_different_paths() {
		let urls = vec![
			"https://eth-mainnet.g.alchemy.com/v2/key1".into(),
			"https://eth-mainnet.g.alchemy.com/v2/key2".into(),
		];
		let err = ensure_distinct_hosts(42161, &urls).unwrap_err().to_string();
		assert!(err.contains("eth-mainnet.g.alchemy.com"), "error: {err}");
		assert!(err.contains("distinct providers"), "error: {err}");
	}

	#[test]
	fn ensure_distinct_hosts_treats_case_as_same_host() {
		let urls = vec!["https://Rpc.Example.com/a".into(), "https://rpc.example.com/b".into()];
		assert!(ensure_distinct_hosts(42161, &urls).is_err());
	}

	#[test]
	fn ensure_distinct_hosts_ignores_port_when_host_matches() {
		let urls =
			vec!["https://rpc.example.com:443/".into(), "https://rpc.example.com:8545/".into()];
		assert!(ensure_distinct_hosts(42161, &urls).is_err());
	}

	#[test]
	fn ensure_distinct_hosts_rejects_unparseable_url() {
		let urls = vec!["https://good.example/".into(), "not a url".into()];
		let err = ensure_distinct_hosts(42161, &urls).unwrap_err().to_string();
		assert!(err.contains("no parseable host"), "error: {err}");
	}

	#[test]
	fn ensure_distinct_hosts_accepts_empty() {
		ensure_distinct_hosts(42161, &[]).unwrap();
	}

	mod parses_tesseract_config {
		use std::collections::HashMap;

		use arb_host::ArbConfig;
		use ismp::host::StateMachine;
		use op_host::OpConfig;
		use primitive_types::H160;
		use tesseract::config::{
			HyperbridgeConfig, HyperbridgeSection, PerChainConfig, RelayerConfig,
		};
		use tesseract_config::AnyConfig;
		use tesseract_consensus_config::AnyConfig as ConsensusConfig;
		use tesseract_evm::EvmConfig;
		use tesseract_substrate::SubstrateConfig;

		use crate::config::validate;

		fn arbitrum_consensus() -> ConsensusConfig {
			ConsensusConfig::ArbitrumOrbit {
				inner: ArbConfig {
					host: arb_host::HostConfig {
						ethereum_rpc_url: vec!["https://eth-l1.example/v2/k".into()],
						rollup_core: H160::repeat_byte(0x11),
						l1_state_machine: StateMachine::Evm(11155111),
						l1_consensus_state_id: "ETH0".into(),
						consensus_state_id: "ARBC".into(),
						consensus_update_frequency: None,
					},
				},
			}
		}

		fn opstack_consensus() -> ConsensusConfig {
			ConsensusConfig::OpStack {
				inner: OpConfig {
					host: op_host::HostConfig {
						ethereum_rpc_url: vec!["https://eth-l1.example/v2/k".into()],
						l2_oracle: None,
						dispute_game_factory: Some(H160::repeat_byte(0x22)),
						message_parser: H160::repeat_byte(0x33),
						proposer_config: None,
						l1_state_machine: StateMachine::Evm(11155111),
						l1_consensus_state_id: "ETH0".into(),
						consensus_state_id: "OPTI".into(),
						consensus_update_frequency: None,
					},
				},
			}
		}

		fn evm_l2(chain_id: u32, rpc_urls: &[&str]) -> PerChainConfig {
			let consensus = if matches!(chain_id, 42161 | 421614) {
				Some(arbitrum_consensus())
			} else {
				Some(opstack_consensus())
			};
			PerChainConfig {
				messaging: AnyConfig::Evm(EvmConfig {
					rpc_urls: rpc_urls.iter().map(|s| (*s).to_string()).collect(),
					state_machine: Some(StateMachine::Evm(chain_id)),
					..EvmConfig::default()
				}),
				consensus,
			}
		}

		fn substrate_hyperbridge() -> SubstrateConfig {
			SubstrateConfig {
				state_machine: Some(StateMachine::Polkadot(4009)),
				hashing: None,
				consensus_state_id: Some("DOT0".into()),
				rpc_ws: "ws://127.0.0.1:9933".into(),
				max_rpc_payload_size: None,
				signer: Some("//Alice".into()),
				initial_height: None,
				max_concurent_queries: None,
				poll_interval: None,
				fee_token_decimals: None,
			}
		}

		/// Returns a fully-resolved [`HyperbridgeConfig`] covering the complete
		/// testnet L2 set (Arbitrum Sepolia, Optimism Sepolia, Base Sepolia)
		/// with two distinct-host rpc_urls each. The relayer section is left
		/// at its [`Default`] (collators don't need to populate it).
		fn complete_testnet_collator_config() -> HyperbridgeConfig {
			let mut chains = HashMap::new();
			for (chain_id, rpcs) in [
				(
					421614u32,
					["https://arb-sepolia.alchemy.com/v2/k", "https://arb-sepolia.infura.io/v3/k"],
				),
				(
					11155420,
					["https://opt-sepolia.alchemy.com/v2/k", "https://opt-sepolia.infura.io/v3/k"],
				),
				(
					84532,
					[
						"https://base-sepolia.alchemy.com/v2/k",
						"https://base-sepolia.infura.io/v3/k",
					],
				),
			] {
				chains.insert(StateMachine::Evm(chain_id), evm_l2(chain_id, &rpcs));
			}
			HyperbridgeConfig {
				hyperbridge: HyperbridgeSection {
					substrate: substrate_hyperbridge(),
					consensus: None,
				},
				chains,
				relayer: RelayerConfig::default(),
			}
		}

		#[test]
		fn validate_accepts_complete_testnet_collator_config() {
			let cfg = complete_testnet_collator_config();
			validate(&cfg).expect("validate should accept a complete testnet collator config");
		}

		#[test]
		fn validate_rejects_missing_signer() {
			let mut cfg = complete_testnet_collator_config();
			cfg.hyperbridge.substrate.signer = None;
			let err = validate(&cfg).unwrap_err().to_string();
			assert!(err.contains("signer"), "error: {err}");
		}

		#[test]
		fn validate_rejects_empty_signer() {
			let mut cfg = complete_testnet_collator_config();
			cfg.hyperbridge.substrate.signer = Some(String::new());
			let err = validate(&cfg).unwrap_err().to_string();
			assert!(err.contains("signer"), "error: {err}");
		}

		#[test]
		fn validate_rejects_partial_l2_coverage() {
			let mut cfg = complete_testnet_collator_config();
			cfg.chains.remove(&StateMachine::Evm(84532));
			let err = validate(&cfg).unwrap_err().to_string();
			assert!(err.contains("testnet"), "error: {err}");
			assert!(err.contains("84532"), "error: {err}");
		}

		#[test]
		fn validate_rejects_chain_with_fewer_than_two_rpc_urls() {
			let mut cfg = complete_testnet_collator_config();
			let AnyConfig::Evm(ref mut evm) = cfg
				.chains
				.get_mut(&StateMachine::Evm(421614))
				.expect("arb sepolia present")
				.messaging
			else {
				panic!("expected evm config");
			};
			evm.rpc_urls.truncate(1);
			let err = validate(&cfg).unwrap_err().to_string();
			assert!(err.contains("421614"), "error: {err}");
			assert!(err.contains("at least"), "error: {err}");
		}

		#[test]
		fn validate_rejects_duplicate_host_within_chain() {
			let mut cfg = complete_testnet_collator_config();
			let AnyConfig::Evm(ref mut evm) = cfg
				.chains
				.get_mut(&StateMachine::Evm(421614))
				.expect("arb sepolia present")
				.messaging
			else {
				panic!("expected evm config");
			};
			evm.rpc_urls = vec![
				"https://arb-sepolia.alchemy.com/v2/key1".into(),
				"https://arb-sepolia.alchemy.com/v2/key2".into(),
			];
			let err = validate(&cfg).unwrap_err().to_string();
			assert!(err.contains("arb-sepolia.alchemy.com"), "error: {err}");
			assert!(err.contains("distinct providers"), "error: {err}");
		}

		#[test]
		fn validate_rejects_l2_missing_consensus() {
			let mut cfg = complete_testnet_collator_config();
			cfg.chains.get_mut(&StateMachine::Evm(421614)).unwrap().consensus = None;
			let err = validate(&cfg).unwrap_err().to_string();
			assert!(err.contains("421614"), "error: {err}");
			assert!(err.contains("consensus"), "error: {err}");
			assert!(err.contains("arbitrum_orbit"), "error: {err}");
		}

		#[test]
		fn validate_rejects_l2_with_wrong_consensus_kind() {
			let mut cfg = complete_testnet_collator_config();
			// Arbitrum Sepolia mistakenly wired with an opstack consensus block.
			cfg.chains.get_mut(&StateMachine::Evm(421614)).unwrap().consensus =
				Some(opstack_consensus());
			let err = validate(&cfg).unwrap_err().to_string();
			assert!(err.contains("421614"), "error: {err}");
			assert!(err.contains("op_stack"), "error: {err}");
			assert!(err.contains("arbitrum_orbit"), "error: {err}");
		}

		#[test]
		fn validate_rejects_opstack_l2_missing_consensus() {
			let mut cfg = complete_testnet_collator_config();
			cfg.chains.get_mut(&StateMachine::Evm(84532)).unwrap().consensus = None;
			let err = validate(&cfg).unwrap_err().to_string();
			assert!(err.contains("84532"), "error: {err}");
			assert!(err.contains("op_stack"), "error: {err}");
		}
	}
}
