#![cfg(test)]

use crate::util::setup_logging;

use ismp::host::StateMachine;

use substrate_state_machine::HashAlgorithm;
use tesseract_grandpa::{GrandpaConfig, GrandpaHost, HostConfig};
use tesseract_primitives::IsmpHost;
use tesseract_substrate::{config::Blake2SubstrateChain, SubstrateConfig};

async fn setup_clients() -> Result<
	(
		GrandpaHost<Blake2SubstrateChain, Blake2SubstrateChain>,
		GrandpaHost<Blake2SubstrateChain, Blake2SubstrateChain>,
	),
	anyhow::Error,
> {
	let config_a = GrandpaConfig {
		substrate: SubstrateConfig {
			state_machine: StateMachine::Substrate(*b"SOLO"),
			hashing: Some(HashAlgorithm::Keccak),
			consensus_state_id: Some("GRNP".to_string()),
			signer: Some(
				"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
			),
			initial_height: None,
			poll_interval: None,
			rpc_ws: "ws://localhost:9990".to_string(),
			max_rpc_payload_size: None,
			max_concurent_queries: None,
		},
		grandpa: HostConfig {
			rpc: "ws://localhost:9922".to_string(),
			slot_duration: 12,
			consensus_update_frequency: Some(60),
			para_ids: vec![],
		},
	};

	let config_b = GrandpaConfig {
		substrate: SubstrateConfig {
			state_machine: StateMachine::Kusama(2001),
			hashing: Some(HashAlgorithm::Keccak),
			consensus_state_id: Some("GRNP".to_string()),
			signer: Some(
				"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
			),
			initial_height: None,
			poll_interval: None,
			rpc_ws: "ws://localhost:9991".to_string(),
			max_rpc_payload_size: None,
			max_concurent_queries: None,
		},
		grandpa: HostConfig {
			rpc: "ws://localhost:9922".to_string(),
			slot_duration: 12,
			consensus_update_frequency: Some(60),
			para_ids: vec![2001],
		},
	};
	let chain_a = GrandpaHost::<Blake2SubstrateChain, Blake2SubstrateChain>::new(&config_a).await?;

	let message_for_b = chain_a.query_initial_consensus_state().await?.unwrap();

	let chain_b = GrandpaHost::<Blake2SubstrateChain, Blake2SubstrateChain>::new(&config_b).await?;

	let message_for_a = chain_b.query_initial_consensus_state().await?.unwrap();
	log::info!("🧊 Setting consensus states");
	chain_b.provider().set_initial_consensus_state(message_for_b).await?;
	chain_a.provider().set_initial_consensus_state(message_for_a).await?;
	Ok((chain_a, chain_b))
}

#[tokio::test]
async fn test_grandpa_messaging_relay() -> Result<(), anyhow::Error> {
	setup_logging();

	log::info!("🧊 Initializing tesseract consensus");

	let (chain_a, chain_b) = setup_clients().await?;

	let handle = tokio::spawn({
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		async move { chain_a.start_consensus(chain_b.provider()).await.unwrap() }
	});

	let handle_b = tokio::spawn({
		let chain_a = chain_a.clone();
		let chain_b = chain_b.clone();
		async move { chain_b.start_consensus(chain_a.provider()).await.unwrap() }
	});

	log::info!("🧊 Initialized consensus tasks");

	let _ = tokio::join!(handle, handle_b);

	Ok(())
}
