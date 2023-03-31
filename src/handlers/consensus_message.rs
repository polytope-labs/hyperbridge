use crate::{
    error::Error,
    handlers::{ConsensusUpdateResult, MessageResult},
    host::ISMPHost,
    messaging::ConsensusMessage,
};
use alloc::collections::BTreeSet;

/// This function handles verification of consensus messages for consensus clients
/// It is up to the consensus client implementation to check for frozen if the consensus state
/// is frozen or expired
/// The client implementation can choose to deposit an event compatible with the host platform on
/// successful verification
pub fn handle_consensus_message(
    host: &dyn ISMPHost,
    msg: ConsensusMessage,
) -> Result<MessageResult, Error> {
    let consensus_client = host.consensus_client(msg.consensus_client_id)?;
    let trusted_state = host.consensus_state(msg.consensus_client_id)?;

    let (new_state, intermediate_states) =
        consensus_client.verify(host, trusted_state, msg.consensus_proof)?;
    host.store_consensus_state(msg.consensus_client_id, new_state)?;
    let timestamp = host.host_timestamp();
    host.store_consensus_update_time(msg.consensus_client_id, timestamp)?;
    let mut state_updates = BTreeSet::new();
    for intermediate_state in intermediate_states {
        // If a state machine is frozen, we skip it
        if host.is_frozen(intermediate_state.height)? {
            continue
        }

        let previous_latest_height = host.latest_commitment_height(intermediate_state.height.id)?;

        // Only allow heights greater than latest height
        if previous_latest_height > intermediate_state.height {
            continue
        }

        // Skip duplicate states
        if host.state_machine_commitment(intermediate_state.height).is_ok() {
            continue
        }

        host.store_state_machine_commitment(
            intermediate_state.height,
            intermediate_state.commitment,
        )?;

        state_updates.insert((previous_latest_height, intermediate_state.height));
        host.store_latest_commitment_height(intermediate_state.height)?;
    }

    let result =
        ConsensusUpdateResult { consensus_client_id: msg.consensus_client_id, state_updates };

    Ok(MessageResult::ConsensusMessage(result))
}
