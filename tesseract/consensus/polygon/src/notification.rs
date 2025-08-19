use crate::{ConsensusState, PolygonConsensusUpdate, PolygonPosHost};
use codec::Decode;
use cometbft::merkle::proof::ProofOps;
use ibc::core::commitment_types::{
	commitment::CommitmentProofBytes, merkle::MerkleProof, proto::v1::MerkleProof as RawMerkleProof,
};
use ics23::CommitmentProof;
use std::{result::Result::Ok, sync::Arc, vec::Vec};
use tendermint_ics23_primitives::ICS23HostFunctions;
use tendermint_primitives::{
	Client, CodecConsensusProof, ConsensusProof, TrustedState, ValidatorSet,
};
use tendermint_verifier::validate_validator_set_hash;
use tesseract_primitives::IsmpProvider;

/// Notification logic for Polygon POS relayer
pub async fn consensus_notification(
	client: &PolygonPosHost,
	_counterparty: Arc<dyn IsmpProvider>,
) -> anyhow::Result<Option<PolygonConsensusUpdate>> {
	let latest_height = client.prover.latest_height().await?;

	let consensus_state_serialized: Vec<u8> =
		_counterparty.query_consensus_state(None, client.consensus_state_id).await?;

	let consensus_state: ConsensusState =
		ConsensusState::decode(&mut &consensus_state_serialized[..])?;

	let trusted_state: TrustedState = consensus_state.clone().tendermint_state.into();

	let untrusted_header = client.prover.signed_header(latest_height).await?;

	let validator_set_hash_match = validate_validator_set_hash(
		&ValidatorSet::new(trusted_state.validators.clone(), None),
		untrusted_header.header.validators_hash,
		false,
	);

	let next_validator_set_hash_match = validate_validator_set_hash(
		&ValidatorSet::new(trusted_state.next_validators.clone(), None),
		untrusted_header.header.validators_hash,
		true,
	);

	let maybe_milestone_update =
		build_milestone_update(client, untrusted_header.header.height.value(), &consensus_state)
			.await?;

	match validator_set_hash_match.is_ok() && next_validator_set_hash_match.is_ok() {
		true => {
			log::trace!(target: "tesseract", "Onchain Validator set matches signed header, constructing consensus proof");
			let next_validators = client.prover.next_validators(latest_height).await?;

			return Ok(Some(PolygonConsensusUpdate {
				tendermint_proof: CodecConsensusProof::from(&ConsensusProof::new(
					untrusted_header.clone(),
					if untrusted_header.header.next_validators_hash.is_empty() {
						None
					} else {
						Some(next_validators)
					},
				)),
				milestone_update: maybe_milestone_update,
			}));
		},
		false => {
			log::trace!(target: "tesseract", "No match found between onchain validator set latest header, will begin syncing");
			// Backward traversal
			let mut height = latest_height - 1;
			let mut matched_header = None;
			while height > trusted_state.height {
				log::trace!(target: "tesseract", "Checking for validator set match at {height}");
				let header_res = client.prover.signed_header(height).await;
				let header = match header_res {
					Ok(h) => h,
					Err(e) => {
						log::trace!(target: "tesseract", "Error fetching tendermint header for {height}, will retry \n {e:?}");
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
					header.header.validators_hash,
					true,
				);
				if validator_set_hash_match.is_ok() || next_validator_set_hash_match.is_ok() {
					log::trace!(target: "tesseract", "validator set match found at {height}");
					matched_header = Some(header);
					break;
				}
				height -= 1;
			}

			if matched_header.is_some() {
				let matched_height = height;
				let matched_header = matched_header.expect("Header must be present if found");
				let next_validators = client.prover.next_validators(matched_height).await?;

				// Also attempt to construct a milestone update corresponding to the matched header
				// height
				let maybe_milestone_update = build_milestone_update(
					client,
					matched_header.header.height.value(),
					&consensus_state,
				)
				.await?;

				return Ok(Some(PolygonConsensusUpdate {
					tendermint_proof: CodecConsensusProof::from(&ConsensusProof::new(
						matched_header.clone(),
						if matched_header.header.next_validators_hash.is_empty() {
							None
						} else {
							Some(next_validators)
						},
					)),
					milestone_update: maybe_milestone_update,
				}));
			} else {
				log::error!(target: "tesseract", "Fatal error, failed to find any header that matches onchain validator set");
			}
		},
	}
	log::trace!(target: "tesseract", "No new update found for polygon");
	Ok(None)
}

async fn build_milestone_update(
	client: &PolygonPosHost,
	reference_height: u64,
	consensus_state: &ConsensusState,
) -> anyhow::Result<Option<ismp_polygon::MilestoneUpdate>> {
	let query_height = reference_height.saturating_sub(1);
	let latest_milestone_at_height =
		client.prover.get_latest_milestone_at_height(query_height).await?;

	let (milestone_number, milestone) = match latest_milestone_at_height {
		Some((number, milestone)) => (number, milestone),
		None => {
			log::warn!(
				target: "tesseract",
				"No milestone found at height {}, falling back to current latest",
				reference_height
			);
			return Ok(None);
		},
	};

	let milestone_proof = client.prover.get_milestone_proof(milestone_number, query_height).await?;

	if milestone_proof.value.is_empty() {
		return Ok(None);
	}

	if milestone.end_block > consensus_state.last_finalized_block {
		let evm_header = client
			.prover
			.fetch_header(milestone.end_block)
			.await?
			.ok_or_else(|| anyhow::anyhow!("EVM header not found"))?;

		let merkle_proof = milestone_proof
			.clone()
			.proof
			.map(|p| convert_tm_to_ics_merkle_proof::<ICS23HostFunctions>(&p))
			.transpose()
			.map_err(|_| anyhow::anyhow!("bad client state proof"))?
			.ok_or_else(|| anyhow::anyhow!("proof not found"))?;

		let proof = CommitmentProofBytes::try_from(merkle_proof)
			.map_err(|e| anyhow::anyhow!("bad client state proof: {}", e))?;

		Ok(Some(ismp_polygon::MilestoneUpdate {
			evm_header,
			milestone_number,
			ics23_state_proof: proof.into(),
			milestone,
		}))
	} else {
		Ok(None)
	}
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
