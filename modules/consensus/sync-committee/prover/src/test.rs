use super::*;
use reqwest_eventsource::EventSource;
use ssz_rs::{calculate_multi_merkle_root, is_valid_merkle_branch, GeneralizedIndex, Merkleized};
use sync_committee_primitives::{
    constants::{
        devnet::Devnet, Root, EXECUTION_PAYLOAD_INDEX_LOG2, NEXT_SYNC_COMMITTEE_INDEX_LOG2,
    },
    types::VerifierState,
};
use sync_committee_verifier::verify_sync_committee_attestation;
use tokio_stream::StreamExt;

#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn fetch_block_header_works() {
    let sync_committee_prover = setup_prover();
    let block_header = sync_committee_prover.fetch_header("head").await;
    assert!(block_header.is_ok());
}

#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn fetch_block_works() {
    let sync_committee_prover = setup_prover();
    let block = sync_committee_prover.fetch_block("head").await;
    assert!(block.is_ok());
}

#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn fetch_validator_works() {
    let sync_committee_prover = setup_prover();
    let validator = sync_committee_prover.fetch_validator("head", "0").await;
    assert!(validator.is_ok());
}

#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn fetch_beacon_state_works() {
    let sync_committee_prover = setup_prover();
    let beacon_state = sync_committee_prover.fetch_beacon_state("head").await;
    assert!(beacon_state.is_ok());
}

#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn state_root_and_block_header_root_matches() {
    let sync_committee_prover = setup_prover();
    let mut beacon_state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();

    let block_header = sync_committee_prover.fetch_header(&beacon_state.slot.to_string()).await;
    assert!(block_header.is_ok());

    let block_header = block_header.unwrap();
    let hash_tree_root = beacon_state.hash_tree_root();

    assert_eq!(block_header.state_root, hash_tree_root.unwrap());
}

#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn fetch_finality_checkpoints_work() {
    let sync_committee_prover = setup_prover();
    let finality_checkpoint = sync_committee_prover.fetch_finalized_checkpoint(None).await;
    assert!(finality_checkpoint.is_ok());
}

#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn test_finalized_header() {
    let sync_committee_prover = setup_prover();
    let mut state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();

    let proof = ssz_rs::generate_proof(&mut state, &vec![FINALIZED_ROOT_INDEX as usize]).unwrap();

    let leaves = vec![Node::from_bytes(
        state
            .finalized_checkpoint
            .hash_tree_root()
            .unwrap()
            .as_ref()
            .try_into()
            .unwrap(),
    )];
    let root = calculate_multi_merkle_root(
        &leaves,
        &proof,
        &[GeneralizedIndex(FINALIZED_ROOT_INDEX as usize)],
    );
    assert_eq!(root, state.hash_tree_root().unwrap());
}

#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn test_execution_payload_proof() {
    let sync_committee_prover = setup_prover();

    let mut finalized_state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();
    let block_id = finalized_state.slot.to_string();
    let execution_payload_proof = prove_execution_payload::<Devnet>(&mut finalized_state).unwrap();

    let finalized_header = sync_committee_prover.fetch_header(&block_id).await.unwrap();

    // verify the associated execution header of the finalized beacon header.
    let mut execution_payload = execution_payload_proof.clone();
    let multi_proof_vec = execution_payload.multi_proof;
    let execution_payload_root = calculate_multi_merkle_root(
        &[
            Node::from_bytes(execution_payload.state_root.as_ref().try_into().unwrap()),
            execution_payload.block_number.hash_tree_root().unwrap(),
            execution_payload.timestamp.hash_tree_root().unwrap(),
        ],
        &multi_proof_vec,
        &[
            GeneralizedIndex(Devnet::EXECUTION_PAYLOAD_STATE_ROOT_INDEX as usize),
            GeneralizedIndex(Devnet::EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX as usize),
            GeneralizedIndex(Devnet::EXECUTION_PAYLOAD_TIMESTAMP_INDEX as usize),
        ],
    );

    let execution_payload_hash_tree_root = finalized_state
        .latest_execution_payload_header
        .clone()
        .hash_tree_root()
        .unwrap();

    assert_eq!(execution_payload_root, execution_payload_hash_tree_root);

    let execution_payload_branch = execution_payload.execution_payload_branch.iter();

    let is_merkle_branch_valid = is_valid_merkle_branch(
        &execution_payload_root,
        execution_payload_branch,
        EXECUTION_PAYLOAD_INDEX_LOG2 as usize,
        EXECUTION_PAYLOAD_INDEX as usize,
        &finalized_header.state_root,
    );

    assert!(is_merkle_branch_valid);
}

#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn test_sync_committee_update_proof() {
    let sync_committee_prover = setup_prover();

    let mut finalized_state = sync_committee_prover.fetch_beacon_state("head").await.unwrap();
    let block_id = finalized_state.slot.to_string();
    let finalized_header = sync_committee_prover.fetch_header(&block_id).await.unwrap();

    let sync_committee_proof = prove_sync_committee_update::<Devnet>(&mut finalized_state).unwrap();

    let mut sync_committee = finalized_state.next_sync_committee;

    let calculated_finalized_root = calculate_multi_merkle_root(
        &[sync_committee.hash_tree_root().unwrap()],
        &sync_committee_proof,
        &[GeneralizedIndex(NEXT_SYNC_COMMITTEE_INDEX as usize)],
    );

    assert_eq!(calculated_finalized_root.as_bytes(), finalized_header.state_root.as_bytes());

    let is_merkle_branch_valid = is_valid_merkle_branch(
        &sync_committee.hash_tree_root().unwrap(),
        sync_committee_proof.iter(),
        NEXT_SYNC_COMMITTEE_INDEX_LOG2 as usize,
        NEXT_SYNC_COMMITTEE_INDEX as usize,
        &finalized_header.state_root,
    );

    assert!(is_merkle_branch_valid);
}

#[allow(non_snake_case)]
#[tokio::test]
#[ignore]
async fn test_prover() {
    use log::LevelFilter;
    use parity_scale_codec::{Decode, Encode};
    env_logger::builder()
        .filter_module("prover", LevelFilter::Debug)
        .format_module_path(false)
        .init();

    let sync_committee_prover = setup_prover();
    let node_url =
        format!("{}/eth/v1/events?topics=finalized_checkpoint", sync_committee_prover.primary_url);
    let block_header = sync_committee_prover.fetch_header("head").await.unwrap();

    let state = sync_committee_prover
        .fetch_beacon_state(&block_header.slot.to_string())
        .await
        .unwrap();

    let mut client_state = VerifierState {
        finalized_header: block_header.clone(),
        latest_finalized_epoch: compute_epoch_at_slot::<Devnet>(block_header.slot),
        current_sync_committee: state.current_sync_committee,
        next_sync_committee: state.next_sync_committee,
        state_period: compute_sync_committee_period_at_slot::<Devnet>(block_header.slot),
    };

    let mut count = 0;

    let mut es = EventSource::get(node_url);
    while let Some(event) = es.next().await {
        match event {
            Ok(reqwest_eventsource::Event::Message(msg)) => {
                let message: EventResponse = serde_json::from_str(&msg.data).unwrap();
                let checkpoint =
                    Checkpoint { epoch: message.epoch.parse().unwrap(), root: message.block };
                let light_client_update = if let Some(update) = sync_committee_prover
                    .fetch_light_client_update(client_state.clone(), checkpoint, None)
                    .await
                    .unwrap()
                {
                    update
                } else {
                    continue;
                };

                if light_client_update.sync_committee_update.is_some() {
                    println!("Sync committee update present");
                    dbg!(light_client_update.attested_header.slot);
                    dbg!(light_client_update.finalized_header.slot);
                    dbg!(client_state.finalized_header.slot);
                }
                let encoded = light_client_update.encode();
                let decoded = VerifierStateUpdate::decode(&mut &*encoded).unwrap();
                assert_eq!(light_client_update, decoded);

                client_state = verify_sync_committee_attestation::<Devnet>(
                    client_state.clone(),
                    light_client_update,
                )
                .unwrap();
                println!(
                    "Sucessfully verified Ethereum block at slot {:?}",
                    client_state.finalized_header.slot
                );

                count += 1;
                // For CI purposes we test finalization of 2 epochs
                if count == 2 {
                    break;
                }
            },
            Err(err) => {
                panic!("Encountered Error and closed stream {err:?}");
            },
            _ => continue,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EventResponse {
    pub block: Root,
    pub state: Root,
    pub epoch: String,
    pub execution_optimistic: bool,
}

fn setup_prover() -> SyncCommitteeProver<Devnet> {
    dotenv::dotenv().ok();
    let consensus_url =
        std::env::var("CONSENSUS_NODE_URL").unwrap_or("http://localhost:3500".to_string());
    SyncCommitteeProver::<Devnet>::new(consensus_url)
}
