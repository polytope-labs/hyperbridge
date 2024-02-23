use crate::SyncCommitteeHost;
use codec::Decode;
use ismp::{
	consensus::StateMachineId,
	host::{Ethereum, StateMachine},
};
use ismp_sync_committee::types::{BeaconClientUpdate, ConsensusState};
use std::collections::BTreeMap;
use sync_committee_primitives::{
	consensus_types::Checkpoint,
	constants::{Config, Root},
	types::VerifierStateUpdate,
};
use tesseract_primitives::{IsmpHost, IsmpProvider};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EventResponse {
	pub block: Root,
	pub state: Root,
	pub epoch: String,
	pub execution_optimistic: bool,
}

pub async fn get_beacon_update<T: Config + Send + Sync + 'static>(
	client: &SyncCommitteeHost<T>,
	consensus_update: VerifierStateUpdate,
	execution_layer_height: u64,
) -> Result<BeaconClientUpdate, anyhow::Error> {
	let latest_height = {
		if execution_layer_height == 0 {
			// Check the past 10 blocks behind latest execution layer block number
			consensus_update.execution_payload.block_number.saturating_sub(10)
		} else {
			execution_layer_height
		}
	};

	let arbitrum_payload = if let Some(arb_client) = client.arbitrum_client.as_ref() {
		let latest_event = arb_client
			.latest_event(latest_height, consensus_update.execution_payload.block_number)
			.await?;
		if let Some(event) = latest_event {
			let payload = arb_client
				.fetch_arbitrum_payload(consensus_update.execution_payload.block_number, event)
				.await?;
			Some(payload)
		} else {
			None
		}
	} else {
		None
	};

	let mut op_stack_payload = BTreeMap::new();
	if let Some(op_client) = client.optimism_client.as_ref() {
		let latest_event = op_client
			.latest_event(latest_height, consensus_update.execution_payload.block_number)
			.await?;
		if let Some(event) = latest_event {
			let payload = op_client
				.fetch_op_payload(consensus_update.execution_payload.block_number, event)
				.await?;
			op_stack_payload.insert(StateMachine::Ethereum(Ethereum::Optimism), payload);
		}
	}

	if let Some(base_client) = client.base_client.as_ref() {
		let latest_event = base_client
			.latest_event(latest_height, consensus_update.execution_payload.block_number)
			.await?;
		if let Some(event) = latest_event {
			let payload = base_client
				.fetch_op_payload(consensus_update.execution_payload.block_number, event)
				.await?;
			op_stack_payload.insert(StateMachine::Ethereum(Ethereum::Base), payload);
		}
	}

	let message = BeaconClientUpdate { consensus_update, op_stack_payload, arbitrum_payload };
	Ok(message)
}

pub async fn consensus_notification<C, T: Config + Send + Sync + 'static>(
	client: &SyncCommitteeHost<T>,
	counterparty: C,
	checkpoint: Checkpoint,
) -> Result<Option<BeaconClientUpdate>, anyhow::Error>
where
	C: IsmpHost + IsmpProvider + 'static,
{
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
		.fetch_light_client_update(
			light_client_state.clone(),
			checkpoint.clone(),
			None,
			"tesseract",
		)
		.await?;

	let execution_layer_height = counterparty.query_latest_height(state_machine_id).await? as u64;

	let consensus_update = if let Some(update) = update { update } else { return Ok(None) };

	if consensus_update.execution_payload.block_number <= execution_layer_height &&
		consensus_update.sync_committee_update.is_none() ||
		consensus_update.attested_header.slot <= light_client_state.finalized_header.slot
	{
		return Ok(None)
	}

	let message = get_beacon_update(client, consensus_update, execution_layer_height).await?;
	Ok(Some(message))
}
