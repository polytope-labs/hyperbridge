//! Block-pinned chain reads, and `readContracts`: reads batched through Multicall3.

use alloy::{
	eips::BlockId,
	primitives::{address, Address, Bytes},
	providers::{DynProvider, Provider},
	rpc::types::{Filter, Log, TransactionRequest},
	sol,
	sol_types::SolCall,
};
use anyhow::Context;
use async_trait::async_trait;
use futures::{stream, StreamExt};

use super::SolverChain;
use crate::errors::is_transient;

sol! {
	interface Multicall3 {
		struct Call3 { address target; bool allowFailure; bytes callData; }
		struct Result { bool success; bytes returnData; }
		function aggregate3(Call3[] calldata calls) external payable returns (Result[] memory returnData);
	}
}

/** Multicall3's deterministic address, the same on every chain it is deployed to. */
pub const MULTICALL3_ADDRESS: Address = address!("cA11bde05977b3631167028862bE2a173976CA11");
/** Reads per `aggregate3` call. */
pub const MULTICALL_BATCH_SIZE: usize = 250;
/** Requests in flight at once where reads are not batched. */
pub const MAX_CONCURRENT_READS: usize = 25;

/** The chain reads this area makes, each pinned to a block. Tests answer them from a model. */
#[async_trait]
pub trait Reads: Send + Sync {
	async fn call(&self, to: Address, data: Bytes, block: u64) -> anyhow::Result<Bytes>;
	async fn code(&self, address: Address, block: u64) -> anyhow::Result<Bytes>;
	async fn logs(&self, filter: &Filter) -> anyhow::Result<Vec<Log>>;
}

#[async_trait]
impl Reads for DynProvider {
	async fn call(&self, to: Address, data: Bytes, block: u64) -> anyhow::Result<Bytes> {
		let request = TransactionRequest::default().to(to).input(data.into());
		Ok(Provider::call(self, request).block(BlockId::number(block)).await?)
	}

	async fn code(&self, address: Address, block: u64) -> anyhow::Result<Bytes> {
		Ok(self.get_code_at(address).block_id(BlockId::number(block)).await?)
	}

	async fn logs(&self, filter: &Filter) -> anyhow::Result<Vec<Log>> {
		Ok(self.get_logs(filter).await?)
	}
}

/** One contract read: its target, calldata, and a label for its failure. */
#[derive(Debug, Clone)]
pub struct ContractRead {
	pub target: Address,
	pub call_data: Bytes,
	pub label: String,
}

impl ContractRead {
	pub fn new<C: SolCall>(target: Address, call: C) -> Self {
		let label =
			format!("{} on {target:#x}", C::SIGNATURE.split('(').next().unwrap_or_default());
		Self { target, call_data: call.abi_encode().into(), label }
	}

	/** Decodes this read's answer as `C`'s return; an undecodable answer fails the read. */
	pub fn decode<C: SolCall>(&self, answer: &Settled<Bytes>) -> Settled<C::Return> {
		let data = answer.as_ref().map_err(Clone::clone)?;
		C::abi_decode_returns(data).map_err(|err| format!("{}: {err}", self.label))
	}
}

/** One read's outcome: a revert or an undecodable answer fails that read alone. */
pub type Settled<T> = Result<T, String>;

/** Runs `work` over `items`, at most MAX_CONCURRENT_READS at a time, keeping their order. */
pub async fn map_concurrently<T, R, F, Fut>(items: Vec<T>, work: F) -> Vec<R>
where
	F: Fn(T) -> Fut,
	Fut: std::future::Future<Output = R>,
{
	stream::iter(items).map(work).buffered(MAX_CONCURRENT_READS).collect().await
}

/** The code of `address` at `block`, settled. */
pub async fn code(reads: &dyn Reads, address: Address, block: u64) -> Settled<Bytes> {
	reads
		.code(address, block)
		.await
		.map_err(|err| format!("getCode of {address:#x}: {err:#}"))
}

async fn has_multicall3(
	chain: &SolverChain,
	reads: &dyn Reads,
	block: u64,
) -> anyhow::Result<bool> {
	if *chain.multicall3.lock().unwrap() {
		return Ok(true);
	}
	let code = reads.code(MULTICALL3_ADDRESS, block).await.context("probing Multicall3")?;
	if code.is_empty() {
		return Ok(false);
	}
	*chain.multicall3.lock().unwrap() = true;
	Ok(true)
}

async fn aggregate(
	reads: &dyn Reads,
	batch: &[ContractRead],
	block: u64,
) -> anyhow::Result<Vec<Settled<Bytes>>> {
	let call = Multicall3::aggregate3Call {
		calls: batch
			.iter()
			.map(|read| Multicall3::Call3 {
				target: read.target,
				allowFailure: true,
				callData: read.call_data.clone(),
			})
			.collect(),
	};
	let answer = reads.call(MULTICALL3_ADDRESS, call.abi_encode().into(), block).await?;
	let results = Multicall3::aggregate3Call::abi_decode_returns(&answer)
		.context("decoding an aggregate3 answer")?;
	anyhow::ensure!(results.len() == batch.len(), "aggregate3 answered {} reads", results.len());
	Ok(batch
		.iter()
		.zip(results)
		.map(|(read, result)| {
			if result.success {
				Ok(result.returnData)
			} else {
				Err(format!("{} reverted", read.label))
			}
		})
		.collect())
}

/**
 * `readContracts`: runs `reads` at `block` and returns their answers in order, one eth_call per
 * MULTICALL_BATCH_SIZE reads. A single read, or a chain without Multicall3, is called directly
 * instead. Either way, a read that reverts fails alone, and an RPC that doesn't answer is an
 * error, which fails the batch for a retry.
 */
pub async fn read_contracts(
	chain: &SolverChain,
	reads: &dyn Reads,
	batch: Vec<ContractRead>,
	block: u64,
) -> anyhow::Result<Vec<Settled<Bytes>>> {
	if batch.is_empty() {
		return Ok(vec![]);
	}
	if batch.len() == 1 || !has_multicall3(chain, reads, block).await? {
		let answers = map_concurrently(batch, |read| async move {
			match reads.call(read.target, read.call_data.clone(), block).await {
				Ok(answer) => Ok(Ok(answer)),
				Err(err) if is_transient(&err) => Err(err),
				Err(err) => Ok(Err(format!("{}: {err:#}", read.label))),
			}
		})
		.await;
		return answers.into_iter().collect();
	}
	let answers = futures::future::try_join_all(
		batch.chunks(MULTICALL_BATCH_SIZE).map(|chunk| aggregate(reads, chunk, block)),
	)
	.await?;
	Ok(answers.into_iter().flatten().collect())
}
