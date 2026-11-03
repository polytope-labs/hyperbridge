//! Reads from Substrate chains: storage through substrate-eth-rpc, other RPCs directly.

use std::time::Duration;

use alloy::{
	eips::BlockId,
	primitives::{keccak256, Address, Bytes},
	providers::Provider,
	rpc::types::TransactionRequest,
	sol,
	sol_types::{SolCall, SolValue},
};
use anyhow::{anyhow, bail, Context};
use blake2::{Blake2b512, Digest};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::chains::Chain;

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
 * (`ismp_queryRequests`, `chain_getBlock`, ...). Retries transport failures three times.
 */
#[derive(Clone)]
pub struct SubstrateRpc {
	url: String,
	http: reqwest::Client,
}

impl SubstrateRpc {
	pub fn new(url: &str, http: reqwest::Client) -> Self {
		let url = if let Some(rest) = url.strip_prefix("wss://") {
			format!("https://{rest}")
		} else if let Some(rest) = url.strip_prefix("ws://") {
			format!("http://{rest}")
		} else {
			url.to_string()
		};
		Self { url, http }
	}

	pub async fn call<T: DeserializeOwned>(
		&self,
		method: &str,
		params: Value,
	) -> anyhow::Result<T> {
		let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
		let mut delay = Duration::from_millis(100);
		let mut attempt = 0;
		let response: Value = loop {
			let sent = self
				.http
				.post(&self.url)
				.timeout(Duration::from_secs(30))
				.json(&body)
				.send()
				.await
				.and_then(|r| r.error_for_status());
			match sent {
				Ok(response) => break response.json().await?,
				Err(err) if attempt < 3 => {
					tracing::warn!(method, "substrate rpc failed, retrying: {err}");
					tokio::time::sleep(delay).await;
					delay *= 2;
					attempt += 1;
				},
				Err(err) => return Err(anyhow!("{method}: {err}")),
			}
		};
		if let Some(error) = response.get("error") {
			bail!("{method}: {error}");
		}
		let result = response.get("result").cloned().unwrap_or(Value::Null);
		serde_json::from_value(result).with_context(|| format!("decoding {method} result"))
	}
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
