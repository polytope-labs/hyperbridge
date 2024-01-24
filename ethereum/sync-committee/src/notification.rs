use crate::SyncCommitteeHost;
use codec::{Decode, Encode};
use ismp::{
	consensus::StateMachineId,
	host::{Ethereum, StateMachine},
	messaging::{ConsensusMessage, Message},
};
use ismp_sync_committee::types::{BeaconClientUpdate, ConsensusState};
use std::collections::BTreeMap;
use sync_committee_primitives::{
	consensus_types::Checkpoint,
	constants::{Config, Root},
	util::compute_sync_committee_period,
};
use tesseract_primitives::{IsmpHost, IsmpProvider};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EventResponse {
	pub block: Root,
	pub state: Root,
	pub epoch: String,
	pub execution_optimistic: bool,
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
	let mut consensus_state = ConsensusState::decode(&mut &*consensus_state)?;
	let mut light_client_state = consensus_state.light_client_state;
	let state_machine_id = StateMachineId {
		state_id: client.state_machine,
		consensus_state_id: client.consensus_state_id,
	};
	// Do a sync check before returning any updates
	let state_period = light_client_state.state_period;

	let checkpoint_period = compute_sync_committee_period::<T>(checkpoint.epoch);
	if !(state_period..=(state_period + 1)).contains(&checkpoint_period) {
		let mut next_period = state_period + 1;
		loop {
			if next_period >= checkpoint_period {
				break
			}
			let update = client.prover.latest_update_for_period(next_period).await?;
			let beacon_message = BeaconClientUpdate {
				consensus_update: update,
				op_stack_payload: Default::default(),
				arbitrum_payload: None,
			};
			let message = ConsensusMessage {
				consensus_proof: beacon_message.encode(),
				consensus_state_id: client.consensus_state_id,
			};

			counterparty.submit(vec![Message::Consensus(message)]).await?;
			next_period += 1;
		}
		// Query the new consensus state so we can process the latest finality checkpoint
		let new_consensus_state =
			counterparty.query_consensus_state(None, client.consensus_state_id).await?;
		consensus_state = ConsensusState::decode(&mut &*new_consensus_state)?;
		light_client_state = consensus_state.light_client_state;
	}
	let execution_layer_height = counterparty.query_latest_height(state_machine_id).await? as u64;
	let update = client
		.prover
		.fetch_light_client_update(
			light_client_state.clone(),
			checkpoint.clone(),
			None,
			"tesseract",
		)
		.await?;
	let consensus_update = if let Some(update) = update { update } else { return Ok(None) };

	if consensus_update.execution_payload.block_number <= execution_layer_height &&
		consensus_update.sync_committee_update.is_none()
	{
		return Ok(None)
	}

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

	Ok(Some(message))
}
