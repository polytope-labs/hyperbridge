//! `StateMachineService` and `state-machine.helper.ts`: `state_machine_update_events`, and the
//! state commitment timestamp each row carries.

use alloy::{
	eips::BlockId,
	primitives::{keccak256, Address, B256, U256},
	providers::Provider,
};
use anyhow::{anyhow, bail};
use blake2::{
	digest::{Update, VariableOutput},
	Blake2bVar,
};
use codec::Encode;
use tokio_postgres::Transaction;

use crate::{
	chains::Chain,
	store::{numeric, timestamp_to_date},
};

/** `getStateId`: a state machine string as the SCALE `StateMachine` the pallet keys by. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateId {
	Evm(u32),
	Polkadot(u32),
	Kusama(u32),
	Substrate(Vec<u8>),
	Tendermint(Vec<u8>),
}

impl StateId {
	/**
	 * Parses `<TYPE>-<value>`, reading only the first two `-` parts as `split("-")` did.
	 * Numbers go through `Number()`, so a non-number is 0 where JavaScript had `NaN` (which
	 * `u32` encoding also turned into 0).
	 */
	pub fn parse(id: &str) -> anyhow::Result<Self> {
		let mut parts = id.split('-');
		let tag = parts.next().unwrap_or_default();
		let value = parts.next().unwrap_or_default();
		let number = || value.parse::<u32>().unwrap_or(0);
		Ok(match tag {
			"EVM" => StateId::Evm(number()),
			"POLKADOT" => StateId::Polkadot(number()),
			"KUSAMA" => StateId::Kusama(number()),
			"SUBSTRATE" => StateId::Substrate(value.as_bytes().to_vec()),
			"TENDERMINT" => StateId::Tendermint(value.as_bytes().to_vec()),
			_ => bail!("Unknown state machine type: {tag}"),
		})
	}

	/** The scale-ts encoding: a variant byte, then a `u32` or the raw id bytes. */
	fn encode(&self) -> Vec<u8> {
		let mut out = vec![];
		match self {
			StateId::Evm(n) => out.extend([0].into_iter().chain(n.to_le_bytes())),
			StateId::Polkadot(n) => out.extend([1].into_iter().chain(n.to_le_bytes())),
			StateId::Kusama(n) => out.extend([2].into_iter().chain(n.to_le_bytes())),
			StateId::Substrate(id) => out.extend([3].into_iter().chain(id.iter().copied())),
			StateId::Tendermint(id) => out.extend([4].into_iter().chain(id.iter().copied())),
		}
		out
	}
}

/** `twox128("Ismp") ++ twox128("BoundedStateCommitments")`. */
const BOUNDED_STATE_COMMITMENTS: [u8; 32] =
	alloy::primitives::hex!("103895530afb23bb607661426d55eb8b1e84813ab41f6335fdb18a4b69508940");

fn blake2_128(data: &[u8]) -> [u8; 16] {
	let mut hasher = Blake2bVar::new(16).expect("16 is a valid blake2b length");
	hasher.update(data);
	let mut out = [0u8; 16];
	hasher.finalize_variable(&mut out).expect("output is 16 bytes");
	out
}

/** The `Ismp.BoundedStateCommitments` key for a state machine id and height. */
pub fn bounded_state_commitment_key(
	state_id: &StateId,
	consensus_state_id: &[u8],
	height: u64,
) -> Vec<u8> {
	let mut id = state_id.encode();
	id.extend_from_slice(consensus_state_id);
	let height = height.encode();
	let mut key = BOUNDED_STATE_COMMITMENTS.to_vec();
	key.extend(blake2_128(&id));
	key.extend(&id);
	key.extend(blake2_128(&height));
	key.extend(&height);
	key
}

/**
 * `fetchStateCommitmentsSubstrate`: the commitment timestamp in seconds, or `None` when the
 * pallet holds no commitment. Read at `block`, where the event wrote it; the TypeScript read
 * the node's head, where old heights have been evicted.
 */
pub async fn substrate_commitment_timestamp(
	chain: &Chain,
	state_machine_id: &str,
	consensus_state_id: &str,
	height: u64,
	block: u64,
) -> anyhow::Result<Option<u64>> {
	let key = bounded_state_commitment_key(
		&StateId::parse(state_machine_id)?,
		consensus_state_id.as_bytes(),
		height,
	);
	let raw = chain.storage(&key, block).await?;
	if raw.is_empty() {
		return Ok(None);
	}
	let timestamp: [u8; 8] = raw
		.get(..8)
		.and_then(|b| b.try_into().ok())
		.ok_or_else(|| anyhow!("StateCommitment is {} bytes", raw.len()))?;
	Ok(Some(u64::from_le_bytes(timestamp)))
}

/**
 * The storage slot `fetchStateCommitmentsEVM` reads the timestamp from. It is the pre-#840
 * `EvmHost` layout; the live host keeps `_stateCommitments` at slot 3, so this reads 0 there.
 * Kept for parity with the rows SubQuery wrote.
 */
const STATE_COMMITMENT_SLOT: u64 = 5;

/** `generateStateCommitmentKeys`: the timestamp slot of `_stateCommitments[paraId][height]`. */
pub fn evm_commitment_slot(para_id: u32, height: u64) -> B256 {
	let mut parent = [0u8; 64];
	parent[..32].copy_from_slice(&U256::from(para_id).to_be_bytes::<32>());
	parent[32..].copy_from_slice(&U256::from(STATE_COMMITMENT_SLOT).to_be_bytes::<32>());
	let parent = keccak256(parent);
	let mut base = [0u8; 64];
	base[..32].copy_from_slice(&U256::from(height).to_be_bytes::<32>());
	base[32..].copy_from_slice(parent.as_slice());
	keccak256(base)
}

/**
 * `fetchStateCommitmentsEVM`: `None` unless the state machine is a Polkadot or Kusama
 * parachain; otherwise the host's storage at `block`, divided by 1000 as the TypeScript did.
 */
pub async fn evm_commitment_timestamp(
	chain: &Chain,
	host: Address,
	state_machine_id: &str,
	height: u64,
	block: u64,
) -> anyhow::Result<Option<U256>> {
	let para_id = match StateId::parse(state_machine_id)? {
		StateId::Polkadot(id) | StateId::Kusama(id) => id,
		_ => return Ok(None),
	};
	let slot = evm_commitment_slot(para_id, height);
	let value = chain
		.provider
		.get_storage_at(host, slot.into())
		.block_id(BlockId::number(block))
		.await?;
	Ok(Some(value / U256::from(1000)))
}

/** One `state_machine_update_events` row. */
pub struct StateMachineUpdate<'a> {
	pub id: String,
	pub state_machine_id: &'a str,
	pub height: u64,
	pub chain: &'a str,
	pub block_hash: &'a str,
	pub block_number: u64,
	pub transaction_hash: &'a str,
	pub transaction_index: u64,
	pub commitment_timestamp: U256,
	/** Seconds or milliseconds. */
	pub timestamp: u64,
}

pub async fn save_state_machine_update(
	db: &Transaction<'_>,
	row: &StateMachineUpdate<'_>,
) -> anyhow::Result<()> {
	let int = |value: u64, what: &str| {
		i32::try_from(value).map_err(|_| anyhow!("{what} {value} does not fit the integer column"))
	};
	db.execute(
		"INSERT INTO app.state_machine_update_events
		   (id, state_machine_id, height, chain, block_hash, block_number, transaction_hash,
		    transaction_index, commitment_timestamp, created_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
		 ON CONFLICT (id) DO UPDATE SET state_machine_id = EXCLUDED.state_machine_id,
		   height = EXCLUDED.height, chain = EXCLUDED.chain, block_hash = EXCLUDED.block_hash,
		   block_number = EXCLUDED.block_number, transaction_hash = EXCLUDED.transaction_hash,
		   transaction_index = EXCLUDED.transaction_index,
		   commitment_timestamp = EXCLUDED.commitment_timestamp, created_at = EXCLUDED.created_at",
		&[
			&row.id,
			&row.state_machine_id,
			&int(row.height, "height")?,
			&row.chain,
			&row.block_hash,
			&int(row.block_number, "block number")?,
			&row.transaction_hash,
			&int(row.transaction_index, "transaction index")?,
			&numeric(row.commitment_timestamp),
			&timestamp_to_date(row.timestamp),
		],
	)
	.await?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_state_ids_like_get_state_id() {
		assert_eq!(StateId::parse("POLKADOT-3367").unwrap(), StateId::Polkadot(3367));
		assert_eq!(StateId::parse("KUSAMA-4009").unwrap(), StateId::Kusama(4009));
		assert_eq!(StateId::parse("EVM-56").unwrap(), StateId::Evm(56));
		assert_eq!(StateId::parse("SUBSTRATE-cere").unwrap(), StateId::Substrate(b"cere".to_vec()));
		assert!(StateId::parse("RELAY-pase-1000").is_err());
	}

	/** The key matches the pallet's own `StorageDoubleMap` with `Blake2_128Concat` hashers. */
	#[test]
	fn bounded_state_commitment_key_matches_the_pallet_layout() {
		use ismp::{consensus::StateMachineId, host::StateMachine};
		let id = StateMachineId { state_id: StateMachine::Evm(56), consensus_state_id: *b"BSC0" };
		let encoded = id.encode();
		let key = bounded_state_commitment_key(&StateId::Evm(56), b"BSC0", 1234);
		let mut expected = BOUNDED_STATE_COMMITMENTS.to_vec();
		expected.extend(blake2_128(&encoded));
		expected.extend(&encoded);
		expected.extend(blake2_128(&1234u64.encode()));
		expected.extend(1234u64.encode());
		assert_eq!(key, expected);
	}

	#[test]
	fn evm_slot_follows_solidity_mapping_layout() {
		// keccak256(pad32(height) ++ keccak256(pad32(paraId) ++ pad32(5)))
		let slot = evm_commitment_slot(3367, 12_597_213);
		let parent = keccak256(
			[U256::from(3367).to_be_bytes::<32>(), U256::from(5).to_be_bytes::<32>()].concat(),
		);
		let expected = keccak256(
			[U256::from(12_597_213).to_be_bytes::<32>().to_vec(), parent.to_vec()].concat(),
		);
		assert_eq!(slot, expected);
	}
}
