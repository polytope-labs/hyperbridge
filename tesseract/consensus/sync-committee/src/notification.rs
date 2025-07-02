use crate::SyncCommitteeHost;
use codec::Decode;
use ismp::consensus::StateMachineId;
use ismp_sync_committee::types::{BeaconClientUpdate, ConsensusState};
use std::sync::Arc;
use sync_committee_primitives::{
	consensus_types::Checkpoint,
	constants::{Config, Root},
};
use tesseract_primitives::IsmpProvider;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EventResponse {
	pub block: Root,
	pub state: Root,
	pub epoch: String,
	pub execution_optimistic: bool,
}

pub async fn consensus_notification<
	T: Config + Send + Sync + 'static,
	const ETH1_DATA_VOTES_BOUND: usize,
>(
	client: &SyncCommitteeHost<T, ETH1_DATA_VOTES_BOUND>,
	counterparty: Arc<dyn IsmpProvider>,
	checkpoint: Checkpoint,
) -> Result<Option<BeaconClientUpdate>, anyhow::Error> {
	let consensus_state =
		counterparty.query_consensus_state(None, client.consensus_state_id).await?;
	let consensus_state = ConsensusState::decode(&mut &*consensus_state)?;
	let light_client_state = consensus_state.light_client_state;
	let state_machine_id = StateMachineId {
		state_id: client.state_machine,
		consensus_state_id: client.consensus_state_id,
	};
	let update = client
		.prover
		.fetch_light_client_update(light_client_state.clone(), checkpoint.clone(), None)
		.await?;

	let execution_layer_height = counterparty.query_latest_height(state_machine_id).await? as u64;

	let consensus_update = if let Some(update) = update {
		update
	} else {
		return Ok(None);
	};

	if consensus_update.execution_payload.block_number <= execution_layer_height &&
		consensus_update.sync_committee_update.is_none() ||
		consensus_update.attested_header.slot <= light_client_state.finalized_header.slot
	{
		return Ok(None);
	}

	let message = BeaconClientUpdate { consensus_update };

	Ok(Some(message))
}
