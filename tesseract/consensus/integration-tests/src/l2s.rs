use std::sync::Arc;
use codec::Encode;

use hex_literal::hex;
use polkadot_sdk::sp_runtime::MultiSigner;
use polkadot_sdk::sp_runtime::traits::IdentifyAccount;
use sp_core::{H160, Pair};
use subxt::tx::TxPayload;
use arb_host::{ArbConfig, ArbHost};
use ismp::consensus::StateMachineId;

use ismp::host::StateMachine;
use ismp::messaging::CreateConsensusState;
use op_host::OpHost;
use substrate_state_machine::HashAlgorithm;
use subxt_utils::{Extrinsic, Hyperbridge, InMemorySigner, send_extrinsic};
use sync_committee_primitives::constants::ETH1_DATA_VOTES_BOUND_ETH;
use tesseract_beefy::host::BeefyHost;
use tesseract_evm::EvmConfig;
use tesseract_grandpa::{GrandpaConfig, GrandpaHost};
use tesseract_primitives::IsmpHost;
use tesseract_substrate::{SubstrateClient, SubstrateConfig};
use tesseract_substrate::config::Blake2SubstrateChain;
use tesseract_sync_committee::SyncCommitteeHost;
use crate::util::setup_logging;

async fn setup_clients() -> Result<
    (
        GrandpaHost<Blake2SubstrateChain, Hyperbridge>,
        SyncCommitteeHost::<
            sync_committee_primitives::constants::sepolia::Sepolia,
            ETH1_DATA_VOTES_BOUND_ETH,
        >,
        ArbHost,
        OpHost
    ),
    anyhow::Error,
> {
    let beacon_url = env!("BEACON_URL").to_string();
    let arb_url = env!("ARB_URL").to_string();
    let op_url = env!("OP_URL").to_string();

    let config_a = SubstrateConfig {
        state_machine: StateMachine::Kusama(2000),
        hashing: Some(HashAlgorithm::Keccak),
        consensus_state_id: Some("PARA".to_string()),
        rpc_ws: "ws://localhost:9944".to_string(),
        max_rpc_payload_size: None,
        signer: Some(
            "0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
        ),
        initial_height: None,
        max_concurent_queries: None,
        poll_interval: None,
        fee_token_decimals: None,
    };

    let host = tesseract_grandpa::HostConfig {
        rpc: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
        slot_duration: 12,
        consensus_update_frequency: Some(60),
        para_ids: vec![],
        max_block_range: None,
    };

    let hyperbridge_grandpa_config = GrandpaConfig {
        substrate: config_a,
        grandpa: host,
    };

    let hyperbridge_chain = GrandpaHost::<Blake2SubstrateChain, Hyperbridge>::new(
        &hyperbridge_grandpa_config
    )
        .await?;

    let sync_committee_chain = {
        let config = EvmConfig {
            rpc_urls: vec![
                beacon_url.clone()
            ],
            state_machine: StateMachine::Evm(11155111),
            consensus_state_id: "ETH0".to_string(),
            ismp_host: hex!("7BdE4Ce065400eE332C20f7df3a35d66674165f6").into(),
            signer: "6284acbdef4b15b21b64d9fbdcb7c7d4fa05f1a96364d12c2988bddc18356d84".to_string(),
            ..Default::default()
        };

        let sync_commitee_config = tesseract_sync_committee::HostConfig {
            beacon_http_urls: vec![beacon_url.clone()],
            consensus_update_frequency: 60,
        };


        SyncCommitteeHost::<
            sync_committee_primitives::constants::sepolia::Sepolia,
            ETH1_DATA_VOTES_BOUND_ETH,
        >::new(&sync_commitee_config, &config, Default::default())
            .await?

    };
    let sync_committee_initial_consensus_state_message_for_other_chains = sync_committee_chain.query_initial_consensus_state().await?.unwrap();

    let arbitrum_chain = {
        let evm_config = EvmConfig {
            rpc_urls: vec![
                arb_url
            ],
            state_machine: StateMachine::Evm(421614),
            consensus_state_id: "ARB0".to_string(),
            ismp_host: hex!("3435bD7e5895356535459D6087D1eB982DAd90e7").into(),
            signer: "6284acbdef4b15b21b64d9fbdcb7c7d4fa05f1a96364d12c2988bddc18356d84".to_string(),
            gas_price_buffer: Some(8),
            ..Default::default()
        };

        let host =  arb_host::HostConfig {
            beacon_rpc_url: vec![
                beacon_url.clone()
            ],
            rollup_core: H160::from(hex!("042B2E6C5E99d4c521bd49beeD5E99651D9B0Cf4")),
            l1_state_machine: StateMachine::Evm(11155111),
            l1_consensus_state_id: "ETH0".to_string(),
            consensus_update_frequency: None,
        };

        ArbHost::new(&host, &evm_config)
            .await?

    };
    let arbirtum_initial_consensus_state_message_for_other_chains = arbitrum_chain.query_initial_consensus_state().await?.unwrap();
    let arbitrum_state_machine_id = StateMachineId {
        state_id: StateMachine::Evm(421614),
        consensus_state_id: *b"ARB0",
    };
    set_arbitrum_config_on_hyperbridge(hyperbridge_chain.clone(), arbitrum_state_machine_id, arbitrum_chain.host.rollup_core).await?;

    let optimism_chain = {
        let evm_config = EvmConfig {
            rpc_urls: vec![
                op_url
            ],
            state_machine: StateMachine::Evm(11155420),
            consensus_state_id: "OPT0".to_string(),
            ismp_host: hex!("6d51b678836d8060d980605d2999eF211809f3C2").into(),
            signer: "6284acbdef4b15b21b64d9fbdcb7c7d4fa05f1a96364d12c2988bddc18356d84".to_string(),
            gas_price_buffer: Some(5),
            ..Default::default()
        };

        let host =  op_host::HostConfig {
            beacon_rpc_url: vec![
                beacon_url
            ],
            l1_state_machine: StateMachine::Evm(11155111),
            l1_consensus_state_id: "ETH0".to_string(),
            consensus_update_frequency: None,

            l2_oracle: None,
            message_parser: H160::from(hex!("4200000000000000000000000000000000000016")),
            dispute_game_factory: Some(H160::from(hex!("05F9613aDB30026FFd634f38e5C4dFd30a197Fa1"))),
            proposer_config: None,
        };

        OpHost::new(&host, &evm_config)
            .await?

    };
    let optimism_state_machine_id = StateMachineId {
        state_id: StateMachine::Evm(11155420),
        consensus_state_id: *b"OPT0",
    };

    set_optimism_config_on_hyperbridge(hyperbridge_chain.clone(), optimism_state_machine_id, optimism_chain.host.dispute_game_factory.unwrap(), vec![0, 1]).await?;


    let optimism_initial_consensus_state_message_for_other_chains = optimism_chain.query_initial_consensus_state().await?.unwrap();


    log::info!("🧊 Setting consensus states");
    hyperbridge_chain.provider().set_initial_consensus_state(sync_committee_initial_consensus_state_message_for_other_chains).await?;
    hyperbridge_chain.provider().set_initial_consensus_state(arbirtum_initial_consensus_state_message_for_other_chains).await?;
    hyperbridge_chain.provider().set_initial_consensus_state(optimism_initial_consensus_state_message_for_other_chains).await?;

    Ok((hyperbridge_chain, sync_committee_chain, arbitrum_chain, optimism_chain))
}

pub async fn set_arbitrum_config_on_hyperbridge(
    hyperbridge_chain: GrandpaHost<Blake2SubstrateChain, Hyperbridge>,
    state_machine_id: StateMachineId,
    rollup_core_address: H160,
) -> Result<(), anyhow::Error> {
    let client = hyperbridge_chain.substrate_client;
    let signer = InMemorySigner {
        account_id: MultiSigner::Sr25519(client.signer.public()).into_account().into(),
        signer: client.signer.clone(),
    };

    let message = (state_machine_id, rollup_core_address);

    let call = message.encode();
    let call = Extrinsic::new("IsmpArbitrum", "set_rollup_core_address", call)
        .encode_call_data(&client.client.metadata())?;
    let tx = Extrinsic::new("Sudo", "sudo", call);
    send_extrinsic(&client.client, signer, tx, None).await?;

    Ok(())
}

pub async fn set_optimism_config_on_hyperbridge(
    hyperbridge_chain: GrandpaHost<Blake2SubstrateChain, Hyperbridge>,
    state_machine_id: StateMachineId,
    dispute_game_factory: H160,
    respected_game_types: Vec<u32>,
) -> Result<(), anyhow::Error> {
    let client = hyperbridge_chain.substrate_client;
    let signer = InMemorySigner {
        account_id: MultiSigner::Sr25519(client.signer.public()).into_account().into(),
        signer: client.signer.clone(),
    };

    let message = (state_machine_id, dispute_game_factory, respected_game_types);

    let call = message.encode();
    let call = Extrinsic::new("IsmpOptimism", "set_dispute_game_factories", call)
        .encode_call_data(&client.client.metadata())?;
    let tx = Extrinsic::new("Sudo", "sudo", call);
    send_extrinsic(&client.client, signer, tx, None).await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_consensus_messaging_relay() -> Result<(), anyhow::Error> {
    setup_logging();

    log::info!("🧊 Initializing tesseract consensus");

    let (hyperbridge_chain, sync_committee_chain, arbitrum_chain, optimism_chain) = setup_clients().await?;

    let handle_a = tokio::spawn({
        let hyperbridge_chain = hyperbridge_chain.clone();
        let sync_committee_chain = sync_committee_chain.clone();
        async move { sync_committee_chain.start_consensus(hyperbridge_chain.provider()).await.unwrap() }
    });

    let handle_b = tokio::spawn({
        let hyperbridge_chain = hyperbridge_chain.clone();
        let arbitrum_chain = arbitrum_chain.clone();
        async move { arbitrum_chain.start_consensus(hyperbridge_chain.provider()).await.unwrap() }
    });


    let handle_c = tokio::spawn({
        let hyperbridge_chain = hyperbridge_chain.clone();
        let optimism_chain = optimism_chain.clone();
        async move { optimism_chain.start_consensus(hyperbridge_chain.provider()).await.unwrap() }
    });

    log::info!("🧊 Initialized consensus tasks");

    let _ = tokio::join!(handle_a, handle_b, handle_c);

    Ok(())
}
