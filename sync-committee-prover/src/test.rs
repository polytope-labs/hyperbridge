use super::*;
use ssz_rs::Merkleized;

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn fetch_block_header_works() {
    let node_url: String = "http://localhost:3500".to_string();
    let sync_committee_prover = SyncCommitteeProver::new(node_url);
    let block_header = sync_committee_prover.fetch_header("100".to_string()).await;
    assert!(block_header.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn fetch_block_works() {
    let node_url: String = "http://localhost:3500".to_string();
    let sync_committee_prover = SyncCommitteeProver::new(node_url);
    let block = sync_committee_prover.fetch_block("100".to_string()).await;
    assert!(block.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn fetch_sync_committee_works() {
    let node_url: String = "http://localhost:3500".to_string();
    let sync_committee_prover = SyncCommitteeProver::new(node_url);
    let block = sync_committee_prover
        .fetch_sync_committee("117".to_string())
        .await;
    assert!(block.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn fetch_validator_works() {
    let node_url: String = "http://localhost:3500".to_string();
    let sync_committee_prover = SyncCommitteeProver::new(node_url);
    let validator = sync_committee_prover
        .fetch_validator("2561".to_string(), "48".to_string())
        .await;
    assert!(validator.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn fetch_processed_sync_committee_works() {
    let node_url: String = "http://localhost:3500".to_string();
    let sync_committee_prover = SyncCommitteeProver::new(node_url);
    let validator = sync_committee_prover
        .fetch_processed_sync_committee("2561".to_string())
        .await;
    assert!(validator.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn fetch_signed_beacon_block_works() {
    let node_url: String = "http://localhost:3500".to_string();
    let sync_committee_prover = SyncCommitteeProver::new(node_url);
    let block = sync_committee_prover.fetch_block("100".to_string()).await;
    assert!(block.is_ok());
    let signed_beacon_block = sync_committee_prover.signed_beacon_block(block.unwrap());
    assert!(signed_beacon_block.is_some());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn fetch_signed_beacon_block_header_works() {
    let node_url: String = "http://localhost:3500".to_string();
    let sync_committee_prover = SyncCommitteeProver::new(node_url);
    // fetch beacon block header
    let header = sync_committee_prover.fetch_header("100".to_string()).await;
    assert!(header.is_ok());

    // fetch block
    let block = sync_committee_prover.fetch_block("100".to_string()).await;
    assert!(block.is_ok());
    // fetch signed beacon block
    let signed_beacon_block = sync_committee_prover.signed_beacon_block(block.unwrap());
    assert!(signed_beacon_block.is_some());

    // fetch sigend beacon block header
    let signed_beacon_block_header =
        sync_committee_prover.signed_beacon_block_header(signed_beacon_block, header.unwrap());
    assert!(signed_beacon_block_header.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn fetch_beacon_state_works() {
    let node_url: String = "http://localhost:3500".to_string();
    let sync_committee_prover = SyncCommitteeProver::new(node_url);
    let beacon_state = sync_committee_prover
        .fetch_beacon_state("genesis".to_string())
        .await;
    assert!(beacon_state.is_ok());
}

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn state_root_and_block_header_root_matches() {
    let node_url: String = "http://localhost:3500".to_string();
    let sync_committee_prover = SyncCommitteeProver::new(node_url);
    let beacon_state = sync_committee_prover
        .fetch_beacon_state("100".to_string())
        .await;
    assert!(beacon_state.is_ok());

    let block_header = sync_committee_prover.fetch_header("100".to_string()).await;
    assert!(block_header.is_ok());

    let state = beacon_state.unwrap();
    let block_header = block_header.unwrap();
    let hash_tree_root = state.clone().hash_tree_root();

    assert!(block_header.state_root == hash_tree_root.unwrap());
}
