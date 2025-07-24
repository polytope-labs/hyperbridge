use crate::{ConsensusState, PolygonConsensusUpdate, PolygonPosHost};
use anyhow::Ok as anyhow_Ok;
use codec::Decode;
use std::{result::Result::Ok, sync::Arc};
use tendermint_primitives::{CodecConsensusProof, CodecTrustedState, ConsensusProof, TrustedState};
use tendermint_prover::{Client, ValidatorSet};
use tendermint_verifier::validate_validator_set_hash;
use tesseract_primitives::IsmpProvider;

/// Notification logic for Polygon POS relayer
pub async fn consensus_notification(
	client: &PolygonPosHost,
	_counterparty: Arc<dyn IsmpProvider>,
) -> anyhow::Result<(Option<PolygonConsensusUpdate>)> {
	let latest_height = client.prover.latest_height().await?;

	let consensus_state_serialized: Vec<u8> =
		_counterparty.query_consensus_state(None, client.consensus_state_id).await?;

	let consensus_state: ConsensusState =
		ConsensusState::decode(&mut &consensus_state_serialized[..])?;

	let trusted_state: TrustedState =
		CodecTrustedState::decode(&mut consensus_state.tendermint_state.as_slice())?
			.to_trusted_state()
			.map_err(|e| anyhow::Error::msg(e))?;

	let untrusted_header = client.prover.signed_header(latest_height).await?;

	let validator_set_hash_match = validate_validator_set_hash(
		&ValidatorSet::new(trusted_state.validators.clone(), None),
		untrusted_header.header.validators_hash,
		false,
	);

	let next_validator_set_hash_match = validate_validator_set_hash(
		&ValidatorSet::new(trusted_state.next_validators.clone(), None),
		untrusted_header.header.next_validators_hash,
		true,
	);

	let (milestone_number, milestone) = client.prover.get_latest_milestone().await?;
	let milestone_end_block = milestone.end_block.parse::<u64>().unwrap_or(0);

	let maybe_milestone_update = if milestone_end_block > consensus_state.last_finalized_block {
		Some(
			build_milestone_update(
				client,
				milestone_number,
				milestone.clone(),
				milestone_end_block,
			)
			.await?,
		)
	} else {
		None
	};

	match validator_set_hash_match.is_ok() || next_validator_set_hash_match.is_ok() {
		true => {
			let ancestry = client
				.prover
				.signed_headers_range(trusted_state.height + 1, latest_height)
				.await?;
			let next_validators = client.prover.next_validators(latest_height).await?;
			return Ok(Some(PolygonConsensusUpdate {
				tendermint_proof: CodecConsensusProof::from(&ConsensusProof::new(
					untrusted_header,
					ancestry,
					Some(next_validators),
				)),
				milestone_update: maybe_milestone_update,
			}));
		},
		false => {
			// Backward traversal
			let mut height = latest_height;
			let mut found = false;
			let mut matched_header = None;
			while height > trusted_state.height {
				let header_res = client.prover.signed_header(height).await;
				let header = match header_res {
					Ok(h) => h,
					Err(_) => {
						height -= 1;
						continue;
					},
				};
				let validator_set_hash_match = validate_validator_set_hash(
					&ValidatorSet::new(trusted_state.validators.clone(), None),
					header.header.validators_hash,
					false,
				);
				let next_validator_set_hash_match = validate_validator_set_hash(
					&ValidatorSet::new(trusted_state.next_validators.clone(), None),
					header.header.next_validators_hash,
					true,
				);
				if validator_set_hash_match.is_ok() || next_validator_set_hash_match.is_ok() {
					found = true;
					matched_header = Some(header);
					break;
				}
				height -= 1;
			}
			if found {
				let matched_height = height;
				let matched_header = matched_header.expect("Header must be present if found");
				let ancestry = client
					.prover
					.signed_headers_range(trusted_state.height + 1, matched_height)
					.await?;
				let next_validators = client.prover.next_validators(matched_height).await?;
				return Ok(Some(PolygonConsensusUpdate {
					tendermint_proof: CodecConsensusProof::from(&ConsensusProof::new(
						matched_header,
						ancestry,
						Some(next_validators),
					)),
					milestone_update: maybe_milestone_update,
				}));
			}
		},
	}
	Ok((None))
}

async fn build_milestone_update(
	client: &PolygonPosHost,
	milestone_number: u64,
	milestone: tendermint_primitives::Milestone,
	milestone_end_block: u64,
) -> anyhow::Result<ismp_polygon::MilestoneUpdate> {
	let evm_header = client
		.prover
		.fetch_header(milestone_end_block)
		.await?
		.ok_or_else(|| anyhow::anyhow!("EVM header not found"))?;
	let abci_query = client.prover.get_ics23_proof(milestone_number, milestone_end_block).await?;
	let ics23_state_proof = abci_query
		.proof
		.as_ref()
		.and_then(|p| p.ops.get(0))
		.map(|op| op.data.clone())
		.unwrap_or_default();
	return Ok(ismp_polygon::MilestoneUpdate {
		evm_header,
		milestone_number: Some(milestone_number),
		ics23_state_proof,
		milestone,
	});
}
