use crate::{
    check_challenge_period, check_client_expiry, frozen_check, mocks::Host,
    timeout_post_processing_check, write_outgoing_commitments,
};

#[test]
fn check_for_duplicate_requests_and_responses() {
    let host = Host::default();
    write_outgoing_commitments(&host).unwrap();
}

#[test]
fn should_reject_updates_within_challenge_period() {
    let host = Host::default();
    check_challenge_period(&host).unwrap()
}

#[test]
fn should_reject_messages_for_frozen_state_machines() {
    let host = Host::default();
    frozen_check(&host).unwrap()
}

#[test]
fn should_reject_expired_check_clients() {
    let host = Host::default();
    check_client_expiry(&host).unwrap()
}
#[test]
fn should_process_timeouts_correctly() {
    let host = Host::default();
    timeout_post_processing_check(&host).unwrap()
}
