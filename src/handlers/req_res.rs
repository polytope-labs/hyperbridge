use crate::consensus_client::ConsensusClient;
use crate::error::Error;
use crate::handlers::verify_delay_passed;
use crate::host::ISMPHost;
use crate::messaging::{Proof, RequestMessage, ResponseMessage};
use alloc::boxed::Box;
use codec::Encode;

fn validate_state_machine(
    host: &dyn ISMPHost,
    proof: &Proof,
) -> Result<Box<dyn ConsensusClient>, Error> {
    // Ensure consensus client is not frozen
    let consensus_client_id = host.client_id_from_state_id(proof.height.id)?;
    let consensus_client = host.consensus_client(consensus_client_id)?;
    if consensus_client.is_frozen(host, consensus_client_id)? {
        return Err(Error::FrozenConsensusClient);
    }

    // Ensure state machine is not frozen
    if host.is_frozen(proof.height)? {
        return Err(Error::FrozenStateMachine);
    }

    // Ensure delay period has elapsed
    if !verify_delay_passed(host, proof.height)? {
        return Err(Error::DelayNotElapsed);
    }

    Ok(consensus_client)
}

pub fn handle_request_message(host: &dyn ISMPHost, msg: RequestMessage) -> Result<(), Error> {
    let consensus_client = validate_state_machine(host, &msg.proof)?;
    let encoded_request = msg.request.encode();
    let commitment = host.sha256(&*encoded_request);
    // Verify membership proof
    consensus_client.verify_membership(host, &commitment[..], msg.proof)?;

    let router = host.ismp_router();

    router.dispatch(msg.request)?;

    // Return some event
    Ok(())
}

pub fn handle_response(host: &dyn ISMPHost, msg: ResponseMessage) -> Result<(), Error> {
    let consensus_client = validate_state_machine(host, &msg.proof)?;
    let encoded_resp = msg.response.encode();
    let commitment = host.sha256(&*encoded_resp);
    // Verify membership proof
    consensus_client.verify_membership(host, &commitment[..], msg.proof)?;

    let router = host.ismp_router();

    router.write_response(msg.response)?;

    // Return some event
    Ok(())
}
