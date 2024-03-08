use crate::{
    internals::timeout_request_stream, mock::erc_20::Erc20, streams::query_request_status_stream,
    types::ClientConfig,
};
use anyhow::Context;
use ethers::{
    core::k256::SecretKey,
    prelude::{LocalWallet, Middleware, MiddlewareBuilder, Provider, Signer, U256},
    providers::{Http, ProviderExt},
    types::H160,
};

use crate::{
    internals::query_request_status_internal,
    types::{ChainConfig, EvmConfig, HashAlgorithm, MessageStatus, SubstrateConfig, TimeoutStatus},
};
use ethers::{
    prelude::{transaction::eip2718::TypedTransaction, NameOrAddress, TransactionRequest},
    utils::hex,
};
use frame_support::crypto::ecdsa::ECDSAExt;
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
#[ignore]
async fn subscribe_to_request_status() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();
    let signing_key = std::env::var("SIGNING_KEY").unwrap();
    let bsc_url = std::env::var("BSC_URL").unwrap();
    let op_url = std::env::var("OP_URL").unwrap();
    let source_chain = EvmConfig {
        rpc_url: bsc_url.clone(),
        state_machine: StateMachine::Bsc,
        host_address: BSC_HOST,
        handler_address: BSC_HANDLER,
        consensus_state_id: *b"BSC0",
    };

    let dest_chain = EvmConfig {
        rpc_url: op_url,
        state_machine: StateMachine::Ethereum(Ethereum::Optimism),
        host_address: OP_HOST,
        handler_address: OP_HANDLER,
        consensus_state_id: *b"ETH0",
    };

    let hyperbrige_config = SubstrateConfig {
        rpc_url: "ws://127.0.0.1:9990".to_string(),
        consensus_state_id: *b"PARA",
        hash_algo: HashAlgorithm::Keccak,
    };
    let config = ClientConfig {
        source: ChainConfig::Evm(source_chain.clone()),
        dest: ChainConfig::Evm(dest_chain.clone()),
        hyperbridge: ChainConfig::Substrate(hyperbrige_config),
    };

    // Send Ping Message
    let signer = sp_core::ecdsa::Pair::from_seed_slice(&hex::decode(signing_key).unwrap()).unwrap();
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
        dest: dest_chain.state_machine.to_string().as_bytes().to_vec().into(),
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
        .address(source_chain.host_address.into())
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
    let source_client = config.source_chain().await?;
    let dest_client = config.dest_chain().await?;
    let hyperbridge_client = config.hyperbridge_client().await?;
    let mut stream = query_request_status_stream(
        post,
        source_client,
        dest_client,
        hyperbridge_client,
        block.low_u64(),
    )
    .await;

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
#[ignore]
async fn test_timeout_request() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();
    let signing_key = std::env::var("SIGNING_KEY").unwrap();
    let bsc_url = std::env::var("BSC_URL").unwrap();
    let op_url = std::env::var("OP_URL").unwrap();
    let source_chain = EvmConfig {
        rpc_url: bsc_url.clone(),
        state_machine: StateMachine::Bsc,
        host_address: BSC_HOST,
        handler_address: BSC_HANDLER,
        consensus_state_id: *b"BSC0",
    };

    let dest_chain = EvmConfig {
        rpc_url: op_url,
        state_machine: StateMachine::Ethereum(Ethereum::Optimism),
        host_address: OP_HOST,
        handler_address: OP_HANDLER,
        consensus_state_id: *b"ETH0",
    };

    let hyperbrige_config = SubstrateConfig {
        rpc_url: "ws://127.0.0.1:9990".to_string(),
        consensus_state_id: *b"PARA",
        hash_algo: HashAlgorithm::Keccak,
    };
    let config = ClientConfig {
        source: ChainConfig::Evm(source_chain.clone()),
        dest: ChainConfig::Evm(dest_chain.clone()),
        hyperbridge: ChainConfig::Substrate(hyperbrige_config),
    };

    // Send Ping Message
    let pair = sp_core::ecdsa::Pair::from_seed_slice(&hex::decode(signing_key).unwrap()).unwrap();

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
        dest: dest_chain.state_machine.to_string().as_bytes().to_vec().into(),
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
        .address(source_chain.host_address.into())
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
    loop {
        let status = query_request_status_internal(post.clone(), config.clone()).await?;
        if status == MessageStatus::Timeout {
            break
        } else {
            println!("{status:?}");
            tokio::time::sleep(Duration::from_secs(2 * 60)).await;
        }
    }

    let mut stream = timeout_request_stream(post, config).await?;

    while let Some(res) = stream.next().await {
        match res {
            Ok(status) => {
                println!("Got Status {:?}", status);
                match status {
                    TimeoutStatus::TimeoutMessage(call_data) => {
                        let gas_price = client.get_gas_price().await?;
                        println!("Sending timeout to BSC");
                        let receipt = client
                            .clone()
                            .send_transaction(
                                TypedTransaction::Legacy(TransactionRequest {
                                    from: Some(H160::from(pair.public().to_eth_address().unwrap())),
                                    to: Some(NameOrAddress::Address(source_chain.handler_address)),
                                    value: Some(Default::default()),
                                    gas_price: Some(gas_price * 5), // experiment with higher?
                                    data: Some(call_data.into()),
                                    ..Default::default()
                                }),
                                None,
                            )
                            .await?
                            .await?;
                        dbg!(receipt.unwrap().transaction_hash);
                    },
                    _ => {},
                }
            },
            Err(e) => {
                println!("{e:?}")
            },
        }
    }
    Ok(())
}
