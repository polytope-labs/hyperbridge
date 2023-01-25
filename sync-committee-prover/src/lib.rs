use ethereum_consensus::bellatrix::{BeaconBlock, BeaconBlockHeader, SignedBeaconBlock, SignedBeaconBlockHeader, SyncCommittee};

pub fn header_route(block_id: String) -> String {
    format!("/eth/v1/beacon/headers/{}", block_id)
}

pub struct SyncCommitteeProver {
    pub node_url: String,
}

impl SyncCommitteeProver {

    pub fn new(node_url: String) -> Self {
        SyncCommitteeProver { node_url }
    }

    pub async fn fetch_header(&self, block_id: String) -> Result<BeaconBlockHeader, reqwest::Error> {
        let mut node_url = self.node_url.clone();
        let path =  header_route(block_id);
        node_url.push_str(&path);

        let client = reqwest::Client::new();
        let response = client
            .get(node_url).send()
            .await?;


        let beacon_block_header = response.json::<BeaconBlockHeader>().await;

        beacon_block_header
    }
    /*pub async fn fetch_block(block_id: String) -> BeaconBlock {  }
    pub async fn fetch_sync_committee(state_id: String) -> SyncCommittee<SYNC_COMMITTEE_SIZE> {  }
    pub fn signed_beacon_block(beacon_block: BeaconBlock) -> SignedBeaconBlock {  }
    pub fn signed_beacon_block_header(beacon_block: SignedBeaconBlock) -> SignedBeaconBlockHeader {  }*/
}
