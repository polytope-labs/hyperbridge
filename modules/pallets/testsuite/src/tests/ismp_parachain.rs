#![cfg(test)]

use codec::Encode;
use polkadot_sdk::*;

use ismp::{
	consensus::StateMachineId,
	host::{IsmpHost, StateMachine},
};
use ismp_parachain::{
	consensus::{
		parachain_header_storage_key, ParachainConsensusProof, ASSET_HUB_PARA_ID,
		PARACHAIN_CONSENSUS_ID, PASEO_CONSENSUS_ID, PASSET_HUB_TESTNET_CHAIN_ID,
	},
	CurrentRelayChainStateRoots, Parachains, SlotDurations,
};
use pallet_ismp::{TimestampDigest, ISMP_TIMESTAMP_ID};
use sp_consensus_aura::{Slot, AURA_ENGINE_ID};
use sp_core::H256;
use sp_runtime::{generic::Header, traits::BlakeTwo256, Digest, DigestItem};
use sp_state_machine::{prove_read, TrieBackendBuilder};
use sp_trie::{trie_types::TrieDBMutBuilderV0, PrefixedMemoryDB};
use trie_db::TrieMut;

use crate::runtime::{new_test_ext, Ismp, Test, Timestamp};

/// Polkadot Hub's aura slot duration in milliseconds.
const SLOT_DURATION: u64 = 24_000;
/// A slot from Polkadot Hub, which puts the block time at `SLOT * SLOT_DURATION`.
const SLOT: u64 = 74_456_947;
const RELAY_HEIGHT: u32 = 42;
const PARA_HEIGHT: u32 = 19_567_971;

fn aura_digest(slot: u64) -> DigestItem {
	DigestItem::PreRuntime(AURA_ENGINE_ID, Slot::from(slot).encode())
}

fn timestamp_digest(timestamp: u64) -> DigestItem {
	DigestItem::Consensus(ISMP_TIMESTAMP_ID, TimestampDigest { timestamp }.encode())
}

fn parachain_header(logs: Vec<DigestItem>) -> Header<u32, BlakeTwo256> {
	Header {
		parent_hash: H256::zero(),
		number: PARA_HEIGHT,
		state_root: H256::repeat_byte(1),
		extrinsics_root: H256::zero(),
		digest: Digest { logs },
	}
}

/// Puts the header where the relay chain keeps it and proves it, the way a relayer would.
fn relay_chain_proof(para_id: u32, header: &Header<u32, BlakeTwo256>) -> (H256, Vec<Vec<u8>>) {
	let key = parachain_header_storage_key(para_id).0;

	let mut db = PrefixedMemoryDB::<BlakeTwo256>::default();
	let mut root = H256::default();
	{
		let mut trie = TrieDBMutBuilderV0::new(&mut db, &mut root).build();
		trie.insert(&key, &header.encode().encode()).unwrap();
	}

	let backend = TrieBackendBuilder::new(db, root).build();
	let proof = prove_read(backend, vec![&key[..]]).unwrap();

	(root, proof.into_nodes().into_iter().collect())
}

/// Registers the parachain, seeds the relay chain state root the proof is checked against and
/// returns the encoded consensus proof.
fn setup(para_id: u32, header: &Header<u32, BlakeTwo256>) -> Vec<u8> {
	let (root, storage_proof) = relay_chain_proof(para_id, header);

	Parachains::<Test>::insert(para_id, ());
	CurrentRelayChainStateRoots::<Test>::insert(RELAY_HEIGHT, root);

	ParachainConsensusProof { relay_height: RELAY_HEIGHT, storage_proof }.encode()
}

fn verify(proof: Vec<u8>) -> Result<Vec<(StateMachineId, u64)>, ismp::error::Error> {
	let host = Ismp::default();
	let client = host.consensus_client(PARACHAIN_CONSENSUS_ID)?;
	let (_, intermediates) = client.verify_consensus(&host, PASEO_CONSENSUS_ID, vec![], proof)?;

	Ok(intermediates
		.into_iter()
		.flat_map(|(id, heights)| {
			heights.into_iter().map(move |height| (id, height.commitment.timestamp))
		})
		.collect())
}

#[test]
fn asset_hub_reads_its_timestamp_from_the_aura_slot() {
	new_test_ext().execute_with(|| {
		Timestamp::set_timestamp(SLOT * SLOT_DURATION + 6_000);
		SlotDurations::<Test>::insert(ASSET_HUB_PARA_ID, SLOT_DURATION);

		let header = parachain_header(vec![aura_digest(SLOT)]);
		let commitments = verify(setup(ASSET_HUB_PARA_ID, &header)).unwrap();

		assert_eq!(
			commitments,
			vec![(
				StateMachineId {
					state_id: StateMachine::Evm(PASSET_HUB_TESTNET_CHAIN_ID),
					consensus_state_id: PASEO_CONSENSUS_ID,
				},
				SLOT * SLOT_DURATION / 1_000,
			)]
		);
	})
}

#[test]
fn the_timestamp_digest_wins_when_both_are_present() {
	new_test_ext().execute_with(|| {
		let timestamp = SLOT * SLOT_DURATION / 1_000 + 12;
		Timestamp::set_timestamp(timestamp * 1_000);
		SlotDurations::<Test>::insert(ASSET_HUB_PARA_ID, SLOT_DURATION);

		let header = parachain_header(vec![aura_digest(SLOT), timestamp_digest(timestamp)]);
		let commitments = verify(setup(ASSET_HUB_PARA_ID, &header)).unwrap();

		assert_eq!(commitments.first().map(|(_, timestamp)| *timestamp), Some(timestamp));
	})
}

#[test]
fn other_parachains_are_not_allowed_the_aura_fallback() {
	new_test_ext().execute_with(|| {
		let para_id = 2000;
		Timestamp::set_timestamp(SLOT * SLOT_DURATION + 6_000);
		SlotDurations::<Test>::insert(para_id, SLOT_DURATION);

		let header = parachain_header(vec![aura_digest(SLOT)]);
		let error = verify(setup(para_id, &header)).unwrap_err();

		assert!(format!("{error:?}").contains("Timestamp not found"), "{error:?}");
	})
}

#[test]
fn a_slot_duration_that_outruns_the_local_clock_is_rejected() {
	new_test_ext().execute_with(|| {
		Timestamp::set_timestamp(SLOT * SLOT_DURATION + 6_000);
		SlotDurations::<Test>::insert(ASSET_HUB_PARA_ID, SLOT_DURATION * 2);

		let header = parachain_header(vec![aura_digest(SLOT)]);
		let error = verify(setup(ASSET_HUB_PARA_ID, &header)).unwrap_err();

		assert!(format!("{error:?}").contains("is in the future"), "{error:?}");
	})
}
