use ethers::abi::{AbiEncode, Tokenizable};
use forge_testsuite::Runner;
use foundry_evm::executor::EvmError;
use ismp::{
    host::{Ethereum, StateMachine},
    router::Post,
};
use ismp_solidity_abi::{evm_host::HostParams, shared_types::PostRequest};
use pallet_ismp_relayer::withdrawal::WithdrawalParams;
use primitive_types::{H160, U256};
use std::{env, path::PathBuf};

#[tokio::test(flavor = "multi_thread")]
async fn test_host_manager_withdraw() -> Result<(), anyhow::Error> {
    let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
    let mut runner = Runner::new(PathBuf::from(&base_dir));
    let mut contract = runner.deploy("HostManagerTest").await;

    let params = WithdrawalParams {
        beneficiary_address: H160::random().as_bytes().to_vec(),
        amount: U256::from(500_000_000_000_000_000_000u128),
    };
    let data = params.abi_encode();

    // create post request object
    let post = Post {
        source: StateMachine::Kusama(2000),
        dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
        nonce: 0,
        from: contract.runner.sender.as_bytes().to_vec(),
        to: vec![],
        timeout_timestamp: 100,
        data,
        gas_limit: 0,
    };

    let request: PostRequest = post.into();

    // execute the test
    contract.call::<_, ()>("HostManagerWithdraw", (request.into_token(),)).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_host_manager_unauthorized_request() -> Result<(), anyhow::Error> {
    let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
    let mut runner = Runner::new(PathBuf::from(&base_dir));
    let mut contract = runner.deploy("HostManagerTest").await;

    let params = WithdrawalParams {
        beneficiary_address: H160::random().as_bytes().to_vec(),
        amount: U256::from(500_000_000_000_000_000_000u128),
    };
    let data = params.abi_encode();

    // create post request object
    let post = Post {
        // wrong source
        source: StateMachine::Polkadot(2000),
        dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
        nonce: 0,
        from: contract.runner.sender.as_bytes().to_vec(),
        to: vec![],
        timeout_timestamp: 100,
        data,
        gas_limit: 0,
    };

    let request: PostRequest = post.into();

    // execute the test
    let EvmError::Execution(error) = contract
        .call::<_, ()>("HostManagerUnauthorizedRequest", (request.into_token(),))
        .await
        .unwrap_err()
    else {
        panic!("Call should revert")
    };

    assert_eq!(error.reason.as_str(), "Unauthorized request");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_host_manager_insufficient_balance() -> Result<(), anyhow::Error> {
    let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
    let mut runner = Runner::new(PathBuf::from(&base_dir));
    let mut contract = runner.deploy("HostManagerTest").await;

    let params = WithdrawalParams {
        beneficiary_address: H160::random().as_bytes().to_vec(),
        amount: U256::from(500_000_000_000_000_000_000u128),
    };
    let data = params.abi_encode();

    // create post request object
    let post = Post {
        // wrong source
        source: StateMachine::Kusama(2000),
        dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
        nonce: 0,
        from: contract.runner.sender.as_bytes().to_vec(),
        to: vec![],
        timeout_timestamp: 100,
        data,
        gas_limit: 0,
    };

    let request: PostRequest = post.into();

    // execute the test
    let EvmError::Execution(error) = contract
        .call::<_, ()>("HostManagerUnauthorizedRequest", (request.into_token(),))
        .await
        .unwrap_err()
    else {
        panic!("Call should revert")
    };

    assert_eq!(error.reason.as_str(), "ERC20: transfer amount exceeds balance");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_host_manager_set_host_params() -> Result<(), anyhow::Error> {
    let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
    let mut runner = Runner::new(PathBuf::from(&base_dir));
    let mut contract = runner.deploy("HostManagerTest").await;

    let params = HostParams {
        default_timeout: Default::default(),
        base_get_request_fee: Default::default(),
        per_byte_fee: Default::default(),
        fee_token_address: Default::default(),
        admin: Default::default(),
        handler: Default::default(),
        host_manager: Default::default(),
        un_staking_period: Default::default(),
        challenge_period: U256::from(5_000_000u128),
        consensus_client: Default::default(),
        consensus_state: Default::default(),
        last_updated: Default::default(),
        latest_state_machine_height: Default::default(),
    };
    let mut data = vec![1u8];
    data.extend_from_slice(params.encode().as_slice());

    // create post request object
    let post = Post {
        source: StateMachine::Kusama(2000),
        dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
        nonce: 0,
        from: contract.runner.sender.as_bytes().to_vec(),
        to: vec![],
        timeout_timestamp: 100,
        data,
        gas_limit: 0,
    };

    let request: PostRequest = post.into();

    // execute the test
    contract.call::<_, ()>("HostManagerSetParams", (request.into_token(),)).await?;

    Ok(())
}
