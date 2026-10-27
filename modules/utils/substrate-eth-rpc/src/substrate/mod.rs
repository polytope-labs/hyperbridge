use std::ops::Range;

use async_trait::async_trait;
use futures::{stream, StreamExt, TryStreamExt};
use sp_crypto_hashing::twox_128;

use crate::types::H256;

pub mod client;
pub mod fixture;

pub use client::LiveRpc;
pub use fixture::{FixtureBlock, FixtureRpc};

#[derive(Debug, thiserror::Error)]
pub enum RpcError {
	#[error("rpc transport: {0}")]
	Transport(String),
	#[error("block {0} not found")]
	BlockNotFound(u64),
	#[error("block hash {0} not found")]
	HashNotFound(H256),
	#[error("decode: {0}")]
	Decode(String),
	#[error("fixture: {0}")]
	Fixture(String),
}

const RATE_LIMIT_MARKERS: &[&str] = &["rate limit", "too many requests"];
const RATE_LIMIT_CODES: &[&str] = &["-32999", "429"];
const TRANSIENT_MARKERS: &[&str] = &[
	"timeout",
	"timed out",
	"connection was lost",
	"connection was closed",
	"connection closed",
	"connection reset",
	"reconnect",
	"not connected",
	"disconnected",
	"restart required",
	"broken pipe",
	"unexpected eof",
	"end of file",
	"bad gateway",
	"service unavailable",
	"gateway timeout",
	"internal server error",
];
const SERVER_STATUS_CODES: &[&str] = &["500", "502", "503", "504"];

impl RpcError {
	pub fn is_rate_limited(&self) -> bool {
		let RpcError::Transport(message) = self else { return false };
		let message = message.to_ascii_lowercase();
		RATE_LIMIT_MARKERS.iter().any(|marker| message.contains(marker)) ||
			RATE_LIMIT_CODES.iter().any(|code| contains_token(&message, code))
	}

	pub fn is_transient(&self) -> bool {
		let RpcError::Transport(message) = self else { return false };
		let message = message.to_ascii_lowercase();
		self.is_rate_limited() ||
			TRANSIENT_MARKERS.iter().any(|marker| message.contains(marker)) ||
			SERVER_STATUS_CODES.iter().any(|code| contains_token(&message, code))
	}
}

fn contains_token(message: &str, token: &str) -> bool {
	message.match_indices(token).any(|(start, _)| {
		let before = message[..start].chars().next_back();
		let after = message[start + token.len()..].chars().next();
		!before.is_some_and(|c| c.is_ascii_alphanumeric()) &&
			!after.is_some_and(|c| c.is_ascii_alphanumeric())
	})
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockHeader {
	pub hash: H256,
	pub number: u64,
	pub parent_hash: H256,
	pub state_root: H256,
	pub extrinsics_root: H256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawBlock {
	pub hash: H256,
	pub number: u64,
	pub parent_hash: H256,
	pub state_root: H256,
	pub extrinsics_root: H256,
	pub extrinsics: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeVersion {
	pub spec_name: String,
	pub spec_version: u32,
	pub transaction_version: u32,
}

const BATCH_CONCURRENCY: usize = 16;

#[async_trait]
pub trait SubstrateRpc: Send + Sync {
	async fn finalized_head(&self) -> Result<H256, RpcError>;

	async fn block_hash(&self, number: u64) -> Result<Option<H256>, RpcError>;

	async fn header(&self, hash: H256) -> Result<Option<BlockHeader>, RpcError>;

	async fn block(&self, hash: H256) -> Result<Option<RawBlock>, RpcError>;

	async fn storage(&self, key: &[u8], at: H256) -> Result<Option<Vec<u8>>, RpcError>;

	async fn child_storage(
		&self,
		child_key: &[u8],
		key: &[u8],
		at: H256,
	) -> Result<Option<Vec<u8>>, RpcError>;

	async fn runtime_version(&self, at: H256) -> Result<RuntimeVersion, RpcError>;

	async fn metadata(&self, at: H256) -> Result<Vec<u8>, RpcError>;

	async fn block_by_number(&self, number: u64) -> Result<RawBlock, RpcError> {
		let hash = self.block_hash(number).await?.ok_or(RpcError::BlockNotFound(number))?;
		self.block(hash).await?.ok_or(RpcError::BlockNotFound(number))
	}

	async fn batch_blocks(&self, numbers: Range<u64>) -> Result<Vec<RawBlock>, RpcError> {
		let mut blocks: Vec<RawBlock> = stream::iter(numbers)
			.map(|number| self.block_by_number(number))
			.buffer_unordered(BATCH_CONCURRENCY)
			.try_collect()
			.await?;
		blocks.sort_by_key(|block| block.number);
		Ok(blocks)
	}
}

pub fn storage_prefix(pallet: &str, item: &str) -> Vec<u8> {
	let mut key = twox_128(pallet.as_bytes()).to_vec();
	key.extend_from_slice(&twox_128(item.as_bytes()));
	key
}

pub fn system_events_key() -> Vec<u8> {
	storage_prefix("System", "Events")
}

pub fn timestamp_now_key() -> Vec<u8> {
	storage_prefix("Timestamp", "Now")
}

#[cfg(test)]
mod tests {
	use super::*;
	use alloy_primitives::hex;

	#[test]
	fn pinned_system_events_key() {
		assert_eq!(
			hex::encode_prefixed(system_events_key()),
			"0x26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7"
		);
	}

	#[test]
	fn classifies_transient_transport_errors() {
		let transport = |message: &str| RpcError::Transport(message.into());
		let rate_limited = transport("User error: RPC rate limit exceeded (-32999)");
		assert!(rate_limited.is_rate_limited());
		assert!(rate_limited.is_transient());
		assert!(transport("Connection rejected with status code: 429").is_rate_limited());
		assert!(transport("Too Many Requests").is_rate_limited());
		assert!(transport("User error: something (-32999)").is_rate_limited());

		for message in [
			"Request timeout",
			"RPC error: the connection was lost (closed); reconnect automatically initiated",
			"The background task closed client is not connected; restart required",
			"Connection reset by peer",
			"Connection rejected with status code: 503",
			"502 Bad Gateway",
		] {
			let err = transport(message);
			assert!(err.is_transient(), "{message}");
			assert!(!err.is_rate_limited(), "{message}");
		}

		for err in [
			transport("injected failure at block 101"),
			transport("User error: Unknown block (-32602)"),
			transport("block 0x1a429b failed"),
			transport("block 5031 is 429x"),
			RpcError::Decode("rate limit".into()),
			RpcError::Fixture("timeout".into()),
			RpcError::BlockNotFound(429),
			RpcError::HashNotFound(H256::ZERO),
		] {
			assert!(!err.is_transient(), "{err}");
			assert!(!err.is_rate_limited(), "{err}");
		}
	}

	#[test]
	fn pinned_timestamp_now_key() {
		assert_eq!(
			hex::encode_prefixed(timestamp_now_key()),
			"0xf0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb"
		);
	}
}
