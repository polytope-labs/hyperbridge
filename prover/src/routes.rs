pub fn header_route(block_id: &str) -> String {
	format!("/eth/v1/beacon/headers/{block_id}")
}

pub fn block_route(block_id: &str) -> String {
	format!("/eth/v2/beacon/blocks/{block_id}")
}

pub fn sync_committee_route(state_id: &str) -> String {
	format!("/eth/v1/beacon/states/{state_id}/sync_committees")
}

pub fn validator_route(state_id: &str, validator_index: &str) -> String {
	format!("/eth/v1/beacon/states/{state_id}/validators/{validator_index}")
}
pub fn beacon_state_route(state_id: &str) -> String {
	format!("/eth/v2/debug/beacon/states/{state_id}")
}
pub fn finality_checkpoints(state_id: &str) -> String {
	format!("/eth/v1/beacon/states/{state_id}/finality_checkpoints")
}
