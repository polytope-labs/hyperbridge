//! ISMP core, ported from `handlers/events/evmHost`, `handlers/events/erc6160ext20` and the
//! `ismp` pallet handlers in `handlers/events/substrateChains`: requests, GET requests and
//! responses, their statuses, state machine updates, relayer delivery stats and the transfers of
//! delivery transactions. The periodic pending-status flush lives in `services::pending`.

pub mod calldata;
pub mod commitment;
pub mod delivery;
pub mod evm;
pub mod requests;
pub mod state_machine;
pub mod substrate;

use std::{
	collections::{HashMap, VecDeque},
	sync::{Arc, Mutex},
};

use alloy::primitives::{keccak256, B256};

use crate::{
	handlers::Handlers,
	registry::{EventContext, LogMeta, Source, Subscription},
};

/** Every handler this area runs. */
pub fn register(handlers: &mut Handlers) {
	let shared = Arc::new(Shared::default());
	let host = Source::EthereumHost;
	let ismp = Source::Pallet("ismp");
	handlers.subscriptions.extend([
		Subscription::new(host, evm::PostRequestEvent(shared.clone())),
		Subscription::new(host, evm::PostRequestHandled(shared.clone())),
		Subscription::new(host, evm::PostRequestTimeoutHandled(shared.clone())),
		Subscription::new(host, evm::GetRequestEvent(shared.clone())),
		Subscription::new(host, evm::GetRequestHandled(shared.clone())),
		Subscription::new(host, evm::GetRequestTimeoutHandled(shared.clone())),
		Subscription::new(host, evm::StateMachineUpdated),
		Subscription::new(Source::Erc6160Ext20, evm::FeeTokenTransfer),
		Subscription::new(ismp, substrate::StateMachineUpdated(shared.clone())),
		Subscription::new(ismp, substrate::Request(shared.clone())),
		Subscription::new(ismp, substrate::Response(shared.clone())),
		Subscription::new(ismp, substrate::PostRequestHandled(shared.clone())),
		Subscription::new(ismp, substrate::PostRequestTimeoutHandled(shared.clone())),
		Subscription::new(ismp, substrate::GetRequestHandled(shared.clone())),
		Subscription::new(ismp, substrate::GetRequestTimeoutHandled(shared)),
	]);
}

/** Caches the handlers share: one delivery transaction emits many events. */
#[derive(Default)]
pub struct Shared {
	transactions: Mutex<VecDeque<(String, B256, Arc<delivery::TxData>)>>,
	timestamps: Mutex<HashMap<String, (u64, u64)>>,
}

const CACHED_TRANSACTIONS: usize = 32;

impl Shared {
	/** The receipt and calldata of the log's transaction, fetched once. */
	pub fn transaction(&self, ctx: &EventContext<'_>) -> Arc<delivery::TxData> {
		let mut recent = self.transactions.lock().unwrap();
		let hash = ctx.log.transaction_hash;
		if let Some((_, _, tx)) =
			recent.iter().find(|(network, h, _)| *h == hash && *network == ctx.chain.name)
		{
			return tx.clone();
		}
		let tx = Arc::new(delivery::TxData::new(hash));
		recent.push_back((ctx.chain.name.clone(), hash, tx.clone()));
		if recent.len() > CACHED_TRANSACTIONS {
			recent.pop_front();
		}
		tx
	}

	/**
	 * `getBlockTimestamp` on a Substrate chain: `Timestamp.Now` in milliseconds at the log's
	 * block. The facade's `blockTimestamp` is this divided by 1000.
	 */
	pub async fn substrate_timestamp_ms(&self, ctx: &EventContext<'_>) -> anyhow::Result<u64> {
		let block = ctx.log.block_number;
		if let Some((cached, ms)) = self.timestamps.lock().unwrap().get(&ctx.chain.name) {
			if *cached == block {
				return Ok(*ms);
			}
		}
		let raw = ctx.chain.storage(&TIMESTAMP_NOW_KEY, block).await?;
		let bytes: [u8; 8] = raw.get(..8).and_then(|b| b.try_into().ok()).ok_or_else(|| {
			anyhow::anyhow!("Timestamp.Now at block {block} is {} bytes", raw.len())
		})?;
		let ms = u64::from_le_bytes(bytes);
		self.timestamps.lock().unwrap().insert(ctx.chain.name.clone(), (block, ms));
		Ok(ms)
	}
}

/** `twox128("Timestamp") ++ twox128("Now")`. */
const TIMESTAMP_NOW_KEY: [u8; 32] =
	alloy::primitives::hex!("f0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb");

/** Hyperbridge's state machine ids on mainnet, testnet and a local network. */
pub fn is_hyperbridge(state_machine: &str) -> bool {
	matches!(state_machine, "POLKADOT-3367" | "KUSAMA-4009" | "KUSAMA-2000")
}

/** `isSubstrateChain`. */
pub fn is_substrate_chain(state_machine: &str) -> bool {
	["POLKADOT", "KUSAMA", "SUBSTRATE"]
		.iter()
		.any(|prefix| state_machine.starts_with(prefix))
}

/**
 * Whether a facade log came from an extrinsic. `Initialization` and `Finalization` events carry
 * a synthetic hash, `keccak256(block_hash ++ "init" | "final")`; SubQuery saw no extrinsic
 * there.
 */
pub fn from_extrinsic(log: &LogMeta) -> bool {
	let synthetic = |tag: &[u8]| {
		let mut preimage = log.block_hash.to_vec();
		preimage.extend_from_slice(tag);
		keccak256(preimage)
	};
	log.transaction_hash != synthetic(b"init") && log.transaction_hash != synthetic(b"final")
}

/** `extrinsic?.extrinsic.hash.toString() || ""`. */
pub fn extrinsic_hash(log: &LogMeta) -> String {
	if from_extrinsic(log) {
		format!("{:#x}", log.transaction_hash)
	} else {
		String::new()
	}
}

pub use crate::errors::{is_database_error, is_transient, Retry};

/**
 * Work the TypeScript ran in a `try` whose `catch` logged and moved on. Writes made before the
 * failure stay, as they did there. Transient failures still fail the batch, so a flaky RPC
 * doesn't silently drop stats: SubQuery dropped them, but only on the run that hit the error.
 * A database error is passed on too. A failed statement leaves the savepoint unusable, so the
 * handler's savepoint undoes its writes, or the batch fails, as the error says.
 */
pub fn non_critical(result: anyhow::Result<()>, what: &str) -> anyhow::Result<()> {
	match result {
		Err(err) if is_transient(&err) || is_database_error(&err) => Err(err),
		Err(err) => {
			tracing::warn!("{what}: {err:#}");
			Ok(())
		},
		Ok(()) => Ok(()),
	}
}

#[cfg(test)]
mod tests {
	use alloy::primitives::Address;

	use super::*;
	use crate::store::{normalize_timestamp, timestamp_to_date};

	/** `date.helpers.test.ts`: 11 digits or fewer are seconds, 12 or more milliseconds. */
	#[test]
	fn timestamps_normalize_like_timestamp_to_date() {
		assert_eq!(normalize_timestamp(1_713_787_200), 1_713_787_200_000);
		assert_eq!(normalize_timestamp(99_999_999_999), 99_999_999_999_000);
		assert_eq!(normalize_timestamp(123), 123_000);
		assert_eq!(normalize_timestamp(0), 0);
		assert_eq!(normalize_timestamp(1_713_787_200_000), 1_713_787_200_000);
		assert_eq!(normalize_timestamp(100_000_000_000), 100_000_000_000);
		assert_eq!(normalize_timestamp(171_378_720_012_345), 171_378_720_012_345);
		assert_eq!(timestamp_to_date(1_700_000_000), timestamp_to_date(1_700_000_000_000));
		assert_eq!(timestamp_to_date(0).and_utc().timestamp(), 0);
	}

	#[test]
	fn synthetic_transactions_are_not_extrinsics() {
		let block_hash = B256::repeat_byte(7);
		let mut log = LogMeta {
			address: Address::ZERO,
			block_number: 1,
			block_hash,
			block_timestamp: 0,
			transaction_hash: keccak256([block_hash.as_slice(), b"init"].concat()),
			transaction_index: 3,
			log_index: 0,
		};
		assert!(!from_extrinsic(&log));
		assert_eq!(extrinsic_hash(&log), "");
		log.transaction_hash = keccak256([block_hash.as_slice(), b"final"].concat());
		assert!(!from_extrinsic(&log));
		log.transaction_hash = B256::repeat_byte(1);
		assert!(from_extrinsic(&log));
		assert_eq!(extrinsic_hash(&log), format!("{:#x}", B256::repeat_byte(1)));
	}

	#[test]
	fn bad_data_is_not_transient() {
		assert!(!is_transient(&anyhow::anyhow!("transfer data 0x is no amount")));
		assert!(is_transient(&anyhow::anyhow!(Retry("no receipt".into()))));
	}
}
