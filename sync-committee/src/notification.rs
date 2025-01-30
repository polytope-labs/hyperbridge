use crate::{L2Host, SyncCommitteeHost};
use codec::Decode;
use ismp::{consensus::StateMachineId, host::StateMachine};
use ismp_sync_committee::types::{BeaconClientUpdate, ConsensusState, L2Consensus};
use std::{collections::BTreeMap, sync::Arc};
use sync_committee_primitives::{
	consensus_types::Checkpoint,
	constants::{Config, Root},
	types::VerifierStateUpdate,
};
use tesseract_primitives::IsmpProvider;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EventResponse {
	pub block: Root,
	pub state: Root,
	pub epoch: String,
	pub execution_optimistic: bool,
}

pub async fn get_beacon_update<
	T: Config + Send + Sync + 'static,
	const ETH1_DATA_VOTES_BOUND: usize,
>(
	client: &SyncCommitteeHost<T, ETH1_DATA_VOTES_BOUND>,
	l2_consensus: BTreeMap<StateMachine, L2Consensus>,
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

	let mut l2_oracle_payload = BTreeMap::new();
	let mut dispute_game_payload = BTreeMap::new();
	let mut arbitrum_payload = BTreeMap::new();
	let mut arbitrum_bold = BTreeMap::new();

	for (state_machine, consensus_mechanic) in l2_consensus {
		if let Some(client) = client.l2_clients.get(&state_machine) {
			match (client, consensus_mechanic.clone()) {
				(L2Host::ArbitrumOrbit(orbit_client), L2Consensus::ArbitrumOrbit(_)) => {
					let latest_event = orbit_client
						.latest_event(
							latest_height,
							consensus_update.execution_payload.block_number,
						)
						.await?;
					if let Some(event) = latest_event {
						let payload = orbit_client
							.fetch_arbitrum_payload(
								consensus_update.execution_payload.block_number,
								event,
							)
							.await?;
						arbitrum_payload.insert(state_machine, payload);
					}
				},

				(L2Host::ArbitrumOrbit(orbit_client), L2Consensus::ArbitrumBold(_)) => {
					let latest_event = orbit_client
						.latest_assertion_event(
							latest_height,
							consensus_update.execution_payload.block_number,
						)
						.await?;
					if let Some(event) = latest_event {
						let payload = orbit_client
							.fetch_arbitrum_bold_payload(
								consensus_update.execution_payload.block_number,
								event,
							)
							.await?;
						arbitrum_bold.insert(state_machine, payload);
					}
				},
				(L2Host::OpStack(op_client), L2Consensus::OpL2Oracle(_)) => {
					let latest_event = op_client
						.latest_event(
							latest_height,
							consensus_update.execution_payload.block_number,
						)
						.await?;
					if let Some(event) = latest_event {
						let payload = op_client
							.fetch_op_payload(
								consensus_update.execution_payload.block_number,
								event,
							)
							.await?;
						l2_oracle_payload.insert(state_machine, payload);
					}
				},
				(
					L2Host::OpStack(op_client),
					L2Consensus::OpFaultProofs((_, respected_game_type)),
				) => {
					let latest_events = op_client
						.latest_dispute_games(
							latest_height,
							consensus_update.execution_payload.block_number,
						)
						.await?;
					let payload = op_client
						.fetch_dispute_game_payload(
							consensus_update.execution_payload.block_number,
							vec![respected_game_type],
							latest_events,
						)
						.await?;
					if let Some(payload) = payload {
						dispute_game_payload.insert(state_machine, payload);
					}
				},
				(
					L2Host::OpStack(op_client),
					L2Consensus::OpFaultProofGames((_, respected_game_types)),
				) => {
					let latest_events = op_client
						.latest_dispute_games(
							latest_height,
							consensus_update.execution_payload.block_number,
						)
						.await?;
					let payload = op_client
						.fetch_dispute_game_payload(
							consensus_update.execution_payload.block_number,
							respected_game_types,
							latest_events,
						)
						.await?;
					if let Some(payload) = payload {
						dispute_game_payload.insert(state_machine, payload);
					}
				},
				_ => {
					log::warn!("Invalid combination of l2 consensus for {state_machine} {consensus_mechanic:?}");
					continue;
				},
			}
		}
	}

	let message = BeaconClientUpdate {
		consensus_update,
		l2_oracle_payload,
		arbitrum_payload,
		dispute_game_payload,
		arbitrum_bold,
	};
	Ok(message)
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

	let message = get_beacon_update(
		client,
		consensus_state.l2_consensus,
		consensus_update,
		execution_layer_height,
	)
	.await?;
	Ok(Some(message))
}
