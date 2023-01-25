mod responses;
#[cfg(test)]
mod test;

use ethereum_consensus::bellatrix::{
    BeaconBlock, BeaconBlockHeader, SignedBeaconBlock, SignedBeaconBlockHeader, SyncCommittee,
};
use reqwest::Client;

use ethereum_consensus::bellatrix::mainnet::{
    BYTES_PER_LOGS_BLOOM, MAX_BYTES_PER_TRANSACTION, MAX_EXTRA_DATA_BYTES,
    MAX_TRANSACTIONS_PER_PAYLOAD, SYNC_COMMITTEE_SIZE,
};
use ethereum_consensus::phase0::mainnet::{
    EPOCHS_PER_HISTORICAL_VECTOR, EPOCHS_PER_SLASHINGS_VECTOR, ETH1_DATA_VOTES_BOUND,
    HISTORICAL_ROOTS_LIMIT, MAX_ATTESTATIONS, MAX_ATTESTER_SLASHINGS, MAX_DEPOSITS,
    MAX_PROPOSER_SLASHINGS, MAX_VALIDATORS_PER_COMMITTEE, MAX_VOLUNTARY_EXITS,
    SLOTS_PER_HISTORICAL_ROOT, VALIDATOR_REGISTRY_LIMIT,
};

pub fn header_route(block_id: String) -> String {
    format!("/eth/v1/beacon/headers/{}", block_id)
}

pub fn block_route(block_id: String) -> String {
    format!("/eth/v2/beacon/blocks/{}", block_id)
}

pub fn sync_committee_route(state_id: String) -> String {
    format!("/eth/v1/beacon/states/{}/sync_committees", state_id)
}

pub struct SyncCommitteeProver {
    pub node_url: String,
    pub client: Client,
}

impl SyncCommitteeProver {
    pub fn new(node_url: String) -> Self {
        let client = reqwest::Client::new();

        SyncCommitteeProver { node_url, client }
    }

    pub async fn fetch_header(
        &self,
        block_id: String,
    ) -> Result<BeaconBlockHeader, reqwest::Error> {
        let path = header_route(block_id);
        let full_url = format!("{}{}", self.node_url.clone(), path);
        let response = self.client.get(full_url).send().await?;
        let response_data = response
            .json::<responses::beacon_block_header_response::Response>()
            .await?;

        let beacon_block_header = response_data.data.header.message;

        Ok(beacon_block_header)
    }
    pub async fn fetch_block(
        &self,
        block_id: String,
    ) -> Result<
        BeaconBlock<
            MAX_PROPOSER_SLASHINGS,
            MAX_VALIDATORS_PER_COMMITTEE,
            MAX_ATTESTER_SLASHINGS,
            MAX_ATTESTATIONS,
            MAX_DEPOSITS,
            MAX_VOLUNTARY_EXITS,
            SYNC_COMMITTEE_SIZE,
            BYTES_PER_LOGS_BLOOM,
            MAX_EXTRA_DATA_BYTES,
            MAX_BYTES_PER_TRANSACTION,
            MAX_TRANSACTIONS_PER_PAYLOAD,
        >,
        reqwest::Error,
    > {
        let path = block_route(block_id);
        let full_url = format!("{}/{}", self.node_url.clone(), path);

        let response = self.client.get(full_url).send().await?;

        let beacon_block = response
            .json::<BeaconBlock<
                MAX_PROPOSER_SLASHINGS,
                MAX_VALIDATORS_PER_COMMITTEE,
                MAX_ATTESTER_SLASHINGS,
                MAX_ATTESTATIONS,
                MAX_DEPOSITS,
                MAX_VOLUNTARY_EXITS,
                SYNC_COMMITTEE_SIZE,
                BYTES_PER_LOGS_BLOOM,
                MAX_EXTRA_DATA_BYTES,
                MAX_BYTES_PER_TRANSACTION,
                MAX_TRANSACTIONS_PER_PAYLOAD,
            >>()
            .await;

        beacon_block
    }
    pub async fn fetch_sync_committee(
        &self,
        state_id: String,
    ) -> Result<SyncCommittee<SYNC_COMMITTEE_SIZE>, reqwest::Error> {
        let path = sync_committee_route(state_id);
        let full_url = format!("{}/{}", self.node_url.clone(), path);

        let response = self.client.get(full_url).send().await?;

        let sync_committee = response.json::<SyncCommittee<SYNC_COMMITTEE_SIZE>>().await;

        sync_committee
    }
    /*pub fn signed_beacon_block(beacon_block: BeaconBlock) -> SignedBeaconBlock {  }
    pub fn signed_beacon_block_header(beacon_block: SignedBeaconBlock) -> SignedBeaconBlockHeader {  }*/
}
