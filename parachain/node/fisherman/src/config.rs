// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Validation of the consolidated relayer's [`HyperbridgeConfig`] for use by
//! the collator-side fisherman. The toml is parsed by
//! [`tesseract::config::HyperbridgeConfig::parse_conf`] — there's no
//! parallel schema here. This module only encodes the rules that the
//! collator path layers on top: a signer must be set, every supported L2 in
//! [`tesseract_evm::registry`] must be present, and every L2 needs at least
//! two `rpc_urls` so the byzantine handler has providers to quorum across.
//!
//! Rules are deliberately stricter than the relayer's: running fisherman is
//! not optional for collators.

use std::collections::HashSet;

use anyhow::anyhow;
use ismp::host::StateMachine;
use tesseract::config::HyperbridgeConfig;
use tesseract_config::AnyConfig;
use tesseract_evm::registry::{
	is_supported_l2, SUPPORTED_L2_CHAIN_IDS_MAINNET, SUPPORTED_L2_CHAIN_IDS_TESTNET,
};
use url::Url;

const MIN_RPC_URLS_PER_L2: usize = 2;

/// Enforce the collator-side rules. Returns the first violation as `Err`.
/// The signer is sourced from the local AURA keystore by the wrapper
pub fn validate(config: &HyperbridgeConfig) -> anyhow::Result<()> {
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

		configured_l2s.push(*chain_id as u64);
	}

	require_complete_set(&configured_l2s, SUPPORTED_L2_CHAIN_IDS_MAINNET, "mainnet")?;
	require_complete_set(&configured_l2s, SUPPORTED_L2_CHAIN_IDS_TESTNET, "testnet")?;
	Ok(())
}

/// Reject if any two URLs in `urls` resolve to the same host. Same host =
/// same provider as far as the byzantine quorum is concerned, so allowing
/// duplicates would silently shrink the effective fan-out.
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

/// Lower-cased host portion of an RPC URL. Two URLs with the same host are
/// treated as the same provider — different paths or API keys on the same
/// vendor (e.g. two Alchemy endpoints) don't add quorum value, so the byzantine
/// handler shouldn't be tricked into thinking they do.
fn rpc_host(url: &str) -> Option<String> {
	Url::parse(url).ok()?.host_str().map(str::to_ascii_lowercase)
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
			"{label} L2 coverage is partial — missing chain_ids {missing:?}. Running fisherman requires all supported L2s to be configured"
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

		use ismp::host::StateMachine;
		use tesseract::config::{
			HyperbridgeConfig, HyperbridgeSection, PerChainConfig, RelayerConfig,
		};
		use tesseract_config::AnyConfig;
		use tesseract_evm::EvmConfig;
		use tesseract_substrate::SubstrateConfig;

		use crate::config::validate;

		fn evm_l2(chain_id: u32, rpc_urls: &[&str]) -> PerChainConfig {
			PerChainConfig {
				messaging: AnyConfig::Evm(EvmConfig {
					rpc_urls: rpc_urls.iter().map(|s| (*s).to_string()).collect(),
					state_machine: Some(StateMachine::Evm(chain_id)),
					..EvmConfig::default()
				}),
				consensus: None,
			}
		}

		fn substrate_hyperbridge() -> SubstrateConfig {
			SubstrateConfig {
				state_machine: Some(StateMachine::Polkadot(4009)),
				hashing: None,
				consensus_state_id: Some("DOT0".into()),
				rpc_ws: "ws://127.0.0.1:9933".into(),
				max_rpc_payload_size: None,
				signer: None,
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
	}
}
