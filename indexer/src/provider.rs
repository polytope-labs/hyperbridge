use std::sync::Arc;

use alloy::{
	network::{AnyRpcBlock, AnyTransactionReceipt},
	primitives::{Bytes, TxHash, B256, U64},
	providers::{DynProvider, Provider},
	rpc::types::{trace::parity::LocalizedTransactionTrace, Filter, Log},
};
use alloy_chains::Chain;
use async_trait::async_trait;
use rindexer::{
	event::RindexerEventFilter,
	notifications::ChainStateNotification,
	provider::{ChainProvider, JsonRpcCachedProvider, ProviderError},
};
use tokio::sync::broadcast::Sender;

/**
 * rindexer's provider, except that `eth_getLogs` asks for every topic the chain's handlers
 * subscribe to. rindexer then runs one stream per chain, and handlers see the chain's logs in
 * block order, as SubQuery delivered them. rindexer's range splitting and retries still apply:
 * they act on the RPC errors this returns.
 */
#[derive(Debug)]
pub struct AllTopicsProvider {
	inner: Arc<JsonRpcCachedProvider>,
	rpc: DynProvider,
	topics: Vec<B256>,
}

impl AllTopicsProvider {
	pub fn new(inner: Arc<JsonRpcCachedProvider>, rpc: DynProvider, topics: Vec<B256>) -> Self {
		Self { inner, rpc, topics }
	}
}

#[async_trait]
impl ChainProvider for AllTopicsProvider {
	fn chain(&self) -> Chain {
		self.inner.chain()
	}

	fn max_block_range(&self) -> Option<U64> {
		self.inner.max_block_range()
	}

	fn chain_state_notification(&self) -> Option<Sender<ChainStateNotification>> {
		self.inner.chain_state_notification()
	}

	async fn get_latest_block(&self) -> Result<Option<Arc<AnyRpcBlock>>, ProviderError> {
		self.inner.get_latest_block().await
	}

	async fn get_block_number(&self) -> Result<U64, ProviderError> {
		self.inner.get_block_number().await
	}

	async fn get_logs(&self, filter: &RindexerEventFilter) -> Result<Vec<Log>, ProviderError> {
		let addresses: Vec<_> = match filter.contract_addresses().await {
			Some(addresses) if addresses.is_empty() => return Ok(vec![]),
			Some(addresses) => addresses.into_iter().collect(),
			None =>
				return Err(ProviderError::CustomError(
					"refusing an eth_getLogs with no address filter".into(),
				)),
		};
		let request = Filter::new()
			.address(addresses)
			.event_signature(self.topics.clone())
			.from_block(filter.from_block().to::<u64>())
			.to_block(filter.to_block().to::<u64>());
		Ok(self.rpc.get_logs(&request).await?)
	}

	async fn get_block_by_number_batch(
		&self,
		block_numbers: &[U64],
		include_txs: bool,
	) -> Result<Vec<AnyRpcBlock>, ProviderError> {
		self.inner.get_block_by_number_batch(block_numbers, include_txs).await
	}

	async fn get_block_by_number_batch_with_size(
		&self,
		block_numbers: &[U64],
		include_txs: bool,
		rpc_batch_size: Option<usize>,
	) -> Result<Vec<AnyRpcBlock>, ProviderError> {
		self.inner
			.get_block_by_number_batch_with_size(block_numbers, include_txs, rpc_batch_size)
			.await
	}

	async fn get_tx_receipts_batch(
		&self,
		hashes: &[TxHash],
	) -> Result<Vec<AnyTransactionReceipt>, ProviderError> {
		self.inner.get_tx_receipts_batch(hashes).await
	}

	async fn trace_block(
		&self,
		block_number: U64,
	) -> Result<Vec<LocalizedTransactionTrace>, ProviderError> {
		self.inner.trace_block(block_number).await
	}

	async fn debug_trace_block_by_number(
		&self,
		block_number: U64,
	) -> Result<Vec<LocalizedTransactionTrace>, ProviderError> {
		self.inner.debug_trace_block_by_number(block_number).await
	}

	async fn eth_call(
		&self,
		to: alloy::primitives::Address,
		data: Bytes,
		block_number: u64,
	) -> Result<String, ProviderError> {
		self.inner.eth_call(to, data, block_number).await
	}

	async fn eth_call_latest(
		&self,
		to: alloy::primitives::Address,
		data: Bytes,
	) -> Result<String, ProviderError> {
		self.inner.eth_call_latest(to, data).await
	}
}
