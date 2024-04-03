use ismp::host::{Ethereum, StateMachine};

use crate::{
    check_challenge_period, check_client_expiry, check_request_source_and_destination,
    check_response_source, frozen_consensus_client_check, frozen_state_machine_check,
    missing_state_commitment_check,
    mocks::{Host, MockDispatcher},
    post_request_timeout_check, post_response_timeout_check,
    prevent_request_processing_on_proxy_with_known_state_machine,
    prevent_request_timeout_on_proxy_with_known_state_machine,
    prevent_response_timeout_on_proxy_with_known_state_machine, write_outgoing_commitments,
};
use std::sync::Arc;

#[test]
fn dispatcher_should_write_receipts_for_outgoing_requests_and_responses() {
    let host = Arc::new(Host::default());
    let dispatcher = MockDispatcher(host.clone());
    write_outgoing_commitments(&*host, &dispatcher).unwrap();
}

#[test]
fn should_reject_updates_within_challenge_period() {
    let host = Host::default();
    check_challenge_period(&host).unwrap()
}

#[test]
fn should_reject_messages_for_deleted_state_machine_commitments() {
    let host = Host::default();
    missing_state_commitment_check(&host).unwrap()
}

#[test]
fn should_reject_messages_for_frozen_consensus_clients() {
    let host = Host::default();
    frozen_consensus_client_check(&host).unwrap()
}

#[test]
fn should_reject_messages_for_frozen_state_machine_clients() {
    let host = Host::default();
    frozen_state_machine_check(&host).unwrap()
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
fn should_prevent_request_timeout_on_proxy_with_known_state_machine() {
    let direct_conn_state_machine = StateMachine::Ethereum(Ethereum::ExecutionLayer);
    prevent_request_timeout_on_proxy_with_known_state_machine(direct_conn_state_machine).unwrap()
}

#[test]
fn should_prevent_request_processing_through_proxy_with_known_state_machine() {
    let direct_conn_state_machine = StateMachine::Ethereum(Ethereum::ExecutionLayer);
    prevent_request_processing_on_proxy_with_known_state_machine(direct_conn_state_machine).unwrap()
}

#[test]
fn should_prevent_response_timeout_on_proxy_with_known_state_machine() {
    let direct_conn_state_machine = StateMachine::Ethereum(Ethereum::ExecutionLayer);
    prevent_response_timeout_on_proxy_with_known_state_machine(direct_conn_state_machine).unwrap()
}

#[test]
fn should_prevent_request_processing_when_proof_metadata_is_mismatched() {
    check_request_source_and_destination().unwrap()
}

#[test]
fn should_prevent_response_processing_when_proof_metadata_is_mismatched() {
    check_response_source().unwrap()
}
