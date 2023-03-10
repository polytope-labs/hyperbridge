use crate::consensus_client::StateMachineHeight;
use crate::error::Error;
use crate::handlers::consensus_message::handle_consensus_message;
use crate::handlers::req_res::{handle_request_message, handle_response};
use crate::host::ISMPHost;
use crate::messaging::Message;

mod consensus_message;
mod req_res;

pub fn handle_incoming_message(host: &dyn ISMPHost, message: Message) -> Result<(), Error> {
    match message {
        Message::Consensus(consensus_message) => handle_consensus_message(host, consensus_message),
        Message::Request(req) => handle_request_message(host, req),
        Message::Response(resp) => handle_response(host, resp),
    }
}

fn verify_delay_passed(
    host: &dyn ISMPHost,
    proof_height: StateMachineHeight,
) -> Result<bool, Error> {
    let update_time = host.state_machine_update_time(proof_height)?;
    let delay_period = host.delay_period(proof_height.id);
    let current_timestamp = host.host_timestamp();
    Ok(current_timestamp - update_time > delay_period)
}
