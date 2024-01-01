use crate::{Keccak256, Mmr};
use ethers::{
    abi::{AbiEncode, Address, Token, Tokenizable},
    core::types::U256,
};
use forge_testsuite::Runner;
use ismp::{
    host::{Ethereum, StateMachine},
    mmr::{DataOrHash, Leaf},
    router::{Post, Request},
};
use ismp_solidity_abi::{
    beefy::IntermediateState,
    handler::{PostRequestLeaf, PostRequestMessage, Proof},
    shared_types::{PostRequest, StateCommitment, StateMachineHeight},
};
use merkle_mountain_range::mmr_position_to_k_index;
use primitive_types::H256;
use std::{env, path::PathBuf};

#[tokio::test(flavor = "multi_thread")]
async fn test_post_request_proof() -> Result<(), anyhow::Error> {
    let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
    let mut runner = Runner::new(PathBuf::from(&base_dir));
    let mut contract = runner.deploy("PostRequestTest").await;

    let destination = contract.call::<_, Address>("module", ()).await?;

    // create post request object
    let post = Post {
        source: StateMachine::Polkadot(2000),
        dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
        nonce: 0,
        from: contract.runner.sender.as_bytes().to_vec(),
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
        mmr.push(DataOrHash::Hash(hash))?;
    }

    let pos = mmr.push(request.clone())?;

    for _ in 0..30 {
        let hash = H256::random();
        mmr.push(DataOrHash::Hash(hash))?;
    }

    let k_index = mmr_position_to_k_index(vec![pos], mmr.mmr_size())[0].1;

    let proof = mmr.gen_proof(vec![pos])?;
    let overlay_root = mmr.get_root()?.hash::<Keccak256>().0;
    let multiproof = proof.proof_items().iter().map(|h| h.hash::<Keccak256>().0).collect();

    // create intermediate state
    let height = StateMachineHeight { state_machine_id: U256::from(2000), height: U256::from(1) };
    let consensus_proof = IntermediateState {
        state_machine_id: height.state_machine_id,
        height: height.height,
        commitment: StateCommitment {
            timestamp: U256::from(20000),
            overlay_root,
            state_root: [0u8; 32],
        },
    }
    .encode();

    let message = PostRequestMessage {
        proof: Proof { height, multiproof, leaf_count: (61).into() },
        requests: vec![PostRequestLeaf {
            request: PostRequest {
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
    contract
        .call::<_, ()>(
            "PostRequestNoChallengeNoTimeout",
            (Token::Bytes(consensus_proof), message.into_token()),
        )
        .await?;

    Ok(())
}
