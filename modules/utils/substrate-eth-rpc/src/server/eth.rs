use std::{future::Future, sync::Arc};

use alloy_consensus::EMPTY_OMMER_ROOT_HASH;
use alloy_primitives::{Address, Bytes, TxKind, B256, B64, U256, U64};
use alloy_rpc_types_eth::{
	Block, BlockId, BlockNumberOrTag, BlockTransactions, Filter, FilterBlockOption, Header, Log,
	TransactionRequest,
};
use alloy_sol_types::{sol, sol_data, SolInterface, SolType};
use jsonrpsee::{
	core::RpcResult,
	types::{ErrorObjectOwned, Params},
	RpcModule,
};
use serde::Serialize;
use tracing::Instrument;

use super::error::{invalid_params, server_error, to_rpc_error};
use crate::{
	blocks::BlockService,
	types::{storage_address, DecodedBlock, EthLog},
};

sol! {
	interface SubstrateStorage {
		function getStorage(bytes key) external view returns (bytes value);
		function getChildStorage(bytes childKey, bytes key) external view returns (bytes value);
	}
}

pub struct ChainRpc {
	service: Arc<BlockService>,
}

impl ChainRpc {
	pub fn new(service: Arc<BlockService>) -> Self {
		Self { service }
	}

	pub fn into_module(self) -> RpcModule<ChainRpc> {
		let mut module = RpcModule::new(self);
		register(&mut module, "eth_chainId", |rpc, _| async move { rpc.chain_id() });
		register(&mut module, "net_version", |rpc, _| async move { rpc.net_version() });
		register(&mut module, "eth_blockNumber", |rpc, _| async move { rpc.block_number().await });
		register(&mut module, "eth_getBlockByNumber", |rpc, params| async move {
			rpc.block_by_number(params).await
		});
		register(&mut module, "eth_getBlockByHash", |rpc, params| async move {
			rpc.block_by_hash(params).await
		});
		register(&mut module, "eth_getLogs", |rpc, params| async move { rpc.logs(params).await });
		register(&mut module, "eth_call", |rpc, params| async move { rpc.call(params).await });
		module
	}

	fn chain_id(&self) -> RpcResult<U64> {
		Ok(U64::from(self.service.chain().eth_chain_id))
	}

	fn net_version(&self) -> RpcResult<String> {
		Ok(self.service.chain().eth_chain_id.to_string())
	}

	async fn block_number(&self) -> RpcResult<U64> {
		let head = self.service.finalized_number().await.map_err(to_rpc_error)?;
		Ok(U64::from(head))
	}

	async fn block_by_number(&self, params: Params<'static>) -> RpcResult<Option<Block<B256>>> {
		let mut seq = params.sequence();
		let tag: BlockNumberOrTag = seq.next()?;
		let _full: Option<bool> = seq.optional_next()?;
		let number = self.resolve_tag(tag).await?;
		let block = self.service.block_by_number(number).await.map_err(to_rpc_error)?;
		Ok(block.map(|block| to_rpc_block(&block)))
	}

	async fn block_by_hash(&self, params: Params<'static>) -> RpcResult<Option<Block<B256>>> {
		let mut seq = params.sequence();
		let hash: B256 = seq.next()?;
		let _full: Option<bool> = seq.optional_next()?;
		let block = self.service.block_by_hash(hash).await.map_err(to_rpc_error)?;
		Ok(block.map(|block| to_rpc_block(&block)))
	}

	async fn logs(&self, params: Params<'static>) -> RpcResult<Vec<Log>> {
		let filter: Filter = params.sequence().next()?;
		let (from, to) = match filter.block_option {
			FilterBlockOption::AtBlockHash(_) =>
				return Err(invalid_params("blockHash filter not supported")),
			FilterBlockOption::Range { from_block, to_block } => (
				self.resolve_tag(from_block.unwrap_or(BlockNumberOrTag::Finalized)).await?,
				self.resolve_tag(to_block.unwrap_or(BlockNumberOrTag::Finalized)).await?,
			),
		};
		let blocks = self.service.range(from, to).await.map_err(to_rpc_error)?;
		let logs = blocks
			.iter()
			.flat_map(|block| block.logs.iter())
			.map(to_rpc_log)
			.filter(|log| filter.matches(&log.inner))
			.collect();
		Ok(logs)
	}

	async fn call(&self, params: Params<'static>) -> RpcResult<Bytes> {
		let mut seq = params.sequence();
		let request: TransactionRequest = seq.next()?;
		let block: Option<BlockId> = seq.optional_next()?;

		if request.to != Some(TxKind::Call(storage_address())) {
			return Err(invalid_params("only the storage address is callable"));
		}
		let data = request.input.input().cloned().unwrap_or_default();
		let call = SubstrateStorage::SubstrateStorageCalls::abi_decode(&data)
			.map_err(|e| invalid_params(format!("unknown storage call: {e}")))?;
		let at = self
			.resolve_block_id(block.unwrap_or(BlockId::Number(BlockNumberOrTag::Finalized)))
			.await?;

		let value = match call {
			SubstrateStorage::SubstrateStorageCalls::getStorage(call) =>
				self.service.storage(&call.key, at).await,
			SubstrateStorage::SubstrateStorageCalls::getChildStorage(call) =>
				self.service.child_storage(&call.childKey, &call.key, at).await,
		}
		.map_err(to_rpc_error)?;
		Ok(Bytes::from(sol_data::Bytes::abi_encode(&value.unwrap_or_default())))
	}

	async fn resolve_tag(&self, tag: BlockNumberOrTag) -> RpcResult<u64> {
		match tag {
			BlockNumberOrTag::Number(number) => Ok(number),
			BlockNumberOrTag::Earliest => Ok(0),
			BlockNumberOrTag::Latest |
			BlockNumberOrTag::Finalized |
			BlockNumberOrTag::Safe |
			BlockNumberOrTag::Pending => self.service.finalized_number().await.map_err(to_rpc_error),
		}
	}

	async fn resolve_block_id(&self, id: BlockId) -> RpcResult<u64> {
		match id {
			BlockId::Number(tag) => self.resolve_tag(tag).await,
			BlockId::Hash(hash) => {
				let block = self
					.service
					.block_by_hash(hash.block_hash)
					.await
					.map_err(to_rpc_error)?
					.ok_or_else(|| {
						server_error(format!("block hash {} not found", hash.block_hash))
					})?;
				Ok(block.number)
			},
		}
	}
}

fn register<R, F, Fut>(module: &mut RpcModule<ChainRpc>, name: &'static str, handler: F)
where
	R: Serialize + Clone + 'static,
	F: Fn(Arc<ChainRpc>, Params<'static>) -> Fut + Clone + Send + Sync + 'static,
	Fut: Future<Output = Result<R, ErrorObjectOwned>> + Send + 'static,
{
	module
		.register_async_method(name, move |params, rpc, _| {
			handler(rpc, params).instrument(tracing::debug_span!("rpc", method = name))
		})
		.expect("method names are unique");
}

pub fn to_rpc_block(block: &DecodedBlock) -> Block<B256> {
	let inner = alloy_consensus::Header {
		parent_hash: block.parent_hash,
		ommers_hash: EMPTY_OMMER_ROOT_HASH,
		beneficiary: Address::ZERO,
		state_root: block.state_root,
		transactions_root: block.extrinsics_root,
		receipts_root: B256::ZERO,
		logs_bloom: block.logs_bloom,
		difficulty: U256::ZERO,
		number: block.number,
		gas_limit: 0,
		gas_used: 0,
		timestamp: block.timestamp,
		extra_data: Bytes::new(),
		mix_hash: B256::ZERO,
		nonce: B64::ZERO,
		base_fee_per_gas: None,
		withdrawals_root: None,
		blob_gas_used: None,
		excess_blob_gas: None,
		parent_beacon_block_root: None,
		requests_hash: None,
	};
	Block {
		header: Header { hash: block.hash, inner, total_difficulty: None, size: None },
		uncles: Vec::new(),
		transactions: BlockTransactions::Hashes(block.extrinsic_hashes.clone()),
		withdrawals: None,
	}
}

pub fn to_rpc_log(log: &EthLog) -> Log {
	Log {
		inner: alloy_primitives::Log::new_unchecked(
			log.address,
			log.topics.clone(),
			log.data.clone(),
		),
		block_hash: Some(log.block_hash),
		block_number: Some(log.block_number),
		block_timestamp: Some(log.block_timestamp),
		transaction_hash: Some(log.transaction_hash),
		transaction_index: Some(log.transaction_index),
		log_index: Some(log.log_index),
		removed: false,
	}
}
