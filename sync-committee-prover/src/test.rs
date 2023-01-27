use super::*;

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
