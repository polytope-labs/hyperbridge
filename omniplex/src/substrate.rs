//! Reads from Substrate chains: storage through substrate-eth-rpc, other RPCs directly.

use std::{sync::Arc, time::Duration};

use alloy::{
	eips::BlockId,
	primitives::{keccak256, Address, Bytes},
	providers::Provider,
	rpc::{json_rpc::ErrorPayload, types::TransactionRequest},
	sol,
	sol_types::{SolCall, SolValue},
};
use anyhow::{bail, Context};
use blake2::{Blake2b512, Digest};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::{
	chains::Chain,
	errors::{lacks_method, not_served, Retry},
	failover::Failover,
};

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
 * (`ismp_queryRequests`, `chain_getBlock`, ...). A request fails over between the node's URLs.
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

	/**
	 * Calls `method` on the first node that serves it. A node that answers it can't serve the
	 * call, lacks the method or doesn't answer hands the call to the next. With none serving
	 * it, the outcome a retry is likeliest to change decides the error: a can't-serve answer
	 * is a [`Retry`], a node that didn't answer is tried three more times, and only when every
	 * node lacks the method is the error lasting.
	 */
	pub async fn call<T: DeserializeOwned>(
		&self,
		method: &str,
		params: Value,
	) -> anyhow::Result<T> {
		let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
		let mut delay = Duration::from_millis(100);
		let mut attempt = 0;
		let response: Value = 'answered: loop {
			let (mut unserved, mut lacking, mut failure) = (None, None, None);
			for index in self.failover.order(self.urls.len()) {
				match self.post(index, &body).await {
					Ok(response) => match response.get("error").map(error_answer) {
						Some(answer) if lacks_method(&answer) => {
							lacking.get_or_insert(response);
						},
						Some(answer) if not_served(&answer) => {
							unserved.get_or_insert(response);
						},
						_ => {
							// A node that only lacks the method stays first for the others.
							if unserved.is_some() || failure.is_some() {
								self.failover.answered(index);
							}
							break 'answered response;
						},
					},
					Err(err) => {
						failure.get_or_insert(err);
					},
				}
			}
			if let Some(response) = unserved {
				return Err(Retry(format!("{method}: {}", response["error"])).into());
			}
			let Some(err) = failure else {
				let response = lacking.expect("a Substrate RPC has a URL");
				bail!("{method}: {}", response["error"]);
			};
			if attempt == 3 {
				return Err(anyhow::Error::new(err).context(method.to_string()));
			}
			tracing::warn!(method, "substrate rpc failed, retrying: {err}");
			tokio::time::sleep(delay).await;
			delay *= 2;
			attempt += 1;
		};
		if let Some(error) = response.get("error") {
			bail!("{method}: {error}");
		}
		let result = response.get("result").cloned().unwrap_or(Value::Null);
		serde_json::from_value(result).with_context(|| format!("decoding {method} result"))
	}

	/**
	 * Fails with a [`Retry`] until a node has block `number`. A node behind a block answers a
	 * lookup at its own head, as if the block held nothing.
	 */
	pub async fn ensure_imported(&self, number: u64) -> anyhow::Result<()> {
		let hash: Option<String> = self.call("chain_getBlockHash", json!([number])).await?;
		match hash {
			Some(_) => Ok(()),
			None => Err(Retry(format!("the node has not imported block {number}")).into()),
		}
	}

	async fn post(&self, index: usize, body: &Value) -> reqwest::Result<Value> {
		let request = self.http.post(&self.urls[index]).timeout(Duration::from_secs(30));
		request.json(body).send().await?.error_for_status()?.json().await
	}
}

/** A node's JSON-RPC error, as the type [`not_served`] and [`lacks_method`] read. */
fn error_answer(error: &Value) -> ErrorPayload {
	let code = error.get("code").and_then(Value::as_i64).unwrap_or_default();
	let message = error.get("message").and_then(Value::as_str).unwrap_or_default().to_string();
	ErrorPayload { code, message: message.into(), data: None }
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
	fn busy_answers_are_not_served() {
		let answer =
			|code: i64, message: &str| error_answer(&json!({ "code": code, "message": message }));
		// A Substrate node's own rate limit and connection limit, and a proxy's.
		assert!(not_served(&answer(-32999, "RPC rate limit exceeded")));
		assert!(not_served(&answer(-32009, "Server is busy, try again later")));
		assert!(not_served(&answer(-32000, "Too Many Requests")));
		assert!(not_served(&answer(9876, "Error fetching requests")));
		assert!(!not_served(&answer(-32601, "Method not found")));
		assert!(!not_served(&answer(9876, "failed to read block events")));
	}

	#[tokio::test]
	async fn a_call_moves_past_a_node_that_is_down_or_behind() {
		use crate::failover::tests::{closed_url, MockRpc, Reply};
		let live = MockRpc::start(Reply::Result("Hyperbridge (Nexus)")).await;
		let behind = MockRpc::start(Reply::Error(9876, "Invalid block number provided")).await;
		let lacking = MockRpc::start(Reply::Error(-32601, "Method not found")).await;
		for (first, order) in [
			(closed_url().await, [1, 0]),
			(behind.url.clone(), [1, 0]),
			// A node that only lacks the method stays first.
			(lacking.url.clone(), [0, 1]),
		] {
			let rpc = SubstrateRpc::new(&[&first, &live.url], reqwest::Client::new());
			let name: String = rpc.call("system_chain", json!([])).await.unwrap();
			assert_eq!(name, "Hyperbridge (Nexus)");
			assert_eq!(rpc.failover.order(2).collect::<Vec<_>>(), order);
		}
		// With no node serving it, the call is retried rather than given up.
		let rpc = SubstrateRpc::new(&[&behind.url], reqwest::Client::new());
		let err = rpc.call::<Value>("ismp_queryEvents", json!([1, 1])).await.unwrap_err();
		assert!(crate::errors::is_transient(&err), "{err:#}");
		let failed = MockRpc::start(Reply::Error(9876, "failed to read block events")).await;
		let rpc = SubstrateRpc::new(&[&failed.url], reqwest::Client::new());
		let err = rpc.call::<Value>("ismp_queryEvents", json!([1, 1])).await.unwrap_err();
		assert!(!crate::errors::is_transient(&err), "{err:#}");
	}

	#[tokio::test]
	async fn a_method_is_given_up_only_when_every_node_lacks_it() {
		use crate::failover::tests::{closed_url, MockRpc, Reply};
		let lacking = MockRpc::start(Reply::Error(-32601, "Method not found")).await;
		let call = |urls: Vec<String>| async move {
			let rpc = SubstrateRpc::new(&urls, reqwest::Client::new());
			rpc.call::<Value>("intents_getBidsForOrder", json!([])).await.unwrap_err()
		};
		let err = call(vec![lacking.url.clone()]).await;
		assert!(!crate::errors::is_transient(&err), "{err:#}");
		// Another node may have the method: one that is behind, or one that is down.
		let behind = MockRpc::start(Reply::Error(-32000, "State already discarded for 0x01")).await;
		let err = call(vec![lacking.url.clone(), behind.url.clone()]).await;
		assert!(crate::errors::is_transient(&err), "{err:#}");
		let err = call(vec![lacking.url.clone(), closed_url().await]).await;
		assert!(crate::errors::is_transient(&err), "{err:#}");
	}

	#[tokio::test]
	async fn a_block_the_node_has_not_imported_is_retried() {
		use crate::failover::tests::{MockRpc, Reply};
		let node = MockRpc::start(Reply::Null).await;
		let rpc = SubstrateRpc::new(&[&node.url], reqwest::Client::new());
		let err = rpc.ensure_imported(7).await.unwrap_err();
		assert!(crate::errors::is_transient(&err), "{err:#}");
		node.set(Reply::Result("0x01"));
		rpc.ensure_imported(7).await.unwrap();
	}

	#[tokio::test]
	async fn an_unreachable_node_is_transient() {
		let closed = crate::failover::tests::closed_url().await;
		let rpc = SubstrateRpc::new(&[&closed], reqwest::Client::new());
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
