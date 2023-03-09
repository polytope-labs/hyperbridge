use crate::error::Error;
use crate::host::ISMPHost;
use crate::messaging::ConsensusMessage;

pub fn handle_consensus_message(host: &dyn ISMPHost, msg: ConsensusMessage) -> Result<(), Error> {
    let consensus_client = host.consensus_client(msg.consensus_client_id)?;
    let trusted_state = host.consensus_state(msg.consensus_client_id)?;

    let (new_state, intermediate_states) =
        consensus_client.verify(host, trusted_state, msg.consensus_proof)?;
    host.store_consensus_state(msg.consensus_client_id, new_state)?;
    let timestamp = host.host_timestamp();
    host.store_consensus_update_time(msg.consensus_client_id, timestamp)?;
    for intermediate_state in intermediate_states {
        host.store_state_machine_commitment(
            intermediate_state.height,
            intermediate_state.commitment,
        )?;
        host.store_state_machine_update_time(intermediate_state.height, timestamp)?;
    }
    Ok(())
}
