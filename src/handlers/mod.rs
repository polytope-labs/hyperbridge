use crate::error::Error;
use crate::handlers::consensus_message::handle_consensus_message;
use crate::host::ISMPHost;
use crate::messaging::Message;

pub mod consensus_message;

pub fn handle_incoming_message(host: &dyn ISMPHost, message: Message) -> Result<(), Error> {
    match message {
        Message::Consensus(consensus_message) => handle_consensus_message(host, consensus_message),
        Message::Request(_) => unimplemented!(),
        Message::Response(_) => unimplemented!(),
    }
}
