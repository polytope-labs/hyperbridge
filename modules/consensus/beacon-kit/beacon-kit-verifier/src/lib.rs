// Copyright (C) 2022 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

mod error;
pub mod primitives;

use crate::error::BeaconKitError;
use alloc::vec::Vec;
use bsc_verifier::aggregate_public_keys;
use primitive_types::H256;
use primitives::{BeaconKitUpdate, Config, VerificationResult};
use ssz_rs::{prelude::*, Merkleized};
use sync_committee_primitives::{
	constants::{BlsPublicKey, Root, VALIDATOR_REGISTRY_LIMIT},
	domains::DomainType,
	util::{compute_domain, compute_signing_root},
};

/// Verifies a Beacon Kit light client update
pub fn verify_beacon_kit_header<C: Config>(
	current_validators: &Vec<BlsPublicKey>,
	mut update: BeaconKitUpdate,
) -> Result<VerificationResult, BeaconKitError> {
	let signers_set_len = update.signers.len();
	let total_validators = current_validators.len();
	let threshold = (2 * total_validators / 3) + 1;

	if signers_set_len < threshold {
		return Err(BeaconKitError::InsufficientSigners);
	}

	if update.signers.iter().any(|signer| !current_validators.contains(signer)) {
		return Err(BeaconKitError::UnknownSigner);
	}

	let domain = compute_domain(
		DomainType::BeaconProposer,
		Some(C::BEACON_KIT_FORK_VERSION),
		Some(Root::from_bytes(C::GENESIS_VALIDATORS_ROOT)),
		C::GENESIS_FORK_VERSION,
	)
	.map_err(|_| BeaconKitError::DomainComputationFailed)?;

	let mut header = update.beacon_header.clone();
	let signing_root = compute_signing_root(&mut header, domain)
		.map_err(|_| BeaconKitError::SigningRootComputationFailed)?;

	let aggregate_pubkey = aggregate_public_keys(&update.signers);

	let verify_sig = bls::verify(
		&aggregate_pubkey,
		&signing_root.as_bytes().to_vec(),
		&update.signature.to_vec(),
		&bls::DST_ETHEREUM.as_bytes().to_vec(),
	);

	if !verify_sig {
		return Err(BeaconKitError::SignatureVerificationFailed)?
	}

	let execution_payload_root = update
		.execution_payload
		.hash_tree_root()
		.map_err(|_| BeaconKitError::ExecutionPayloadHashFailed)?;
	let execution_payload_proof_nodes: Vec<Node> = update
		.execution_payload_proof
		.iter()
		.map(|byte| Node::from_bytes(byte.as_ref().try_into().expect("Infallible")))
		.collect();

	let is_payload_valid = is_valid_merkle_branch(
		&execution_payload_root,
		execution_payload_proof_nodes.iter(),
		C::EXECTION_PAYLOAD_INDEX_LOG2,
		C::EXECUTION_PAYLOAD_INDEX,
		&update.beacon_header.state_root,
	);

	if !is_payload_valid {
		return Err(BeaconKitError::InvalidExecutionPayloadProof)?
	}

	let next_validators = if let Some(validator_proof) = update.validator_set_proof {
		let mut validators_list = List::<BlsPublicKey, VALIDATOR_REGISTRY_LIMIT>::try_from(
			validator_proof.validators.clone(),
		)
		.map_err(|_| BeaconKitError::SszListCreationFailure)?;

		let validators_root = validators_list
			.hash_tree_root()
			.map_err(|_| BeaconKitError::ValidatorSetHashFailed)?;
		let validator_proof_nodes: Vec<Node> = validator_proof
			.proof
			.iter()
			.map(|byte| Node::from_bytes(byte.as_ref().try_into().expect("Infallible")))
			.collect();

		let is_validator_proof_valid = is_valid_merkle_branch(
			&validators_root,
			validator_proof_nodes.iter(),
			C::VALIDATOR_REGISTRY_INDEX_LOG2,
			C::VALIDATOR_REGSITRY_INDEX,
			&update.beacon_header.state_root,
		);

		if !is_validator_proof_valid {
			return Err(BeaconKitError::InvalidValidatorSetProof)?;
		}

		Some(validator_proof.validators)
	} else {
		None
	};

	Ok(VerificationResult {
		hash: H256::from_slice(signing_root.as_bytes()),
		finalized_header: update.beacon_header,
		next_validators,
	})
}
