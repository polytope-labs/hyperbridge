use crate::{Config, Event as PalletEvent};
use ismp_rust::{consensus_client::StateMachineId, host::ChainID};

#[derive(codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum Event {
    /// Event to be emitted when the challenge period for a state machine update has elapsed
    StateMachineUpdated {
        state_machine_id: StateMachineId,
        latest_height: u64,
        previous_height: u64,
    },
    Response {
        /// Chain that this response will be routed to
        dest_chain: ChainID,
        /// Source Chain for this response
        source_chain: ChainID,
        /// Nonce for the request which this response is for
        request_nonce: u64,
    },
    Request {
        /// Chain that this request will be routed to
        dest_chain: ChainID,
        /// Source Chain for request
        source_chain: ChainID,
        /// Request nonce
        request_nonce: u64,
    },
}

pub fn to_core_protocol_events<T: Config>(event: PalletEvent<T>) -> Option<Event> {
    match event {
        PalletEvent::StateMachineUpdated { state_machine_id, latest_height, previous_height } => {
            Some(Event::StateMachineUpdated { state_machine_id, latest_height, previous_height })
        }
        PalletEvent::Response { dest_chain, source_chain, request_nonce } => {
            Some(Event::Response { dest_chain, source_chain, request_nonce })
        }
        PalletEvent::Request { dest_chain, source_chain, request_nonce } => {
            Some(Event::Request { dest_chain, source_chain, request_nonce })
        }
        _ => None,
    }
}
