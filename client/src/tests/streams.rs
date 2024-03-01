use crate::{
    internals::timeout_request, mock::erc_20::Erc20, streams::query_request_status_stream,
    types::ClientConfig,
};
use anyhow::Context;
use ethers::{
    core::k256::SecretKey,
    prelude::{LocalWallet, Middleware, MiddlewareBuilder, Provider, Signer, Ws, U256},
    providers::{Http, ProviderExt},
    types::H160,
};

use futures::StreamExt;
use hex_literal::hex;
use ismp::host::{Ethereum, StateMachine};
use ismp_solidity_abi::{
    evm_host::EvmHost,
    ping_module::{PingMessage, PingModule},
};
use sp_core::Pair;
use std::{sync::Arc, time::Duration};

const OP_HOST: H160 = H160(hex!("1B58A47e61Ca7604b634CBB00b4e275cCd7c9E95"));
const BSC_HOST: H160 = H160(hex!("022DDE07A21d8c553978b006D93CDe68ac83e677"));
const OP_HANDLER: H160 = H160(hex!("a25151598Dc180fc03635858f37bDF8427f47845"));
const BSC_HANDLER: H160 = H160(hex!("43a0BcC347894303f93905cE137CB3b804bE990d"));

#[tokio::test]
async fn subscribe_to_request_status() -> Result<(), anyhow::Error> {
    let bsc_url = "https://clean-capable-dew.bsc-testnet.quiknode.pro/bed456956996abb801b7ab44fdb3f6f63cd1a4ec";
    let config = ClientConfig {
        source_state_machine: StateMachine::Bsc.to_string(),
        dest_state_machine: StateMachine::Ethereum(Ethereum::Optimism).to_string(),
        hyperbridge_state_machine: StateMachine::Kusama(2000).to_string(),
        source_rpc_url: bsc_url.to_string(),
        dest_rpc_url: "https://opt-sepolia.g.alchemy.com/v2/qzZKMgRJ7zHxeUPoEvjYCmuAsJnx0oVP"
            .to_string(),
        hyper_bridge_url: "ws://127.0.0.1:9990".to_string(),
        destination_ismp_host_address: OP_HOST,
        source_ismp_host_address: BSC_HOST,
        consensus_state_id_source: *b"BSC0",
        consensus_state_id_dest: *b"ETH0",
        destination_ismp_handler: OP_HANDLER,
        source_ismp_handler: BSC_HANDLER,
    };

    // Send Ping Message
    let signer = sp_core::ecdsa::Pair::from_seed_slice(&hex!(
        "6456101e79abe59d2308d63314503446857d4f1f949468bf5627e86e3d6adebd"
    ))
    .unwrap();
    let provider = Arc::new(Provider::<Http>::try_connect(&bsc_url).await?);
    let signer = LocalWallet::from(SecretKey::from_slice(signer.seed().as_slice())?)
        .with_chain_id(provider.get_chainid().await?.low_u64());
    let client = Arc::new(provider.with_signer(signer));
    let ping_addr = H160(hex!("4c1b6031d5BB8A52EF7A13b32852fbE070733FCA"));
    let ping = PingModule::new(ping_addr, client.clone());
    let chain = StateMachine::Bsc;
    let host_addr = ping.host().await.context(format!("Error in {chain:?}"))?;
    dbg!(&host_addr);
    let host = EvmHost::new(host_addr, client.clone());
    let erc_20 =
        Erc20::new(host.dai().await.context(format!("Error in {chain:?}"))?, client.clone());
    let call = erc_20.approve(host_addr, U256::max_value());

    let gas = call.estimate_gas().await.context(format!("Error in {chain:?}"))?;
    call.gas(gas)
        .send()
        .await
        .context(format!("Error in {chain:?}"))?
        .await
        .context(format!("Error in {chain:?}"))?;
    let call = ping.ping(PingMessage {
        dest: config.dest_state_machine.as_bytes().to_vec().into(),
        module: ping_addr.clone().into(),
        timeout: 10 * 60 * 60,
        fee: U256::from(9_000_000_000_000_000_000u128),
        count: U256::from(1),
    });
    let gas = call.estimate_gas().await.context(format!("Error in {chain:?}"))?;
    let receipt = call
        .gas(gas)
        .send()
        .await
        .context(format!("Error in {chain:?}"))?
        .await
        .context(format!("Error in {chain:?}"))?;

    assert!(receipt.is_some());
    let block = receipt.unwrap().block_number.unwrap();
    let events = host
        .events()
        .address(config.source_ismp_host_address.into())
        .from_block(block)
        .to_block(block)
        .query()
        .await?;
    let mut event = events.into_iter().filter_map(|ev| ev.try_into().ok());

    let event = event.find_map(|ev| match ev {
        ismp::events::Event::PostRequest(post) =>
            if post.dest == StateMachine::Ethereum(Ethereum::Optimism) {
                Some(post)
            } else {
                None
            },
        _ => None,
    });

    let post = event.expect("Post request event should be available");

    let mut stream = query_request_status_stream(post, config, block.low_u64()).await;

    while let Some(res) = stream.next().await {
        match res {
            Ok(status) => {
                println!("Got Status {:?}", status);
            },
            Err(e) => {
                println!("{e:?}")
            },
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_timeout_request() -> Result<(), anyhow::Error> {
    let bsc_url = "https://clean-capable-dew.bsc-testnet.quiknode.pro/bed456956996abb801b7ab44fdb3f6f63cd1a4ec";
    let config = ClientConfig {
        source_state_machine: StateMachine::Bsc.to_string(),
        dest_state_machine: StateMachine::Ethereum(Ethereum::Optimism).to_string(),
        hyperbridge_state_machine: StateMachine::Kusama(2000).to_string(),
        source_rpc_url: bsc_url.to_string(),
        dest_rpc_url: "https://opt-sepolia.g.alchemy.com/v2/qzZKMgRJ7zHxeUPoEvjYCmuAsJnx0oVP"
            .to_string(),
        hyper_bridge_url: "ws://127.0.0.1:9990".to_string(),
        destination_ismp_host_address: OP_HOST,
        source_ismp_host_address: BSC_HOST,
        consensus_state_id_source: *b"BSC0",
        consensus_state_id_dest: *b"ETH0",
        destination_ismp_handler: OP_HANDLER,
        source_ismp_handler: BSC_HANDLER,
    };

    // Send Ping Message
    let pair = sp_core::ecdsa::Pair::from_seed_slice(&hex!(
        "6456101e79abe59d2308d63314503446857d4f1f949468bf5627e86e3d6adebd"
    ))
    .unwrap();

    let provider = Arc::new(Provider::<Http>::try_connect(&bsc_url).await?);
    let chain_id = provider.get_chainid().await?.low_u64();
    let signer =
        LocalWallet::from(SecretKey::from_slice(pair.seed().as_slice())?).with_chain_id(chain_id);
    let client = Arc::new(provider.with_signer(signer));
    let ping_addr = H160(hex!("4c1b6031d5BB8A52EF7A13b32852fbE070733FCA"));
    let ping = PingModule::new(ping_addr, client.clone());
    let chain = StateMachine::Bsc;
    let host_addr = ping.host().await.context(format!("Error in {chain:?}"))?;
    dbg!(&host_addr);
    let host = EvmHost::new(host_addr, client.clone());
    let erc_20 =
        Erc20::new(host.dai().await.context(format!("Error in {chain:?}"))?, client.clone());
    let call = erc_20.approve(host_addr, U256::max_value());

    let gas = call.estimate_gas().await.context(format!("Error in {chain:?}"))?;
    call.gas(gas)
        .send()
        .await
        .context(format!("Error in {chain:?}"))?
        .await
        .context(format!("Error in {chain:?}"))?;
    let call = ping.ping(PingMessage {
        dest: config.dest_state_machine.as_bytes().to_vec().into(),
        module: ping_addr.clone().into(),
        timeout: 5 * 60,
        fee: U256::from(0u128),
        count: U256::from(1),
    });
    let gas = call.estimate_gas().await.context(format!("Error in {chain:?}"))?;
    let receipt = call
        .gas(gas)
        .send()
        .await
        .context(format!("Error in {chain:?}"))?
        .await
        .context(format!("Error in {chain:?}"))?;

    assert!(receipt.is_some());
    let block = receipt.unwrap().block_number.unwrap();
    let events = host
        .events()
        .address(config.source_ismp_host_address.into())
        .from_block(block)
        .to_block(block)
        .query()
        .await?;
    let mut event = events.into_iter().filter_map(|ev| ev.try_into().ok());

    let event = event.find_map(|ev| match ev {
        ismp::events::Event::PostRequest(post) =>
            if post.dest == StateMachine::Ethereum(Ethereum::Optimism) {
                Some(post)
            } else {
                None
            },
        _ => None,
    });

    let post = event.expect("Post request event should be available");

    dbg!(&post.timeout_timestamp);
    tokio::time::sleep(Duration::from_secs(12 * 60)).await;
    let message = timeout_request(post, config).await?;

    dbg!(message);
    Ok(())
}
