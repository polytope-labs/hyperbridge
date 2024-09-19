use crate::verify_parachain_headers_with_grandpa_finality_proof;
use codec::{Decode, Encode};
use futures::StreamExt;
use grandpa_prover::GrandpaProver;
use grandpa_verifier_primitives::{
	justification::GrandpaJustification, ParachainHeadersWithFinalityProof,
};
use ismp::host::StateMachine;
use polkadot_core_primitives::Header;
use serde::{Deserialize, Serialize};
use subxt::{
	config::substrate::{BlakeTwo256, SubstrateHeader},
	rpc_params,
};
pub type Justification = GrandpaJustification<Header>;

/// An encoded justification proving that the given header has been finalized
#[derive(Clone, Serialize, Deserialize)]
pub struct JustificationNotification(sp_core::Bytes);

#[ignore]
#[tokio::test]
async fn follow_grandpa_justifications() {
	env_logger::builder()
		.filter_module("grandpa", log::LevelFilter::Trace)
		.format_module_path(false)
		.init();

	let relay_ws_url = std::env::var("RELAY_HOST")
		.unwrap_or_else(|_| "wss://hyperbridge-paseo-relay.blockops.network:443".to_string());

	// let relay_ws_url = format!("ws://{relay}:9944");

	let para_ids = vec![2000];
	let babe_epoch_start_key =
		hex::decode("1cb6f36e027abb2091cfb5110ab5087fe90e2fbf2d792cb324bffa9427fe1f0e").unwrap();
	let current_set_id_key =
		hex::decode("5f9cc45b7a00c5899361e1c6099678dc8a2d09463effcc78a22d75b9cb87dffc").unwrap();

	println!("Connecting to relay chain {relay_ws_url}");
	let prover = GrandpaProver::<subxt_utils::BlakeSubstrateChain>::new(
		&relay_ws_url,
		para_ids,
		StateMachine::Polkadot(0),
		babe_epoch_start_key,
		current_set_id_key,
	)
	.await
	.unwrap();

	println!("Connected to relay chain");

	println!("Waiting for grandpa proofs to become available");
	let session_length = prover.session_length().await.unwrap();
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

	let mut subscription = prover
		.client
		.rpc()
		.subscribe::<JustificationNotification>(
			"grandpa_subscribeJustifications",
			rpc_params![],
			"grandpa_unsubscribeJustifications",
		)
		.await
		.unwrap();

	// slot duration in milliseconds for parachains
	let slot_duration = 6000;

	let mut consensus_state = prover.initialize_consensus_state(slot_duration).await.unwrap();

	println!("Grandpa proofs are now available");
	while let Some(Ok(_)) = subscription.next().await {
		let next_relay_height = consensus_state.latest_height + 1;

		// prove finality should give us the justification for the highest finalized block of the
		// authority set the block provided to it belongs
		let finality_proof = prover
			.query_finality_proof::<SubstrateHeader<u32, BlakeTwo256>>(
				consensus_state.latest_height,
				next_relay_height,
			)
			.await
			.unwrap();

		let justification = Justification::decode(&mut &finality_proof.justification[..]).unwrap();

		println!("current_set_id: {}", consensus_state.current_set_id);
		println!("latest_relay_height: {}", consensus_state.latest_height);
		println!(
			"For relay chain header: Hash({:?}), Number({})",
			justification.commit.target_hash, justification.commit.target_number
		);

		let proof = prover
			.query_finalized_parachain_headers_with_proof::<SubstrateHeader<u32, BlakeTwo256>>(
				consensus_state.latest_height,
				justification.commit.target_number,
				finality_proof.clone(),
			)
			.await
			.expect("Failed to fetch finalized parachain headers with proof");

		let proof = proof.encode();
		let proof = ParachainHeadersWithFinalityProof::<Header>::decode(&mut &*proof).unwrap();

		let (new_consensus_state, _parachain_headers) =
			verify_parachain_headers_with_grandpa_finality_proof::<Header>(
				consensus_state.clone(),
				proof.clone(),
			)
			.expect("Failed to verify parachain headers with grandpa finality_proof");

		if !proof.parachain_headers.is_empty() {
			assert!(new_consensus_state.latest_height > consensus_state.latest_height);
		}

		consensus_state = new_consensus_state;
		println!("========= Successfully verified grandpa justification =========");
	}
}
