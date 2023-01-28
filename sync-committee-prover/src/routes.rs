pub fn header_route(block_id: String) -> String {
    format!("/eth/v1/beacon/headers/{}", block_id)
}

pub fn block_route(block_id: String) -> String {
    format!("/eth/v2/beacon/blocks/{}", block_id)
}

pub fn sync_committee_route(state_id: String) -> String {
    format!("/eth/v1/beacon/states/{}/sync_committees", state_id)
}

pub fn validator_route(state_id: String, validator_index: String) -> String {
    format!(
        "/eth/v1/beacon/states/{}/validators/{}",
        state_id, validator_index
    )
}
