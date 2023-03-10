use crate::consensus_client::ConsensusClient;
use crate::error::Error;
use crate::handlers::verify_delay_passed;
use crate::host::ISMPHost;
use crate::messaging::{Proof, RequestMessage, ResponseMessage};
use alloc::boxed::Box;

fn validate_state_machine(
    host: &dyn ISMPHost,
    proof: &Proof,
) -> Result<Box<dyn ConsensusClient>, Error> {
    // Ensure consensus client is not frozen
    let consensus_client_id = host.client_id_from_state_id(proof.height.id)?;
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
            update_time: host.state_machine_update_time(proof.height)?,
        });
    }

    Ok(consensus_client)
}

pub fn handle_request_message(host: &dyn ISMPHost, msg: RequestMessage) -> Result<(), Error> {
    let consensus_client = validate_state_machine(host, &msg.proof)?;
    let commitment = host.get_request_commitment(&msg.request);
    // Verify membership proof
    consensus_client.verify_request_membership(host, &commitment[..], &msg)?;

    let router = host.ismp_router();

    router.dispatch(msg.request)?;

    Ok(())
}

pub fn handle_response(host: &dyn ISMPHost, msg: ResponseMessage) -> Result<(), Error> {
    let consensus_client = validate_state_machine(host, &msg.proof)?;
    // If host chain is the destination of the response, check if a request commitment exists
    if host.host() == msg.response.request.source_chain {
        let commitment = host.get_request_commitment(&msg.response.request);
        if commitment != host.request_commitment(&msg.response.request)? {
            return Err(Error::RequestCommitmentNotFound {
                nonce: msg.response.request.nonce,
                source: msg.response.request.source_chain,
                dest: msg.response.request.dest_chain,
            });
        }
    }

    let commitment = host.get_response_commitment(&msg.response);
    // Verify membership proof
    consensus_client.verify_response_membership(host, &commitment[..], &msg)?;

    let router = host.ismp_router();

    router.write_response(msg.response)?;

    Ok(())
}
