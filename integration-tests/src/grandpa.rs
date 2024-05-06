#![cfg(test)]

use crate::{
    transfer_assets,
    util::{setup_logging, Hyperbridge},
};
use codec::Encode;
use futures::StreamExt;
use ismp::{host::StateMachine, messaging::CreateConsensusState};
use ismp_grandpa::consensus::{
    GRANDPA_CONSENSUS_ID, KUSAMA_CONSENSUS_STATE_ID, POLKADOT_CONSENSUS_STATE_ID,
};
use ismp::HashAlgorithm;
use tesseract_grandpa::{GrandpaConfig, GrandpaHost, GrandpaProverConfig};
use tesseract_substrate::{SubstrateClient, SubstrateConfig};
type GrandpaClient<T> = SubstrateClient<GrandpaHost<T>, T>;

async fn setup_clients(
) -> Result<(GrandpaClient<Hyperbridge>, GrandpaClient<Hyperbridge>), anyhow::Error> {
    let config_a = GrandpaConfig {
        chain: "ws://localhost:9944".to_string(),
        state_machine: StateMachine::Kusama(0),
        consensus_state_id: POLKADOT_CONSENSUS_STATE_ID,
        substrate: SubstrateConfig {
            state_machine: StateMachine::Kusama(2000),
            hashing: HashAlgorithm::Blake2,
            consensus_state_id: "polk".to_string(),
            ws_url: "ws://localhost:9988".to_string(),
            signer: "0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a"
                .to_string(),
            latest_height: None,
        },
        grandpa_prover_config: GrandpaProverConfig {
            para_ids: vec![2000],
            babe_epoch_start_key: None,
            current_set_id: None,
        },
    };
    let host_a = GrandpaHost::<Hyperbridge>::new(&config_a).await?;
    let chain_a = SubstrateClient::new(host_a.clone(), config_a.substrate).await?;
    println!("Waiting for grandpa proofs to become available");
    let session_length = host_a.prover.session_length().await.unwrap();
    host_a
        .prover
        .client
        .blocks()
        .subscribe_finalized()
        .await
        .unwrap()
        .filter_map(|result| futures::future::ready(result.ok()))
        .skip_while(|h| futures::future::ready(h.number() < (session_length * 2) + 10))
        .take(1)
        .collect::<Vec<_>>()
        .await;

    println!("Grandpa proofs are now available");
    let consensus_state = host_a.prover.initialize_consensus_state(12).await?;
    let message_for_b = CreateConsensusState {
        consensus_state: consensus_state.encode(),
        consensus_client_id: GRANDPA_CONSENSUS_ID,
        consensus_state_id: POLKADOT_CONSENSUS_STATE_ID,
        unbonding_period: 60 * 60 * 24 * 14,
        challenge_period: 60,
        state_machine_commitments: vec![],
    };

    let config_b = GrandpaConfig {
        chain: "ws://localhost:9944".to_string(),
        state_machine: StateMachine::Kusama(0),
        consensus_state_id: KUSAMA_CONSENSUS_STATE_ID,
        substrate: SubstrateConfig {
            state_machine: StateMachine::Kusama(2001),
            hashing: HashAlgorithm::Blake2,
            consensus_state_id: "sama".to_string(),
            ws_url: "ws://localhost:9188".to_string(),
            signer: "0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a"
                .to_string(),
            latest_height: None,
        },
        grandpa_prover_config: GrandpaProverConfig {
            para_ids: vec![2001],
            babe_epoch_start_key: None,
            current_set_id: None,
        },
    };

    let host_b = GrandpaHost::<Hyperbridge>::new(&config_b).await?;
    let chain_b = SubstrateClient::new(host_b.clone(), config_b.substrate).await?;
    let consensus_state = host_b.prover.initialize_consensus_state(12).await?;
    let message_for_a = CreateConsensusState {
        consensus_state: consensus_state.encode(),
        consensus_client_id: GRANDPA_CONSENSUS_ID,
        consensus_state_id: KUSAMA_CONSENSUS_STATE_ID,
        unbonding_period: 60 * 60 * 24 * 14,
        challenge_period: 60,
        state_machine_commitments: vec![],
    };
    chain_b.create_consensus_state(message_for_b).await?;
    chain_a.create_consensus_state(message_for_a).await?;
    Ok((chain_a, chain_b))
}

#[tokio::test]
async fn test_grandpa_messaging_relay() -> Result<(), anyhow::Error> {
    setup_logging();

    let (chain_a, chain_b) = setup_clients().await?;

    let _ = tokio::spawn({
        let chain_a = chain_a.clone();
        let chain_b = chain_b.clone();
        async move { tesseract_consensus::relay(chain_a, chain_b).await.unwrap() }
    });

    let _ = tokio::spawn({
        let chain_a = chain_a.clone();
        let chain_b = chain_b.clone();
        async move { tesseract_messaging::relay(chain_a, chain_b, None).await.unwrap() }
    });

    // Make transfers each from both chains
    transfer_assets(&chain_a, &chain_b, 60 * 20).await?;
    Ok(())
}
