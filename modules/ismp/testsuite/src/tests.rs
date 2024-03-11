

use ismp::host::StateMachine;

use crate::{
    check_challenge_period, check_client_expiry, check_request_source_and_destinatione, frozen_check, mock_consensus_state_id, mock_proxy_consensus_state_id, mocks::{Host, MockDispatcher}, post_request_timeout_check, post_response_timeout_check, prevent_request_timeout_on_proxy_with_known_state_machine, prevent_response_timeout_on_proxy_with_known_state_machine, sanity_check_for_proxies, write_outgoing_commitments
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
fn should_prevent_request_timeout_on_proxy_with_known_state_machine () {
    let host = Arc::new(Host::default());
    let dispatcher = MockDispatcher(host.clone());
    let proxy_state_machine = StateMachine::Kusama(2000);
    let direct_conn_state_machine = StateMachine::Bsc; 
    prevent_request_timeout_on_proxy_with_known_state_machine(&*host, &dispatcher, proxy_state_machine, direct_conn_state_machine).unwrap()
}


#[test]
fn should_prevent_response_timeout_on_proxy_with_known_state_machine () {
    let host = Arc::new(Host::default());
    let dispatcher = MockDispatcher(host.clone());
    let proxy_state_machine = StateMachine::Kusama(2000);
    let direct_conn_state_machine = StateMachine::Bsc; 
    prevent_response_timeout_on_proxy_with_known_state_machine(&*host, &dispatcher, proxy_state_machine, direct_conn_state_machine).unwrap()
}

#[test]
fn should_check_request_source_and_destinatione() {
    let host = Arc::new(Host::default());
    let dispatcher = MockDispatcher(host.clone());
    check_request_source_and_destinatione(&*host, &dispatcher).unwrap();
}

