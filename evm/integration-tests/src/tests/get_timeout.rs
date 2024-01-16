use ethers::abi::{Address, Tokenizable};
use forge_testsuite::Runner;
use ismp::{
    host::{Ethereum, StateMachine},
    router::Get,
};
use ismp_solidity_abi::shared_types::GetRequest;
use primitive_types::H256;
use std::{env, path::PathBuf};

#[tokio::test(flavor = "multi_thread")]
async fn test_get_timeout() -> Result<(), anyhow::Error> {
    let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
    let mut runner = Runner::new(PathBuf::from(&base_dir));
    let mut contract = runner.deploy("GetRequestTest").await;
    let destination = contract.call::<_, Address>("module", ()).await?;

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

    let mut sol_get = GetRequest {
        source: get.source.to_string().as_bytes().to_vec().into(),
        dest: get.dest.to_string().as_bytes().to_vec().into(),
        nonce: get.nonce,
        keys: get.keys.into_iter().map(Into::into).collect(),
        from: get.from.into(),
        timeout_timestamp: get.timeout_timestamp,
        gaslimit: get.gas_limit,
        height: get.height,
    };

    sol_get.timeout_timestamp -= 1;

    // execute the test
    contract.call::<_, ()>("GetTimeoutNoChallenge", (sol_get.into_token(),)).await?;

    Ok(())
}
