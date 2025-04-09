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
#[warn(unused_imports)]
#[warn(unused_variables)]
use polkadot_sdk::*;

use alloc::vec::Vec;
use anyhow::anyhow;
use bls::{point_to_pubkey, types::G1ProjectivePoint};
use geth_primitives::{CodecHeader, Header};
use ismp::messaging::Keccak256;
use primitives::{parse_extra, BscClientUpdate, Config, VALIDATOR_BIT_SET_SIZE};
use sp_core::H256;
use ssz_rs::{Bitvector, Deserialize};
use sync_committee_primitives::constants::BlsPublicKey;
use sync_committee_verifier::crypto::pubkey_to_projective;

pub mod primitives;

extern crate alloc;

#[derive(Debug, Clone)]
pub struct VerificationResult {
	pub hash: H256,
	pub finalized_header: CodecHeader,
	pub next_validators: Option<NextValidators>,
}

#[derive(Debug, Clone, Default, codec::Encode, codec::Decode, PartialEq, Eq)]
pub struct NextValidators {
	pub validators: Vec<BlsPublicKey>,
	pub rotation_block: u64,
}

pub fn verify_bsc_header<H: Keccak256, C: Config>(
	current_validators: &Vec<BlsPublicKey>,
	update: BscClientUpdate,
	epoch_length: u64,
) -> Result<VerificationResult, anyhow::Error> {
	let extra_data = parse_extra::<H, C>(&update.attested_header)
		.map_err(|_| anyhow!("could not parse extra data from header"))?;
	let source_hash = H256::from_slice(&extra_data.vote_data.source_hash.0);
	let target_hash = H256::from_slice(&extra_data.vote_data.target_hash.0);
	if source_hash == Default::default() || target_hash == Default::default() {
		Err(anyhow!("Vote data is empty"))?
	}

	let validators_bit_set = Bitvector::<VALIDATOR_BIT_SET_SIZE>::deserialize(
		extra_data.vote_address_set.to_le_bytes().to_vec().as_slice(),
	)
	.map_err(|_| anyhow!("Could not deseerialize vote address set"))?;

	if validators_bit_set.iter().as_bitslice().count_ones() <
		((2 * current_validators.len() / 3) + 1)
	{
		Err(anyhow!("Not enough participants"))?
	}

	let source_header_hash = Header::from(&update.source_header).hash::<H>();
	let target_header_hash = Header::from(&update.target_header).hash::<H>();

	if source_header_hash.0 != extra_data.vote_data.source_hash.0 ||
		target_header_hash.0 != extra_data.vote_data.target_hash.0
	{
		Err(anyhow!("Target and Source headers do not match vote data"))?
	}

	let participants: Vec<BlsPublicKey> = current_validators
		.iter()
		.zip(validators_bit_set.iter())
		.filter_map(|(validator, bit)| if *bit { Some(validator.clone()) } else { None })
		.collect();

	let aggregate_public_key = aggregate_public_keys(&participants);
	let msg = H::keccak256(alloy_rlp::encode(extra_data.vote_data.clone()).as_slice());
	let signature = extra_data.agg_signature;

	let verify = bls::verify(
		&aggregate_public_key,
		&msg.as_ref().to_vec(),
		signature.to_vec().as_ref(),
		&bls::DST_ETHEREUM.as_bytes().to_vec(),
	);

	if !verify {
		Err(anyhow!("Could not verify aggregate signature"))?
	}

	let next_validator_addresses: Option<NextValidators> =
        // If an epoch ancestry was provided, we try to extract the next validator set from it
        if !update.epoch_header_ancestry.is_empty() {
            let mut parent_hash = Header::from(&update.epoch_header_ancestry[0]).hash::<H>();
            for header in update.epoch_header_ancestry[1..].into_iter() {
                if parent_hash != header.parent_hash {
                    Err(anyhow!("Epoch ancestry submitted is invalid"))?
                }
                parent_hash = Header::from(header).hash::<H>()
            }
            if parent_hash != update.source_header.parent_hash {
                Err(anyhow!("Epoch ancestry submitted is invalid"))?
            }
            let epoch_header = update.epoch_header_ancestry[0].clone();
            let epoch_header_extra_data = parse_extra::<H, C>(&epoch_header)
                .map_err(|_| anyhow!("could not parse extra data from epoch header"))?;
            let validators = epoch_header_extra_data
                .validators
                .into_iter()
                .map(|val| val.bls_public_key.as_slice().try_into().expect("Infallible"))
                .collect::<Vec<BlsPublicKey>>();

            if !validators.is_empty() {
                Some(NextValidators {
                    validators,
                    rotation_block: epoch_header.number.low_u64() +
                        (current_validators.len() as u64 / 2),
                })
            } else {
                Err(anyhow!(
                    "Epoch header provided does not have a validator set present in its extra data"
                ))?
            }
            // If the source header that was finalized is the epoch header we extract the next validator set
        } else if update.source_header.number.low_u64() % epoch_length == 0 {
            let epoch_header_extra_data = parse_extra::<H, C>(&update.source_header)
                .map_err(|_| anyhow!("could not parse extra data from epoch header"))?;
            let validators = epoch_header_extra_data
                .validators
                .into_iter()
                .map(|val| val.bls_public_key.as_slice().try_into().expect("Infallible"))
                .collect::<Vec<BlsPublicKey>>();

            if !validators.is_empty() {
                Some(NextValidators {
                    validators,
                    rotation_block: update.source_header.number.low_u64() +
                        (current_validators.len() as u64 / 2),
                })
            } else {
                Err(anyhow!(
                    "Epoch header provided does not have a validator set present in its extra data"
                ))?
            }
        } else {
            None
        };

	Ok(VerificationResult {
		hash: source_header_hash,
		finalized_header: update.source_header,
		next_validators: next_validator_addresses,
	})
}

pub fn aggregate_public_keys(keys: &[BlsPublicKey]) -> Vec<u8> {
	let aggregate = keys
		.into_iter()
		.filter_map(|key| pubkey_to_projective(key).ok())
		.fold(G1ProjectivePoint::default(), |acc, next| acc + next);

	point_to_pubkey(aggregate.into())
}
