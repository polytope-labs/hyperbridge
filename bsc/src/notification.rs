use bsc_prover::UpdateParams;
use codec::{Decode, Encode};
use ismp::messaging::{ConsensusMessage, Message};
use ismp_bsc::ConsensusState;

use bsc_verifier::primitives::{compute_epoch, BscClientUpdate, Config};
use sp_core::H160;

use std::{cmp::max, sync::Arc};
use tesseract_primitives::IsmpProvider;

use crate::{BscPosHost, KeccakHasher};

pub async fn consensus_notification<C: Config>(
	client: &BscPosHost<C>,
	counterparty: Arc<dyn IsmpProvider>,
) -> Result<(Option<BscClientUpdate>, Option<ConsensusState>), anyhow::Error> {
	let counterparty_finalized = counterparty.query_finalized_height().await?;
	let consensus_state = counterparty
		.query_consensus_state(Some(counterparty_finalized), client.consensus_state_id)
		.await?;
	let epoch_length = client.host.epoch_length;
	let consensus_state = ConsensusState::decode(&mut &*consensus_state)?;
	let current_epoch = max(
		compute_epoch(consensus_state.finalized_height, epoch_length),
		consensus_state.current_epoch,
	);
	let attested_header = client.prover.latest_header().await?;

	let attested_epoch = compute_epoch(attested_header.number.low_u64(), epoch_length);
	if attested_epoch < current_epoch ||
		attested_epoch > current_epoch ||
		consensus_state.finalized_height >= attested_header.number.low_u64()
	{
		return Ok((None, None));
	}

	let mut bsc_client_update = client
		.prover
		.fetch_bsc_update::<KeccakHasher>(UpdateParams {
			attested_header,
			epoch_length,
			epoch: current_epoch,
			fetch_val_set_change: false,
			validator_size: consensus_state.current_validators.len() as u64,
		})
		.await?;
	// Dry run the update so we know it will succeed, this ensures client does not get stalled
	// If the update is a None value, we want to try again in the next tick
	let dry_run_result = if let Some(update) = bsc_client_update.as_ref() {
		let msg = ConsensusMessage {
			consensus_proof: update.encode(),
			consensus_state_id: client.consensus_state_id,
			signer: H160::random().0.to_vec(),
		};
		let res = counterparty.estimate_gas(vec![Message::Consensus(msg)]).await?[0];
		res.successful_execution
	} else {
		false
	};
	let cs_state = dry_run_result.then(|| consensus_state);
	// If the dry run failed, we skip the update
	if !dry_run_result {
		log::info!(target: "tesseract", "Skipping invalid update in bsc client");
		bsc_client_update = None
	}
	return Ok((bsc_client_update, cs_state));
}
