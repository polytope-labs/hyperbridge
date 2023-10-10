#![cfg(test)]

use crate::{
    abi,
    forge::{execute_single, single_runner},
    runner, Keccak256,
};
use ethers::{
    abi::{AbiEncode, Token, Tokenizable},
    core::types::U256,
};
use foundry_evm::Address;
use ismp::{
    host::{Ethereum, StateMachine},
    router::{Get, Request},
    util::hash_request,
};
use primitive_types::H256;
use sp_core::KeccakHasher;
use sp_trie::{LayoutV0, MemoryDB};
use std::collections::HashSet;
use trie_db::{Recorder, Trie, TrieDBBuilder, TrieDBMutBuilder, TrieMut};

fn generate_proof(request: H256, key: Vec<u8>) -> (H256, Vec<Vec<u8>>) {
    let storage_prefix = b":child_storage:default:".to_vec();

    // Populate DB with full trie from entries.
    let (child_db, child_root) = {
        let mut db = <MemoryDB<KeccakHasher>>::default();
        let mut root = Default::default();
        let mut trie = TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut db, &mut root).build();
        trie.insert(&key, H256::random().as_bytes()).unwrap();
        drop(trie);

        (db, root)
    };

    let child_proof = {
        let mut recorder = Recorder::<LayoutV0<KeccakHasher>>::new();
        let trie_db = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&child_db, &child_root)
            .with_recorder(&mut recorder)
            .build();

        // try to get the keys we need from the trie
        let _ = trie_db.get(key.as_ref()).unwrap();

        let proof = recorder.drain().into_iter().map(|f| f.data).collect::<HashSet<_>>();

        proof.into_iter().collect::<Vec<_>>()
    };

    let key = [storage_prefix.clone(), request.as_bytes().to_vec()].concat();

    let entries = (0..10)
        .into_iter()
        .map(|_| {
            let key = [storage_prefix.clone(), H256::random().as_bytes().to_vec()].concat();

            (key, H256::random().as_bytes().to_vec())
        })
        .collect::<Vec<_>>();

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
            trie.insert(key.as_ref(), child_root.as_ref()).unwrap();
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
        let _ = trie_db.get(key.as_ref()).unwrap();

        let proof = recorder.drain().into_iter().map(|f| f.data).collect::<HashSet<_>>();

        proof.into_iter().collect::<Vec<_>>()
    };

    let proof = [child_proof, proof].concat();

    (root, proof)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_response() {
    let mut runner = runner();
    let (mut contract, address) = single_runner(&mut runner, "GetResponseTest").await;
    let destination =
        execute_single::<Address, _>(&mut contract, address.clone(), "module", ()).unwrap();

    let key = H256::random().as_bytes().to_vec();

    // create post request object
    let get = Get {
        dest: StateMachine::Polkadot(2000),
        source: StateMachine::Ethereum(Ethereum::ExecutionLayer),
        nonce: 0,
        from: destination.as_bytes().to_vec(),
        keys: vec![key.clone()],
        timeout_timestamp: 100,
        gas_limit: 0,
        height: 0,
    };

    let request = Request::Get(get.clone());
    let request_commitment = hash_request::<Keccak256>(&request);
    let (root, proof) = generate_proof(request_commitment, key.clone());

    // create intermediate state
    let height =
        abi::StateMachineHeight { state_machine_id: U256::from(2000), height: U256::from(1) };
    let consensus_proof = abi::IntermediateState {
        state_machine_id: height.state_machine_id,
        height: height.height,
        commitment: abi::StateCommitment {
            timestamp: U256::from(20000),
            overlay_root: [0u8; 32],
            state_root: root.0,
        },
    }
    .encode();

    let mut sol_get = abi::GetRequest {
        source: get.source.to_string().as_bytes().to_vec().into(),
        dest: get.dest.to_string().as_bytes().to_vec().into(),
        nonce: get.nonce,
        keys: get.keys.into_iter().map(Into::into).collect(),
        from: get.from.into(),
        timeout_timestamp: get.timeout_timestamp,
        gaslimit: get.gas_limit,
        height: get.height,
    };

    let message = abi::GetResponseMessage {
        proof: proof.into_iter().map(Into::into).collect(),
        height,
        requests: vec![sol_get.clone()],
    };

    sol_get.timeout_timestamp -= 1;

    // execute the test
    execute_single::<(), _>(
        &mut contract,
        address.clone(),
        "GetResponseNoChallengeNoTimeout",
        (Token::Bytes(consensus_proof), sol_get.into_token(), message.into_token()),
    )
    .unwrap();
}
