// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! State verification functions

use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use codec::Decode;
use core::fmt::Debug;
use hash_db::{HashDB, Hasher, EMPTY_PREFIX};
use sp_core::H256;
use sp_trie::{LayoutV0, StorageProof, Trie, TrieDBBuilder};

#[derive(Debug, derive_more::From, derive_more::Display)]
pub enum Error<H>
where
    H: Hasher,
    H::Out: Debug,
{
    #[display(fmt = "Trie Error: {:?}", _0)]
    Trie(Box<sp_trie::TrieError<LayoutV0<H>>>),
    #[display(fmt = "Error verifying key: {key:?}, Expected: {expected:?}, Got: {got:?}")]
    ValueMismatch { key: Option<String>, expected: Option<Vec<u8>>, got: Option<Vec<u8>> },
    #[display(fmt = "Invalid Proof")]
    InvalidProof,
}

/// Lifted directly from [`sp_state_machine::read_proof_check`](https://github.com/paritytech/substrate/blob/b27c470eaff379f512d1dec052aff5d551ed3b03/primitives/state-machine/src/lib.rs#L1075-L1094)
pub fn read_proof_check<H, I>(
    root: &H::Out,
    proof: StorageProof,
    keys: I,
) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error<H>>
where
    H: Hasher<Out = H256>,
    H::Out: Debug,
    I: IntoIterator,
    I::Item: AsRef<[u8]>,
{
    let db = proof.into_memory_db();

    if !db.contains(root, EMPTY_PREFIX) {
        Err(Error::InvalidProof)?
    }

    let trie = TrieDBBuilder::<LayoutV0<H>>::new(&db, root).build();
    let mut result = BTreeMap::new();

    for key in keys.into_iter() {
        let value = trie.get(key.as_ref())?.and_then(|val| Decode::decode(&mut &val[..]).ok());
        result.insert(key.as_ref().to_vec(), value);
    }

    Ok(result)
}
