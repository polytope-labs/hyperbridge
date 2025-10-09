use crate::{ConsensusState, TendermintHost};
use codec::Decode;
use ismp_tendermint::TendermintConsensusUpdate;
use std::{result::Result::Ok, sync::Arc, vec::Vec};
use tendermint_primitives::{
	Client, CodecConsensusProof, ConsensusProof, TrustedState, ValidatorSet,
};
use tendermint_verifier::validate_validator_set_hash;
use tesseract_primitives::IsmpProvider;

/// Notification logic for Tendermint relayer
pub async fn consensus_notification(
	client: &TendermintHost,
	_counterparty: Arc<dyn IsmpProvider>,
) -> anyhow::Result<Option<TendermintConsensusUpdate>> {
	let latest_height = client.prover.latest_height().await?;

	let consensus_state_serialized: Vec<u8> =
		_counterparty.query_consensus_state(None, client.consensus_state_id).await?;

	let consensus_state: ConsensusState =
		ConsensusState::decode(&mut &consensus_state_serialized[..])?;

	let trusted_state: TrustedState = consensus_state.tendermint_state.into();

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

	match validator_set_hash_match.is_ok() && next_validator_set_hash_match.is_ok() {
		true => {
			log::trace!(target: "tesseract", "Onchain Validator set matches signed header, constructing consensus proof");
			let next_validators = client.prover.next_validators(latest_height).await?;

			return Ok(Some(TendermintConsensusUpdate {
				tendermint_proof: CodecConsensusProof::from(&ConsensusProof::new(
					untrusted_header.clone(),
					if untrusted_header.header.next_validators_hash.is_empty() {
						None
					} else {
						Some(next_validators)
					},
				)),
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
				if validator_set_hash_match.is_ok() && next_validator_set_hash_match.is_ok() {
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

				return Ok(Some(TendermintConsensusUpdate {
					tendermint_proof: CodecConsensusProof::from(&ConsensusProof::new(
						matched_header.clone(),
						if matched_header.header.next_validators_hash.is_empty() {
							None
						} else {
							Some(next_validators)
						},
					)),
				}));
			} else {
				log::error!(target: "tesseract", "Fatal error, failed to find any header that matches onchain validator set");
			}
		},
	}
	log::trace!(target: "tesseract", "No new update found");
	Ok(None)
}
