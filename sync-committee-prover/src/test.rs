use super::*;

#[cfg(test)]
#[allow(non_snake_case)]
#[actix_rt::test]
async fn fetches_block_header_works() {
    let node_url: String = "http://localhost:3500".to_string();
    let sync_committee_prover = SyncCommitteeProver::new(node_url);
    let block_header = sync_committee_prover.fetch_header("100".to_string()).await;
    assert!(block_header.is_ok());
}
