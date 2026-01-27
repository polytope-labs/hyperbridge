use alloc::vec::Vec;

/// Chain-specific key layouts for EVM-like modules under Tendermint/CometBFT chains.

/// Trait for chains exposing EVM-like KV layout under a Cosmos SDK store.
pub trait EvmStoreKeys: Send + Sync {
	/// Return the module store key (e.g., "evm").
	fn store_key() -> &'static str;

	/// Storage key = `0x03 || <20-byte address> || <32-byte slot>`
	fn storage_key(addr: &[u8; 20], slot: [u8; 32]) -> Vec<u8>;
}

/// Sei chain implementation of EvmStoreKeys
pub struct SeiEvmKeys;

impl EvmStoreKeys for SeiEvmKeys {
	fn store_key() -> &'static str {
		"evm"
	}

	fn storage_key(addr: &[u8; 20], slot: [u8; 32]) -> Vec<u8> {
		let mut k = Vec::with_capacity(1 + 20 + 32);
		k.push(0x03);
		k.extend_from_slice(addr);
		k.extend_from_slice(&slot);
		k
	}
}

/// Uses the standard ethermint/evmos EVM module
/// Chains: Kava, XrpEvm, Cronos, Injective
pub struct DefaultEvmKeys;

impl EvmStoreKeys for DefaultEvmKeys {
	fn store_key() -> &'static str {
		"evm"
	}

	fn storage_key(addr: &[u8; 20], slot: [u8; 32]) -> Vec<u8> {
		let mut k = Vec::with_capacity(1 + 20 + 32);
		k.push(0x02);
		k.extend_from_slice(addr);
		k.extend_from_slice(&slot);
		k
	}
}
