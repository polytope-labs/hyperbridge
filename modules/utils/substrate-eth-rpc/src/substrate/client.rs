use std::time::Duration;

use alloy_primitives::{hex, Bytes};
use async_trait::async_trait;
use codec::{Decode, Encode};
use serde::Deserialize;
use subxt::{
	backend::rpc::RpcClient,
	ext::subxt_rpcs::{
		client::reconnecting_rpc_client::{PingConfig, RpcClientBuilder},
		rpc_params,
	},
};

use super::{BlockHeader, RawBlock, RpcError, RuntimeVersion, SubstrateRpc};
use crate::types::H256;

const MAX_PAYLOAD_SIZE: u32 = 32 * 1024 * 1024;
const METADATA_VERSION: u32 = 15;

pub struct LiveRpc {
	rpc: RpcClient,
}

impl LiveRpc {
	pub async fn connect(urls: &[String]) -> Result<Self, RpcError> {
		let mut failures = Vec::new();
		for url in urls {
			match Self::connect_one(url).await {
				Ok(rpc) => return Ok(Self { rpc }),
				Err(err) => {
					tracing::warn!(url, %err, "substrate rpc connect failed");
					failures.push(format!("{url}: {err}"));
				},
			}
		}
		if failures.is_empty() {
			failures.push("no rpc urls configured".into());
		}
		Err(RpcError::Transport(failures.join("; ")))
	}

	async fn connect_one(url: &str) -> Result<RpcClient, RpcError> {
		let client = RpcClientBuilder::new()
			.max_request_size(MAX_PAYLOAD_SIZE)
			.max_response_size(MAX_PAYLOAD_SIZE)
			.enable_ws_ping(
				PingConfig::new()
					.ping_interval(Duration::from_secs(6))
					.inactive_limit(Duration::from_secs(30)),
			)
			.build(url)
			.await
			.map_err(|err| RpcError::Transport(err.to_string()))?;
		Ok(RpcClient::new(client))
	}

	async fn metadata_at_version(&self, at: H256) -> Result<Option<Vec<u8>>, RpcError> {
		let args = hex::encode_prefixed(METADATA_VERSION.encode());
		let raw: Bytes = self
			.rpc
			.request("state_call", rpc_params!["Metadata_metadata_at_version", args, at])
			.await
			.map_err(transport)?;
		Option::<Vec<u8>>::decode(&mut &raw[..]).map_err(|err| RpcError::Decode(err.to_string()))
	}
}

#[derive(Deserialize)]
struct BlockResponse {
	block: BlockBody,
}

#[derive(Deserialize)]
struct BlockBody {
	header: Header,
	extrinsics: Vec<Bytes>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Header {
	parent_hash: H256,
	number: String,
	state_root: H256,
	extrinsics_root: H256,
}

impl Header {
	fn into_block_header(self, hash: H256) -> Result<BlockHeader, RpcError> {
		Ok(BlockHeader {
			hash,
			number: parse_hex_number(&self.number)?,
			parent_hash: self.parent_hash,
			state_root: self.state_root,
			extrinsics_root: self.extrinsics_root,
		})
	}
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeVersionResponse {
	spec_name: String,
	spec_version: u32,
	transaction_version: u32,
}

fn transport(err: subxt::ext::subxt_rpcs::Error) -> RpcError {
	RpcError::Transport(err.to_string())
}

fn parse_hex_number(raw: &str) -> Result<u64, RpcError> {
	let digits = raw.strip_prefix("0x").unwrap_or(raw);
	u64::from_str_radix(digits, 16)
		.map_err(|err| RpcError::Decode(format!("block number {raw:?}: {err}")))
}

#[async_trait]
impl SubstrateRpc for LiveRpc {
	async fn finalized_head(&self) -> Result<H256, RpcError> {
		self.rpc
			.request("chain_getFinalizedHead", rpc_params![])
			.await
			.map_err(transport)
	}

	async fn block_hash(&self, number: u64) -> Result<Option<H256>, RpcError> {
		self.rpc
			.request("chain_getBlockHash", rpc_params![number])
			.await
			.map_err(transport)
	}

	async fn header(&self, hash: H256) -> Result<Option<BlockHeader>, RpcError> {
		let response: Option<Header> = self
			.rpc
			.request("chain_getHeader", rpc_params![hash])
			.await
			.map_err(transport)?;
		response.map(|header| header.into_block_header(hash)).transpose()
	}

	async fn block(&self, hash: H256) -> Result<Option<RawBlock>, RpcError> {
		let response: Option<BlockResponse> =
			self.rpc.request("chain_getBlock", rpc_params![hash]).await.map_err(transport)?;
		let Some(BlockResponse { block }) = response else { return Ok(None) };
		let header = block.header.into_block_header(hash)?;
		Ok(Some(RawBlock {
			hash,
			number: header.number,
			parent_hash: header.parent_hash,
			state_root: header.state_root,
			extrinsics_root: header.extrinsics_root,
			extrinsics: block.extrinsics.into_iter().map(|ext| ext.to_vec()).collect(),
		}))
	}

	async fn storage(&self, key: &[u8], at: H256) -> Result<Option<Vec<u8>>, RpcError> {
		let value: Option<Bytes> = self
			.rpc
			.request("state_getStorage", rpc_params![hex::encode_prefixed(key), at])
			.await
			.map_err(transport)?;
		Ok(value.map(|value| value.to_vec()))
	}

	async fn child_storage(
		&self,
		child_key: &[u8],
		key: &[u8],
		at: H256,
	) -> Result<Option<Vec<u8>>, RpcError> {
		let params = rpc_params![hex::encode_prefixed(child_key), hex::encode_prefixed(key), at];
		let value: Option<Bytes> =
			self.rpc.request("childstate_getStorage", params).await.map_err(transport)?;
		Ok(value.map(|value| value.to_vec()))
	}

	async fn runtime_version(&self, at: H256) -> Result<RuntimeVersion, RpcError> {
		let version: RuntimeVersionResponse = self
			.rpc
			.request("state_getRuntimeVersion", rpc_params![at])
			.await
			.map_err(transport)?;
		Ok(RuntimeVersion {
			spec_name: version.spec_name,
			spec_version: version.spec_version,
			transaction_version: version.transaction_version,
		})
	}

	async fn metadata(&self, at: H256) -> Result<Vec<u8>, RpcError> {
		match self.metadata_at_version(at).await {
			Ok(Some(metadata)) => return Ok(metadata),
			Ok(None) => {},
			Err(err) => {
				tracing::debug!(%err, "Metadata_metadata_at_version unavailable, falling back")
			},
		}
		let raw: Bytes = self
			.rpc
			.request("state_getMetadata", rpc_params![at])
			.await
			.map_err(transport)?;
		Ok(raw.to_vec())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_hex_block_numbers() {
		assert_eq!(parse_hex_number("0x1a").unwrap(), 26);
		assert_eq!(parse_hex_number("0x0").unwrap(), 0);
		assert!(parse_hex_number("0xzz").is_err());
	}

	#[test]
	fn decodes_chain_get_block_shape() {
		let json = r#"{
			"block": {
				"header": {
					"parentHash": "0x1111111111111111111111111111111111111111111111111111111111111111",
					"number": "0x2a",
					"stateRoot": "0x2222222222222222222222222222222222222222222222222222222222222222",
					"extrinsicsRoot": "0x3333333333333333333333333333333333333333333333333333333333333333",
					"digest": { "logs": [] }
				},
				"extrinsics": ["0x0401", "0x"]
			},
			"justifications": null
		}"#;
		let response: BlockResponse = serde_json::from_str(json).unwrap();
		assert_eq!(parse_hex_number(&response.block.header.number).unwrap(), 42);
		assert_eq!(response.block.extrinsics[0].to_vec(), vec![0x04, 0x01]);
		assert!(response.block.extrinsics[1].is_empty());

		let header = response.block.header.into_block_header(H256::repeat_byte(0xaa)).unwrap();
		assert_eq!(header.hash, H256::repeat_byte(0xaa));
		assert_eq!(header.number, 42);
		assert_eq!(header.parent_hash, H256::repeat_byte(0x11));
		assert_eq!(header.state_root, H256::repeat_byte(0x22));
		assert_eq!(header.extrinsics_root, H256::repeat_byte(0x33));
	}
}
