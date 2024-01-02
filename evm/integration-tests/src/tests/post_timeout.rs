use crate::Keccak256;
use ethers::{
    abi::{AbiEncode, Address, Token, Tokenizable},
    core::types::U256,
};
use forge_testsuite::Runner;
use hex_literal::hex;
use ismp::{
    host::{Ethereum, StateMachine},
    router::{Post, Request},
    util::hash_request,
};
use ismp_solidity_abi::{
    beefy::IntermediateState,
    handler::PostTimeoutMessage,
    shared_types::{PostRequest, StateCommitment, StateMachineHeight},
};
use primitive_types::H256;
use sp_core::KeccakHasher;
use sp_trie::{HashDBT, LayoutV0, MemoryDB, StorageProof, TrieDBBuilder, EMPTY_PREFIX};
use std::{
    collections::{BTreeMap, HashSet},
    env,
    path::PathBuf,
};
use trie_db::{Recorder, Trie, TrieDBMutBuilder, TrieMut};

#[tokio::test(flavor = "multi_thread")]
async fn test_post_timeout_proof() -> Result<(), anyhow::Error> {
    let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
    let mut runner = Runner::new(PathBuf::from(&base_dir));
    let mut contract = runner.deploy("PostTimeoutTest").await;

    let module = contract.call::<_, Address>("module", ()).await?;
    let storage_prefix =
        hex!("103895530afb23bb607661426d55eb8b0484aecefe882c3ce64e6f82507f715a").to_vec();

    // create post request object
    let post = Post {
        source: StateMachine::Ethereum(Ethereum::ExecutionLayer),
        dest: StateMachine::Polkadot(2000),
        nonce: 0,
        from: module.as_bytes().to_vec(),
        to: module.as_bytes().to_vec(),
        timeout_timestamp: 10_000,
        data: storage_prefix.clone(),
        gas_limit: 0,
    };
    let commitment = hash_request::<Keccak256>(&Request::Post(post.clone()));

    let mut key = storage_prefix.clone();
    key.extend_from_slice(commitment.as_ref());

    let entries = (1..50)
        .into_iter()
        .map(|_| {
            let mut key = storage_prefix.clone();
            key.extend_from_slice(&H256::random().0.to_vec());
            (key, H256::random().0.to_vec())
        })
        .collect::<Vec<_>>();

    let (root, proof) = generate_proof(entries.clone(), vec![key.clone()]);

    let storage_proof = StorageProof::new(proof.clone().into_iter());
    let result = read_proof_check(&root, storage_proof, vec![key.as_slice()]).unwrap();

    // The value should be None since it's a None membership proof
    assert!(result.get(&key).unwrap().is_none());

    // create intermediate state
    let height = StateMachineHeight { state_machine_id: U256::from(2000), height: U256::from(1) };
    let consensus_proof = IntermediateState {
        state_machine_id: height.state_machine_id,
        height: height.height,
        commitment: StateCommitment {
            timestamp: U256::from(20_000),
            overlay_root: [0u8; 32],
            state_root: root.0,
        },
    }
    .encode();

    let mut sol_post = PostRequest {
        source: post.source.to_string().as_bytes().to_vec().into(),
        dest: post.dest.to_string().as_bytes().to_vec().into(),
        nonce: post.nonce,
        from: post.from.into(),
        to: post.to.into(),
        timeout_timestamp: post.timeout_timestamp,
        body: post.data.into(),
        gaslimit: post.gas_limit,
    };

    let message = PostTimeoutMessage {
        proof: proof.into_iter().map(|node| node.into()).collect(),
        timeouts: vec![sol_post.clone()],
        height,
    };
    sol_post.timeout_timestamp -= 1;

    // execute the test
    contract
        .call::<_, ()>(
            "PostTimeoutNoChallenge",
            (Token::Bytes(consensus_proof), sol_post.into_token(), message.into_token()),
        )
        .await?;

    Ok(())
}

fn generate_proof(entries: Vec<(Vec<u8>, Vec<u8>)>, keys: Vec<Vec<u8>>) -> (H256, Vec<Vec<u8>>) {
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
