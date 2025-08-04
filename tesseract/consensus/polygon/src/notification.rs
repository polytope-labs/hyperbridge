use crate::{ConsensusState, PolygonConsensusUpdate, PolygonPosHost};
use codec::Decode;
use cometbft::merkle::proof::ProofOps;
use ibc::core::commitment_types::{
	commitment::CommitmentProofBytes, merkle::MerkleProof, proto::v1::MerkleProof as RawMerkleProof,
};
use ics23::CommitmentProof;
use log::trace;
use std::{result::Result::Ok, sync::Arc, vec::Vec};
use tendermint_ics23_primitives::ICS23HostFunctions;
use tendermint_primitives::{
	Client, CodecConsensusProof, CodecTrustedState, ConsensusProof, TrustedState, ValidatorSet,
};
use tendermint_verifier::validate_validator_set_hash;
use tesseract_primitives::IsmpProvider;

/// Notification logic for Polygon POS relayer
pub async fn consensus_notification(
	client: &PolygonPosHost,
	_counterparty: Arc<dyn IsmpProvider>,
) -> anyhow::Result<Option<PolygonConsensusUpdate>> {
	let latest_height = client.prover.latest_height().await?;

	trace!("latest_height: {:?}", latest_height);

	let consensus_state_serialized: Vec<u8> =
		_counterparty.query_consensus_state(None, client.consensus_state_id).await?;

	let consensus_state: ConsensusState =
		ConsensusState::decode(&mut &consensus_state_serialized[..])?;

	let trusted_state: TrustedState =
		CodecTrustedState::decode(&mut consensus_state.tendermint_state.as_slice())?.into();

	trace!("trusted_state height: {:?}", trusted_state.height);

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

	let maybe_milestone_update = if milestone.end_block > consensus_state.last_finalized_block {
		Some(
			build_milestone_update(
				client,
				milestone_number,
				milestone.clone(),
				milestone.end_block,
				untrusted_header.header.height.into(),
			)
			.await?,
		)
	} else {
		None
	};

	match validator_set_hash_match.is_ok() || next_validator_set_hash_match.is_ok() {
		true => {
			let ancestry = if latest_height > trusted_state.height + 1 {
				client
					.prover
					.signed_headers_range(trusted_state.height + 1, latest_height - 1)
					.await?
			} else {
				Vec::new()
			};

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

				trace!("3. Validator set hash match: {:?}", validator_set_hash_match);
				trace!("4. Next validator set hash match: {:?}", next_validator_set_hash_match);
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
				let ancestry = if matched_height > trusted_state.height + 1 {
					client
						.prover
						.signed_headers_range(trusted_state.height + 1, matched_height - 1)
						.await?
				} else {
					Vec::new()
				};
				let next_validators = client.prover.next_validators(matched_height).await?;
				return Ok(Some(PolygonConsensusUpdate {
					tendermint_proof: CodecConsensusProof::from(&ConsensusProof::new(
						matched_header,
						ancestry,
						Some(next_validators),
					)),
					milestone_update: None,
				}));
			}
		},
	}
	Ok(None)
}

async fn build_milestone_update(
	client: &PolygonPosHost,
	milestone_number: u64,
	milestone: ismp_polygon::Milestone,
	milestone_end_block: u64,
	untrusted_header_height: u64,
) -> anyhow::Result<ismp_polygon::MilestoneUpdate> {
	let evm_header = client
		.prover
		.fetch_header(milestone_end_block)
		.await?
		.ok_or_else(|| anyhow::anyhow!("EVM header not found"))?;
	let response = client.prover.get_ics23_proof(milestone_number, untrusted_header_height).await?;
	let merkle_proof = response
		.clone()
		.proof
		.map(|p| convert_tm_to_ics_merkle_proof::<ICS23HostFunctions>(&p))
		.transpose()
		.map_err(|_| anyhow::anyhow!("bad client state proof"))?
		.ok_or_else(|| anyhow::anyhow!("proof not found"))?;

	let proof = CommitmentProofBytes::try_from(merkle_proof)
		.map_err(|e| anyhow::anyhow!("bad client state proof: {}", e))?;

	return Ok(ismp_polygon::MilestoneUpdate {
		evm_header,
		milestone_number,
		ics23_state_proof: proof.into(),
		milestone,
	});
}

pub fn convert_tm_to_ics_merkle_proof<H>(
	tm_proof: &ProofOps,
) -> Result<MerkleProof, anyhow::Error> {
	let mut proofs = Vec::new();

	for op in &tm_proof.ops {
		let mut parse = CommitmentProof { proof: None };
		prost::Message::merge(&mut parse, op.data.as_slice())
			.map_err(|e| anyhow::anyhow!("commitment proof decoding failed: {}", e))?;

		proofs.push(parse);
	}
	let raw_merkle_proof = RawMerkleProof { proofs };

	let merkle_proof = MerkleProof::try_from(raw_merkle_proof)
		.map_err(|e| anyhow::anyhow!("bad client state proof: {}", e))?;

	Ok(merkle_proof)
}
