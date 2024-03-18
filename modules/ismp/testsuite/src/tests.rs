use crate::{
    check_challenge_period, check_client_expiry, check_response_source, frozen_check,
    mocks::{Host, MockDispatcher},
    post_request_timeout_check, post_response_timeout_check, sanity_check_for_proxies,
    write_outgoing_commitments,
};
use std::sync::Arc;

#[test]
fn dispatcher_should_write_receipts_for_outgoing_requests_and_responses() {
    let host = Arc::new(Host::default());
    let dispatcher = MockDispatcher(host.clone());
    write_outgoing_commitments(&*host, &dispatcher).unwrap();
}

#[test]
#[ignore]
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
fn should_process_post_request_timeouts_correctly() {
    let host = Arc::new(Host::default());
    let dispatcher = MockDispatcher(host.clone());
    post_request_timeout_check(&*host, &dispatcher).unwrap()
}

#[test]
fn should_process_post_response_timeouts_correctly() {
    let host = Arc::new(Host::default());
    let dispatcher = MockDispatcher(host.clone());
    post_response_timeout_check(&*host, &dispatcher).unwrap()
}

#[test]
fn should_check_response_source() {
    let host = Arc::new(Host::default());
    check_response_source(&*host).unwrap()
}

#[test]
fn should_perform_sanity_check_for_proxies() {
    let host = Arc::new(Host::default());
    let dispatcher = MockDispatcher(host.clone());
    sanity_check_for_proxies(&*host, &dispatcher).unwrap()
}
