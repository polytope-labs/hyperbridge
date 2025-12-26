pub fn header_route(block_id: &str) -> String {
    format!("//eth/v1/beacon/headers/{}", block_id)
}

pub fn beacon_state_route(state_id: &str) -> String {
    format!("/eth/v2/debug/beacon/states/{}", state_id)
}

pub fn consensus_data_route(block_id: &str) -> String {
    format!("/eth/v1/beacon/consensus/{}", block_id
    )
}