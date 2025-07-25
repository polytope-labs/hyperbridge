#![cfg(test)]
use polkadot_sdk::*;

use crate::{verify_grandpa_finality_proof, verify_parachain_headers_with_grandpa_finality_proof};
use anyhow::anyhow;
use codec::{Decode, Encode};
use futures::StreamExt;
use grandpa_prover::{GrandpaProver, ProverOptions};
use grandpa_verifier_primitives::{
	justification::GrandpaJustification, FinalityProof, ParachainHeadersWithFinalityProof,
};
use ismp::host::StateMachine;
use polkadot_core_primitives::Header;
use serde::{Deserialize, Serialize};
use subxt::{
	backend::legacy::LegacyRpcMethods,
	ext::subxt_rpcs::{rpc_params, RpcClient},
	OnlineClient,
};

pub type Justification = GrandpaJustification<Header>;

/// An encoded justification proving that the given header has been finalized
#[derive(Clone, Serialize, Deserialize)]
pub struct JustificationNotification(sp_core::Bytes);

/// Returns the session length in blocks
pub async fn session_length<T: subxt::Config>(
	client: &OnlineClient<T>,
) -> Result<u32, anyhow::Error> {
	let metadata = client.metadata();
	let metadata = metadata
		.pallet_by_name_err("Babe")?
		.constant_by_name("EpochDuration")
		.ok_or(anyhow!("Failed to fetch constant"))?;
	Ok(Decode::decode(&mut metadata.value())?)
}

#[ignore]
#[tokio::test]
async fn follow_grandpa_justifications() {
	env_logger::builder()
		.filter_module("grandpa", log::LevelFilter::Trace)
		.format_module_path(false)
		.init();

	let (ws_url, para_ids, is_relay) =
		match (std::env::var("RELAY_HOST"), std::env::var("SOLO_HOST")) {
			(Ok(relay_url), _) => (relay_url, vec![1000], true),
			(_, Ok(solo_url)) => (solo_url, vec![], false),
			_ => panic!("Please supply either RELAY_HOST or SOLO_HOST"),
		};

	log::info!("Connecting to relay chain {ws_url}");
	let prover = GrandpaProver::<subxt_utils::BlakeSubstrateChain>::new(ProverOptions {
		ws_url: ws_url.clone(),
		para_ids,
		state_machine: StateMachine::Polkadot(0),
		max_rpc_payload_size: u32::MAX,
		max_block_range: 2000,
	})
	.await
	.unwrap();

	let prover_rpc_client = RpcClient::from_url(&ws_url).await.unwrap();
	let prover_rpc =
		LegacyRpcMethods::<subxt_utils::BlakeSubstrateChain>::new(prover_rpc_client.clone());

	log::info!("Connected to relay chain");
	log::info!("Waiting for grandpa proofs to become available");
	let session_length = session_length(&prover.client).await.unwrap();
	prover
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

	let mut subscription = prover_rpc_client
		.subscribe::<JustificationNotification>(
			"grandpa_subscribeJustifications",
			rpc_params![],
			"grandpa_unsubscribeJustifications",
		)
		.await
		.unwrap();

	// slot duration in milliseconds for parachains
	let slot_duration = 6000;
	let hash = prover_rpc.chain_get_finalized_head().await.unwrap();
	let mut consensus_state = prover.initialize_consensus_state(slot_duration, hash).await.unwrap();

	log::info!("Grandpa proofs are now available");
	while let Some(result) = subscription.next().await {
		match result {
			Ok(_) => {},
			Err(err) => {
				log::error!("Got error in subscription stream: {err:?}");
				continue;
			},
		}

		// prove finality should give us the justification for the highest finalized block of the
		// authority set the block provided to it belongs
		let finality_proof = prover.query_finality_proof(consensus_state.clone()).await.unwrap();

		let proof = finality_proof.encode();
		let finality_proof = FinalityProof::<Header>::decode(&mut &*proof).unwrap();

		let (new_consensus_state, _, _, _) = verify_grandpa_finality_proof::<Header>(
			consensus_state.clone(),
			finality_proof.clone(),
		)
		.unwrap();

		if is_relay {
			let justification =
				Justification::decode(&mut &finality_proof.justification[..]).unwrap();

			log::info!("current_set_id: {}", consensus_state.current_set_id);
			log::info!("latest_relay_height: {}", consensus_state.latest_height);
			log::info!(
				"For relay chain header: Hash({:?}), Number({})",
				justification.commit.target_hash,
				justification.commit.target_number
			);

			let parachain_headers = prover
				.query_finalized_parachain_headers_with_proof(justification.commit.target_hash)
				.await
				.expect("Failed to fetch finalized parachain headers with proof");

			let proof = proof.encode();
			let proof = ParachainHeadersWithFinalityProof::<Header>::decode(&mut &*proof).unwrap();

			let (new_consensus_state, _parachain_headers) =
				verify_parachain_headers_with_grandpa_finality_proof::<Header>(
					consensus_state.clone(),
					ParachainHeadersWithFinalityProof { finality_proof, parachain_headers },
				)
				.expect("Failed to verify parachain headers with grandpa finality_proof");

			if !proof.parachain_headers.is_empty() {
				assert!(new_consensus_state.latest_height > consensus_state.latest_height);
			}
		}

		consensus_state = new_consensus_state;
		log::info!("========= Successfully verified grandpa justification =========");
	}
}
