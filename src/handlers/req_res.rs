use crate::consensus_client::ConsensusClient;
use crate::error::Error;
use crate::handlers::verify_delay_passed;
use crate::host::ISMPHost;
use crate::messaging::{Proof, RequestMessage, ResponseMessage};
use crate::paths::{RequestPath, ResponsePath};
use alloc::boxed::Box;
use alloc::string::ToString;

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
        return Err(Error::FrozenConsensusClient {
            id: consensus_client_id,
        });
    }

    // Ensure state machine is not frozen
    if host.is_frozen(proof.height)? {
        return Err(Error::FrozenStateMachine {
            height: proof.height,
        });
    }

    // Ensure delay period has elapsed
    if !verify_delay_passed(host, proof.height)? {
        return Err(Error::DelayNotElapsed {
            current_time: host.host_timestamp(),
            update_time: host.consensus_update_time(proof.height.id.consensus_client)?,
        });
    }

    Ok(consensus_client)
}

/// Validate the state machine, verify the request message and dispatch the message to the router
pub fn handle_request_message(host: &dyn ISMPHost, msg: RequestMessage) -> Result<(), Error> {
    let consensus_client = validate_state_machine(host, &msg.proof)?;
    let commitment = host.get_request_commitment(&msg.request);
    // Verify membership proof
    let key = RequestPath {
        dest_chain: msg.request.dest_chain,
        source_chain: msg.request.source_chain,
        nonce: msg.request.nonce,
    }
    .to_string()
    .as_bytes()
    .to_vec();
    consensus_client.verify_membership(host, key, commitment)?;

    let router = host.ismp_router();

    router.dispatch(msg.request)?;

    Ok(())
}

/// Validate the state machine, verify the response message and dispatch the message to the router
pub fn handle_response_message(host: &dyn ISMPHost, msg: ResponseMessage) -> Result<(), Error> {
    let consensus_client = validate_state_machine(host, &msg.proof)?;
    // For a response to be valid a request commitment must be present in storage
    let commitment = host.request_commitment(&msg.response.request)?;

    if commitment != host.get_request_commitment(&msg.response.request) {
        return Err(Error::RequestCommitmentNotFound {
            nonce: msg.response.request.nonce,
            source: msg.response.request.source_chain,
            dest: msg.response.request.dest_chain,
        });
    }

    let commitment = host.get_response_commitment(&msg.response);
    let key = ResponsePath {
        dest_chain: msg.response.request.source_chain,
        source_chain: msg.response.request.dest_chain,
        nonce: msg.response.request.nonce,
    }
    .to_string()
    .as_bytes()
    .to_vec();
    // Verify membership proof
    consensus_client.verify_membership(host, key, commitment)?;

    let router = host.ismp_router();

    router.write_response(msg.response)?;

    Ok(())
}
