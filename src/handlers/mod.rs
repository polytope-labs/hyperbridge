use crate::consensus_client::StateMachineHeight;
use crate::error::Error;
use crate::handlers::consensus_message::handle_consensus_message;
use crate::handlers::req_res::{handle_request_message, handle_response_message};
use crate::host::ISMPHost;
use crate::messaging::Message;

mod consensus_message;
mod req_res;

/// This function serves as an entry point to handle the message types provided by the ISMP protocol
/// Does not handle create consensus client message.
pub fn handle_incoming_message(host: &dyn ISMPHost, message: Message) -> Result<(), Error> {
    match message {
        Message::Consensus(consensus_message) => handle_consensus_message(host, consensus_message),
        Message::Request(req) => handle_request_message(host, req),
        Message::Response(resp) => handle_response_message(host, resp),
        _ => Err(Error::CannotHandleConsensusMessage),
    }
}

/// This function checks to see that the delay period configured on the host chain
/// for the state machine has elasped.
fn verify_delay_passed(
    host: &dyn ISMPHost,
    proof_height: StateMachineHeight,
) -> Result<bool, Error> {
    let update_time = host.consensus_update_time(proof_height.id.consensus_client)?;
    let delay_period = host.delay_period(proof_height.id.consensus_client);
    let current_timestamp = host.host_timestamp();
    Ok(current_timestamp - update_time > delay_period)
}
