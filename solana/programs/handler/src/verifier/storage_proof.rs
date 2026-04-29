//! Substrate global-trie storage proof verifier (Blake2b256).

extern crate alloc;
use alloc::vec::Vec;

use anchor_lang::prelude::*;
use polkadot_sdk::*;
use sp_core::{Blake2Hasher, H256};
use sp_trie::{LayoutV0, StorageProof, Trie, TrieDBBuilder};

use crate::error::HandlerError;

/// `Ok(Some(value))` — present. `Ok(None)` — provably absent.
/// `Err(_)` — proof malformed or doesn't hash to `state_root`.
pub fn verify_substrate_storage_proof(
	state_root: &[u8; 32],
	key: &[u8],
	proof_nodes: Vec<Vec<u8>>,
) -> Result<Option<Vec<u8>>> {
	let root: H256 = (*state_root).into();
	let db = StorageProof::new(proof_nodes).into_memory_db::<Blake2Hasher>();
	let trie = TrieDBBuilder::<LayoutV0<Blake2Hasher>>::new(&db, &root).build();
	trie.get(key).map_err(|_| error!(HandlerError::InvalidStorageProof))
}
