use crate::{
    consensus_client::{ConsensusClientId, StateMachineHeight},
    error::Error,
    handlers::{
        consensus_message::handle_consensus_message,
        req_res::{handle_request_message, handle_response_message},
    },
    host::{ChainID, ISMPHost},
    messaging::Message,
};
use alloc::collections::BTreeSet;

mod consensus_message;
mod req_res;

pub struct ConsensusUpdateResult {
    /// Consensus client Id
    pub consensus_client_id: ConsensusClientId,
    /// Tuple of previous latest height and new latest height for a state machine
    pub state_updates: BTreeSet<(StateMachineHeight, StateMachineHeight)>,
}

pub struct RequestResponseResult {
    /// Destination chain for request or response
    pub dest_chain: ChainID,
    /// Source chain for request or response
    pub source_chain: ChainID,
    /// Request nonce
    pub nonce: u64,
}

/// Result returned when ismp messages are handled successfully
pub enum MessageResult {
    ConsensusMessage(ConsensusUpdateResult),
    Request(RequestResponseResult),
    Response(RequestResponseResult),
}

/// This function serves as an entry point to handle the message types provided by the ISMP protocol
/// Does not handle create consensus client message.
pub fn handle_incoming_message(
    host: &dyn ISMPHost,
    message: Message,
) -> Result<MessageResult, Error> {
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
