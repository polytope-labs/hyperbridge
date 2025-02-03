use polkadot_sdk::*;
use std::collections::{BTreeMap, HashSet};

use crate::{DataOrHash, Mmr};
use ismp_solidity_abi::{beefy::StateMachineHeight, handler::Proof};
use primitive_types::{H256, U256};
use sp_core::KeccakHasher;
use sp_trie::{LayoutV0, MemoryDB, StorageProof, TrieDBBuilder, EMPTY_PREFIX};
use trie_db::{HashDB, Recorder, Trie, TrieDBMutBuilder, TrieMut};

/// Initialize an MMR tree, inserting the given leaf into it and returning the root of the tree, the
/// proof and the k-index of the leaf
pub fn initialize_mmr_tree(
	leaf: DataOrHash,
	block_height: u64,
) -> Result<([u8; 32], Proof, usize), anyhow::Error> {
	// create the mmr tree and insert it
	let mut mmr = Mmr::default();

	for _ in 0..30 {
		let hash = H256::random();
		mmr.push(DataOrHash::Hash(hash))?;
	}

	let pos = mmr.push(leaf.clone())?;

	for _ in 0..30 {
		let hash = H256::random();
		mmr.push(DataOrHash::Hash(hash))?;
	}

	let k_index = mmr_primitives::mmr_position_to_k_index(vec![pos], mmr.mmr_size())[0].1;
	let proof = mmr.gen_proof(vec![pos])?;
	let root = mmr.get_root()?.hash().0;
	let multiproof = proof.proof_items().iter().map(|h| h.hash().0).collect();
	let height =
		StateMachineHeight { state_machine_id: U256::from(2000), height: U256::from(block_height) };
	let proof = Proof { height, multiproof, leaf_count: (61).into() };

	Ok(((root, proof, k_index)))
}

/// Initialize a state trie
pub fn generate_non_membership_proof(
	prefix: Vec<u8>,
	keys: Vec<Vec<u8>>,
	insert_keys: bool,
) -> (H256, Vec<Vec<u8>>) {
	let mut entries = (1..50)
		.into_iter()
		.map(|_| {
			let mut key = prefix.clone();
			key.extend_from_slice(&H256::random().0.to_vec());
			(key, H256::random().0.to_vec())
		})
		.collect::<Vec<_>>();

	if insert_keys {
		let extension = keys
			.clone()
			.into_iter()
			.map(|key| (key, H256::random().0.to_vec()))
			.collect::<Vec<_>>();

		entries.extend_from_slice(&extension);
	}

	// Populate DB with full trie from entries.
	let (db, root) = {
		let mut db = <MemoryDB<KeccakHasher>>::default();
		let mut root = Default::default();
		{
			let mut trie =
				TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut db, &mut root).build();
			for (key, value) in &entries {
				trie.insert(key, &value).unwrap();
			}
		}
		(db, root)
	};

	// Generate proof for the given keys..
	let proof = {
		let mut recorder = Recorder::<LayoutV0<KeccakHasher>>::new();
		let trie_db = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&db, &root)
			.with_recorder(&mut recorder)
			.build();

		// try to get the keys we need from the trie
		for key in &keys {
			let _ = trie_db.get(key).unwrap();
		}

		let proof = recorder.drain().into_iter().map(|f| f.data).collect::<HashSet<_>>();

		proof.into_iter().collect::<Vec<_>>()
	};

	(root, proof)
}

pub fn read_proof_check<I>(
	root: &H256,
	proof: StorageProof,
	keys: I,
) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, ()>
where
	I: IntoIterator,
	I::Item: AsRef<[u8]>,
{
	let db = proof.into_memory_db::<KeccakHasher>();

	if !db.contains(root, EMPTY_PREFIX) {
		Err(())?
	}

	let trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&db, root).build();
	let mut result = BTreeMap::new();

	for key in keys.into_iter() {
		let value = trie.get(key.as_ref()).map_err(|_| ())?;
		result.insert(key.as_ref().to_vec(), value);
	}

	Ok(result)
}
