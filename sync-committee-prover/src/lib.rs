mod responses;
#[cfg(test)]
mod test;

use ethereum_consensus::bellatrix::{
    BeaconBlock, BeaconBlockHeader, SignedBeaconBlock, SignedBeaconBlockHeader, SyncCommittee,
};
use reqwest::Client;

//TODO: Remove all these and use the light client primitive consts in the other PR
const MAX_PROPOSER_SLASHINGS: usize = 0;
const MAX_VALIDATORS_PER_COMMITTEE: usize = 0;
const MAX_ATTESTER_SLASHINGS: usize = 0;
const MAX_ATTESTATIONS: usize = 0;
const MAX_DEPOSITS: usize = 0;
const MAX_VOLUNTARY_EXITS: usize = 0;
const SYNC_COMMITTEE_SIZE: usize = 0;
const BYTES_PER_LOGS_BLOOM: usize = 0;
const MAX_EXTRA_DATA_BYTES: usize = 0;
const MAX_BYTES_PER_TRANSACTION: usize = 0;
const MAX_TRANSACTIONS_PER_PAYLOAD: usize = 0;

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
