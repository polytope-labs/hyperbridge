use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use codec::Encode;
use polkadot_sdk::{
	sc_client_api::{Backend, BlockBackend, StorageProvider},
	sp_api::{CallApiAt, CallContext, Metadata, ProvideRuntimeApi},
	sp_blockchain::HeaderBackend,
	sp_core::storage::{ChildInfo, ChildType, PrefixedStorageKey, StorageData, StorageKey},
	sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor, UniqueSaturatedInto},
};

use super::{BlockHeader, RawBlock, RpcError, RuntimeVersion, SubstrateRpc};
use crate::types::H256;

const METADATA_VERSION: u32 = 15;

pub struct ClientRpc<C, B, Block> {
	client: Arc<C>,
	_marker: PhantomData<(B, Block)>,
}

impl<C, B, Block> ClientRpc<C, B, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: PhantomData }
	}
}

fn transport(err: impl ToString) -> RpcError {
	RpcError::Transport(err.to_string())
}

fn hash(hash: polkadot_sdk::sp_core::H256) -> H256 {
	H256::from(hash.0)
}

fn block_header<Block>(hash: H256, header: &Block::Header) -> BlockHeader
where
	Block: BlockT<Hash = polkadot_sdk::sp_core::H256>,
	NumberFor<Block>: UniqueSaturatedInto<u64>,
{
	BlockHeader {
		hash,
		number: (*header.number()).unique_saturated_into(),
		parent_hash: self::hash(*header.parent_hash()),
		state_root: self::hash(*header.state_root()),
		extrinsics_root: self::hash(*header.extrinsics_root()),
	}
}

#[async_trait]
impl<C, B, Block> SubstrateRpc for ClientRpc<C, B, Block>
where
	Block: BlockT<Hash = polkadot_sdk::sp_core::H256>,
	NumberFor<Block>: TryFrom<u64> + UniqueSaturatedInto<u64>,
	B: Backend<Block> + Send + Sync + 'static,
	C: HeaderBackend<Block>
		+ BlockBackend<Block>
		+ StorageProvider<Block, B>
		+ CallApiAt<Block>
		+ ProvideRuntimeApi<Block>
		+ Send
		+ Sync
		+ 'static,
	C::Api: Metadata<Block>,
{
	async fn finalized_head(&self) -> Result<H256, RpcError> {
		Ok(hash(self.client.info().finalized_hash))
	}

	async fn block_hash(&self, number: u64) -> Result<Option<H256>, RpcError> {
		let Ok(number) = NumberFor::<Block>::try_from(number) else { return Ok(None) };
		Ok(self.client.hash(number).map_err(transport)?.map(hash))
	}

	async fn header(&self, at: H256) -> Result<Option<BlockHeader>, RpcError> {
		let header = self.client.header(at.0.into()).map_err(transport)?;
		Ok(header.map(|header| block_header::<Block>(at, &header)))
	}

	async fn block(&self, at: H256) -> Result<Option<RawBlock>, RpcError> {
		let Some(signed) = self.client.block(at.0.into()).map_err(transport)? else {
			return Ok(None);
		};
		let header = block_header::<Block>(at, signed.block.header());
		// `chain_getBlock` serialises each extrinsic as its SCALE encoding (length prefix
		// included), so hashing these bytes matches the standalone service and SubQuery.
		let extrinsics = signed.block.extrinsics().iter().map(Encode::encode).collect();
		Ok(Some(RawBlock {
			hash: at,
			number: header.number,
			parent_hash: header.parent_hash,
			state_root: header.state_root,
			extrinsics_root: header.extrinsics_root,
			extrinsics,
		}))
	}

	async fn storage(&self, key: &[u8], at: H256) -> Result<Option<Vec<u8>>, RpcError> {
		let value =
			self.client.storage(at.0.into(), &StorageKey(key.to_vec())).map_err(transport)?;
		Ok(value.map(|StorageData(bytes)| bytes))
	}

	async fn child_storage(
		&self,
		child_key: &[u8],
		key: &[u8],
		at: H256,
	) -> Result<Option<Vec<u8>>, RpcError> {
		// `child_key` is the `:child_storage:default:`-prefixed key `childstate_getStorage`
		// takes, so strip the prefix the same way the node's own RPC does.
		let prefixed = PrefixedStorageKey::new(child_key.to_vec());
		let child_info = match ChildType::from_prefixed_key(&prefixed) {
			Some((ChildType::ParentKeyId, inner)) => ChildInfo::new_default(inner),
			None => return Err(RpcError::Decode("invalid child storage key".into())),
		};
		let value = self
			.client
			.child_storage(at.0.into(), &child_info, &StorageKey(key.to_vec()))
			.map_err(transport)?;
		Ok(value.map(|StorageData(bytes)| bytes))
	}

	async fn runtime_version(&self, at: H256) -> Result<RuntimeVersion, RpcError> {
		let version = self
			.client
			.runtime_version_at(at.0.into(), CallContext::Offchain)
			.map_err(transport)?;
		Ok(RuntimeVersion {
			spec_name: version.spec_name.to_string(),
			spec_version: version.spec_version,
			transaction_version: version.transaction_version,
		})
	}

	async fn metadata(&self, at: H256) -> Result<Vec<u8>, RpcError> {
		let api = self.client.runtime_api();
		match api.metadata_at_version(at.0.into(), METADATA_VERSION) {
			Ok(Some(metadata)) => return Ok(metadata.to_vec()),
			Ok(None) => {},
			Err(err) => {
				tracing::debug!(%err, "Metadata_metadata_at_version unavailable, falling back")
			},
		}
		let metadata = api.metadata(at.0.into()).map_err(transport)?;
		Ok(metadata.to_vec())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use polkadot_sdk::sp_runtime::OpaqueExtrinsic;

	#[test]
	fn opaque_extrinsic_encodes_with_length_prefix() {
		let raw = vec![0x04u8, 0x01, 0xaa, 0xbb];
		let opaque = OpaqueExtrinsic::from_blob(raw.clone());
		let mut expected = codec::Compact(raw.len() as u32).encode();
		expected.extend_from_slice(&raw);
		assert_eq!(opaque.encode(), expected);
		assert_eq!(OpaqueExtrinsic::try_from_encoded_extrinsic(&expected).unwrap(), opaque);
	}

	#[test]
	fn rejects_unprefixed_child_key() {
		let prefixed = PrefixedStorageKey::new(b"not a child key".to_vec());
		assert!(ChildType::from_prefixed_key(&prefixed).is_none());
		let prefixed = PrefixedStorageKey::new(
			[b":child_storage:default:".as_slice(), b"ismp".as_slice()].concat(),
		);
		let (child_type, inner) = ChildType::from_prefixed_key(&prefixed).unwrap();
		assert_eq!(child_type, ChildType::ParentKeyId);
		assert_eq!(inner, b"ismp");
	}
}
