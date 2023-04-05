use crate::{
    consensus_client::ConsensusClient,
    error::Error,
    handlers::{verify_delay_passed, MessageResult, RequestResponseResult},
    host::ISMPHost,
    messaging::{Proof, RequestMessage, ResponseMessage},
    router::RequestResponse,
};
use alloc::boxed::Box;

/// This function does the preliminary checks for a request or response message
/// - It ensures the consensus client is not frozen
/// - It ensures the state machine is not frozen
/// - Checks that the delay period configured for the state machine has elaspsed.
fn validate_state_machine(
    host: &dyn ISMPHost,
    proof: &Proof,
) -> Result<Box<dyn ConsensusClient>, Error> {
    // Ensure consensus client is not frozen
    let consensus_client_id = proof.height.id.consensus_client;
    let consensus_client = host.consensus_client(consensus_client_id)?;
    if consensus_client.is_frozen(host, consensus_client_id)? {
        return Err(Error::FrozenConsensusClient { id: consensus_client_id })
    }

    // Ensure state machine is not frozen
    if host.is_frozen(proof.height)? {
        return Err(Error::FrozenStateMachine { height: proof.height })
    }

    // Ensure delay period has elapsed
    if !verify_delay_passed(host, proof.height)? {
        return Err(Error::DelayNotElapsed {
            current_time: host.host_timestamp(),
            update_time: host.consensus_update_time(proof.height.id.consensus_client)?,
        })
    }

    Ok(consensus_client)
}

/// Validate the state machine, verify the request message and dispatch the message to the router
pub fn handle_request_message(
    host: &dyn ISMPHost,
    msg: RequestMessage,
) -> Result<MessageResult, Error> {
    let consensus_client = validate_state_machine(host, &msg.proof)?;
    // Verify membership proof
    let state = host.state_machine_commitment(msg.proof.height)?;
    consensus_client.verify_membership(
        host,
        RequestResponse::Request(msg.request.clone()),
        state.commitment_root,
        &msg.proof,
    )?;

    let router = host.ismp_router();

    let result = RequestResponseResult {
        dest_chain: msg.request.dest_chain,
        source_chain: msg.request.source_chain,
        nonce: msg.request.nonce,
    };

    router.dispatch(msg.request)?;

    Ok(MessageResult::Request(result))
}

/// Validate the state machine, verify the response message and dispatch the message to the router
pub fn handle_response_message(
    host: &dyn ISMPHost,
    msg: ResponseMessage,
) -> Result<MessageResult, Error> {
    let consensus_client = validate_state_machine(host, &msg.proof)?;
    // For a response to be valid a request commitment must be present in storage
    let commitment = host.request_commitment(&msg.response.request)?;

    if commitment != host.get_request_commitment(&msg.response.request) {
        return Err(Error::RequestCommitmentNotFound {
            nonce: msg.response.request.nonce,
            source: msg.response.request.source_chain,
            dest: msg.response.request.dest_chain,
        })
    }

    let state = host.state_machine_commitment(msg.proof.height)?;
    // Verify membership proof
    consensus_client.verify_membership(
        host,
        RequestResponse::Response(msg.response.clone()),
        state.commitment_root,
        &msg.proof,
    )?;

    let router = host.ismp_router();

    let result = RequestResponseResult {
        dest_chain: msg.response.request.source_chain,
        source_chain: msg.response.request.dest_chain,
        nonce: msg.response.request.nonce,
    };

    router.write_response(msg.response)?;

    Ok(MessageResult::Response(result))
}
