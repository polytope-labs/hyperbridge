#![cfg(test)]

use crate::{
    abi,
    forge::{execute_single, single_runner},
    runner, unwrap_hash, Mmr,
};
use ethers::{
    abi::{AbiEncode, Token, Tokenizable},
    core::types::U256,
};
use foundry_evm::Address;
use ismp::{
    host::{Ethereum, StateMachine},
    router::{Post, Request},
};
use ismp::mmr::{DataOrHash, Leaf};
use merkle_mountain_range_labs::mmr_position_to_k_index;
use primitive_types::H256;

#[tokio::test(flavor = "multi_thread")]
async fn test_post_request_proof() {
    let mut runner = runner();
    let (mut contract, address) = single_runner(&mut runner, "PostRequestTest").await;
    let destination =
        execute_single::<Address, _>(&mut contract, address.clone(), "module", ()).unwrap();

    // create post request object
    let post = Post {
        source: StateMachine::Polkadot(2000),
        dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
        nonce: 0,
        from: contract.sender.as_bytes().to_vec(),
        to: destination.as_bytes().to_vec(),
        timeout_timestamp: 100,

        data: vec![],
        gas_limit: 0,
    };
    let request = DataOrHash::Data(Leaf::Request(Request::Post(post.clone())));

    // create the mmr tree and insert it
    let mut mmr = Mmr::default();

    for _ in 0..30 {
        let hash = H256::random();
        mmr.push(DataOrHash::Hash(hash)).unwrap();
    }

    let pos = mmr.push(request.clone()).unwrap();

    for _ in 0..30 {
        let hash = H256::random();
        mmr.push(DataOrHash::Hash(hash)).unwrap();
    }

    let k_index = mmr_position_to_k_index(vec![pos], mmr.mmr_size())[0].1;

    let proof = mmr.gen_proof(vec![pos]).unwrap();
    let overlay_root = unwrap_hash(&mmr.get_root().unwrap());
    let multiproof = proof.proof_items().iter().map(unwrap_hash).collect();

    // create intermediate state
    let height =
        abi::StateMachineHeight { state_machine_id: U256::from(2000), height: U256::from(1) };
    let consensus_proof = abi::IntermediateState {
        state_machine_id: height.state_machine_id,
        height: height.height,
        commitment: abi::StateCommitment {
            timestamp: U256::from(20000),
            overlay_root,
            state_root: [0u8; 32],
        },
    }
    .encode();

    let message = abi::PostRequestMessage {
        proof: abi::Proof { height, multiproof, leaf_count: (61).into() },
        requests: vec![abi::PostRequestLeaf {
            request: abi::PostRequest {
                source: post.source.to_string().as_bytes().to_vec().into(),
                dest: post.dest.to_string().as_bytes().to_vec().into(),
                nonce: post.nonce,
                from: post.from.into(),
                to: post.to.into(),
                timeout_timestamp: post.timeout_timestamp,
                body: post.data.into(),
                gaslimit: post.gas_limit,
            },
            index: 30.into(),
            k_index: k_index.into(),
        }],
    };

    // execute the test
    execute_single::<(), _>(
        &mut contract,
        address.clone(),
        "PostRequestNoChallengeNoTimeout",
        (Token::Bytes(consensus_proof), message.into_token()),
    )
    .unwrap();
}
