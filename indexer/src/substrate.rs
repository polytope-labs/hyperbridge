//! Reads from Substrate chains: storage through substrate-eth-rpc, other RPCs directly.

use std::{sync::Arc, time::Duration};

use alloy::{
	eips::BlockId,
	primitives::{keccak256, Address, Bytes},
	providers::Provider,
	rpc::types::TransactionRequest,
	sol,
	sol_types::{SolCall, SolValue},
};
use anyhow::{bail, Context};
use blake2::{Blake2b512, Digest};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::{chains::Chain, errors::Retry, failover::Failover};

sol! {
	interface SubstrateStorage {
		function getStorage(bytes key) external view returns (bytes value);
		function getChildStorage(bytes childKey, bytes key) external view returns (bytes value);
	}
}

/** The address substrate-eth-rpc answers storage `eth_call`s at. */
pub fn storage_address() -> Address {
	Address::from_slice(&keccak256(b"substrate-storage")[12..])
}

impl Chain {
	/** Raw SCALE bytes at `key` as of `block`. An absent key reads as empty. */
	pub async fn storage(&self, key: &[u8], block: u64) -> anyhow::Result<Vec<u8>> {
		let call = SubstrateStorage::getStorageCall { key: Bytes::copy_from_slice(key) };
		self.storage_call(call.abi_encode(), block).await
	}

	/** Raw SCALE bytes at `key` in the child trie `child_key`, as of `block`. */
	pub async fn child_storage(
		&self,
		child_key: &[u8],
		key: &[u8],
		block: u64,
	) -> anyhow::Result<Vec<u8>> {
		let call = SubstrateStorage::getChildStorageCall {
			childKey: Bytes::copy_from_slice(child_key),
			key: Bytes::copy_from_slice(key),
		};
		self.storage_call(call.abi_encode(), block).await
	}

	async fn storage_call(&self, input: Vec<u8>, block: u64) -> anyhow::Result<Vec<u8>> {
		let request = TransactionRequest::default().to(storage_address()).input(input.into());
		let output = self
			.provider
			.call(request)
			.block(BlockId::number(block))
			.await
			.with_context(|| format!("{}: storage read at block {block}", self.name))?;
		Ok(Bytes::abi_decode(&output)?.to_vec())
	}
}

/**
 * JSON-RPC over HTTP to a Substrate node, for methods substrate-eth-rpc doesn't carry
 * (`ismp_queryRequests`, `chain_getBlock`, ...). A request fails over between the node's URLs,
 * and is retried three times when none answers.
 */
#[derive(Clone)]
pub struct SubstrateRpc {
	urls: Arc<Vec<String>>,
	failover: Arc<Failover>,
	http: reqwest::Client,
}

impl SubstrateRpc {
	/** A client for these URLs, of which there is at least one. */
	pub fn new(urls: &[impl AsRef<str>], http: reqwest::Client) -> Self {
		let urls = urls
			.iter()
			.map(|url| {
				let url = url.as_ref();
				if let Some(rest) = url.strip_prefix("wss://") {
					format!("https://{rest}")
				} else if let Some(rest) = url.strip_prefix("ws://") {
					format!("http://{rest}")
				} else {
					url.to_string()
				}
			})
			.collect();
		Self { urls: Arc::new(urls), failover: Default::default(), http }
	}

	pub async fn call<T: DeserializeOwned>(
		&self,
		method: &str,
		params: Value,
	) -> anyhow::Result<T> {
		let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
		let mut delay = Duration::from_millis(100);
		let mut attempt = 0;
		let response: Value = 'answered: loop {
			let mut last = None;
			for index in self.failover.order(self.urls.len()) {
				let sent = self
					.http
					.post(&self.urls[index])
					.timeout(Duration::from_secs(30))
					.json(&body)
					.send()
					.await
					.and_then(|r| r.error_for_status());
				match sent {
					Ok(response) => {
						self.failover.answered(index);
						break 'answered response.json().await?;
					},
					Err(err) => last = Some(err),
				}
			}
			let err = last.expect("a Substrate RPC has a URL");
			if attempt == 3 {
				return Err(anyhow::Error::new(err).context(method.to_string()));
			}
			tracing::warn!(method, "substrate rpc failed, retrying: {err}");
			tokio::time::sleep(delay).await;
			delay *= 2;
			attempt += 1;
		};
		if let Some(error) = response.get("error") {
			if is_rate_limit(error) {
				return Err(Retry(format!("{method}: {error}")).into());
			}
			bail!("{method}: {error}");
		}
		let result = response.get("result").cloned().unwrap_or(Value::Null);
		serde_json::from_value(result).with_context(|| format!("decoding {method} result"))
	}
}

/** A JSON-RPC error saying the node is busy, which goes away on a retry. */
fn is_rate_limit(error: &Value) -> bool {
	let code = error.get("code").and_then(Value::as_i64);
	let message = error.get("message").and_then(Value::as_str).unwrap_or_default().to_lowercase();
	matches!(code, Some(-32999 | -32005 | 429)) ||
		message.contains("rate limit") ||
		message.contains("too many requests")
}

/** SS58 address of a 32-byte account, for network prefixes below 64 (0 Polkadot, 42 generic). */
pub fn ss58(account: &[u8; 32], prefix: u8) -> String {
	encode_address(account, prefix).expect("32-byte keys and prefixes below 64 encode")
}

/**
 * `encodeAddress` from `@polkadot/util-crypto`: SS58 for keys of 1, 2, 4, 8, 32 or 33 bytes and
 * prefixes below 64. polkadot-js threw for any other length; this returns `None`.
 */
pub fn encode_address(key: &[u8], prefix: u8) -> Option<String> {
	if ![1, 2, 4, 8, 32, 33].contains(&key.len()) || prefix >= 64 {
		return None;
	}
	let mut payload = Vec::with_capacity(key.len() + 3);
	payload.push(prefix);
	payload.extend_from_slice(key);
	let mut hasher = Blake2b512::new();
	hasher.update(b"SS58PRE");
	hasher.update(&payload);
	let checksum = hasher.finalize();
	let checksum_len = if key.len() >= 32 { 2 } else { 1 };
	payload.extend_from_slice(&checksum[..checksum_len]);
	Some(bs58::encode(payload).into_string())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn busy_answers_are_rate_limits() {
		assert!(is_rate_limit(&json!({ "code": -32999, "message": "RPC rate limit exceeded" })));
		assert!(is_rate_limit(&json!({ "code": -32000, "message": "Too Many Requests" })));
		assert!(!is_rate_limit(&json!({ "code": -32601, "message": "Method not found" })));
	}

	#[tokio::test]
	async fn a_call_moves_past_a_node_that_is_down() {
		let (live, _server) = crate::failover::tests::rpc_answering("Hyperbridge (Nexus)").await;
		let rpc = SubstrateRpc::new(&["http://127.0.0.1:9", live.as_str()], reqwest::Client::new());
		let name: String = rpc.call("system_chain", json!([])).await.unwrap();
		assert_eq!(name, "Hyperbridge (Nexus)");
		assert_eq!(rpc.failover.order(2).collect::<Vec<_>>(), [1, 0]);
	}

	#[tokio::test]
	async fn an_unreachable_node_is_transient() {
		let rpc = SubstrateRpc::new(&["http://127.0.0.1:9"], reqwest::Client::new());
		let err = rpc.call::<Value>("chain_getHeader", json!([])).await.unwrap_err();
		assert!(crate::errors::is_transient(&err), "{err:#}");
	}

	#[test]
	fn storage_address_matches_the_facade() {
		assert_eq!(
			storage_address(),
			"0x401e375f643e635ef3b498cedbc47931d2474364".parse::<Address>().unwrap()
		);
	}

	#[test]
	fn ss58_matches_known_addresses() {
		// Alice's sr25519 public key.
		let alice: [u8; 32] =
			hex::decode("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d")
				.unwrap()
				.try_into()
				.unwrap();
		assert_eq!(ss58(&alice, 42), "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY");
		assert_eq!(ss58(&alice, 0), "15oF4uVJwmo4TdGW7VfQxNLavjCXviqxT9S1MgbjMNHr6Sp5");
	}
}
