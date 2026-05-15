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
use crypto_utils::aggregate_public_keys;
use geth_primitives::{CodecHeader, Header};
use ismp::messaging::Keccak256;
use primitives::{BscClientUpdate, Config, VALIDATOR_BIT_SET_SIZE, parse_extra};
use sp_core::H256;
use ssz_rs::{Bitvector, Deserialize};
use sync_committee_primitives::constants::BlsPublicKey;

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

	// `VALIDATOR_BIT_SET_SIZE` is a fixed 64-bit width; the active
	// validator set is smaller, so bits at positions `>= validators.len()`
	// have no corresponding validator. Setting them would inflate
	// `count_ones()` past the supermajority threshold without any extra
	// validator actually signing.
	if validators_bit_set
		.iter()
		.enumerate()
		.any(|(i, bit)| i >= current_validators.len() && *bit)
	{
		Err(anyhow!("Vote address set has bits set beyond validator count"))?
	}

	// We have to use the same threshold specified in the bsc parlia consensus which is 2/3
	// https://github.com/bnb-chain/bsc/blob/da35ee13e2fe38efaeab2d6fb27f112332459b50/consensus/parlia/parlia.go#L557
	let participant_count = validators_bit_set
		.iter()
		.take(current_validators.len())
		.filter(|bit| **bit)
		.count();
	if participant_count < ((2 * current_validators.len()) / 3) {
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

	let aggregate_public_key = aggregate_public_keys(&participants)
		.map_err(|err| anyhow!("Failed to aggregate participant public keys: {err:?}"))?;
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

#[cfg(test)]
mod tests {
	use super::*;
	use alloy_primitives::{B256, Bytes, FixedBytes};
	use alloy_rlp::Encodable;
	use geth_primitives::CodecHeader;
	use primitive_types::{H160, H256, U256};
	use primitives::{Testnet, VoteAttestationData, VoteData};

	/// `sp_core::keccak_256` host wired into the `Keccak256` trait.
	struct TestHost;
	impl Keccak256 for TestHost {
		fn keccak256(bytes: &[u8]) -> H256 {
			sp_core::keccak_256(bytes).into()
		}
	}

	/// Build a `CodecHeader` whose `extra_data` parses into the requested
	/// `VoteAttestationData`. The RLP-encoded attestation always starts with
	/// `0xf8`, so the validator-section branch in `parse_extra` is skipped.
	fn header_with_vote_set(
		vote_address_set: u64,
		source_hash: B256,
		target_hash: B256,
	) -> CodecHeader {
		let attestation = VoteAttestationData {
			vote_address_set,
			agg_signature: FixedBytes::<96>::from([0u8; 96]),
			data: VoteData { source_number: 1, source_hash, target_number: 2, target_hash },
			extra: Bytes::new(),
		};

		let mut attestation_rlp = Vec::new();
		attestation.encode(&mut attestation_rlp);
		// Sanity: long-list prefix so the parser skips validator parsing.
		assert_eq!(attestation_rlp[0], 0xf8);

		let mut extra_data = Vec::with_capacity(32 + attestation_rlp.len() + 65);
		extra_data.extend_from_slice(&[0u8; 32]);
		extra_data.extend_from_slice(&attestation_rlp);
		extra_data.extend_from_slice(&[0u8; 65]);

		CodecHeader {
			parent_hash: H256::zero(),
			uncle_hash: H256::zero(),
			coinbase: H160::zero(),
			state_root: H256::zero(),
			transactions_root: H256::zero(),
			receipts_root: H256::zero(),
			logs_bloom: Default::default(),
			difficulty: U256::zero(),
			number: U256::zero(),
			gas_limit: 0,
			gas_used: 0,
			timestamp: 0,
			extra_data,
			mix_hash: H256::zero(),
			nonce: Default::default(),
			base_fee_per_gas: None,
			withdrawals_hash: None,
			blob_gas_used: None,
			excess_blob_gas_used: None,
			parent_beacon_root: None,
			requests_hash: None,
		}
	}

	fn dummy_validators(n: usize) -> Vec<BlsPublicKey> {
		(0..n).map(|i| vec![i as u8; 48].try_into().expect("48 byte pubkey")).collect()
	}

	fn update_with(attested_header: CodecHeader) -> BscClientUpdate {
		BscClientUpdate {
			source_header: attested_header.clone(),
			target_header: attested_header.clone(),
			attested_header,
			epoch_header_ancestry: Default::default(),
		}
	}

	/// 21 validators (the BSC testnet shape). All 21 in-range bits set —
	/// well above the 2/3 threshold, so the bit-set check passes. The
	/// signature check will then fail (we use a dummy aggregate), but
	/// that's downstream of what we're asserting.
	#[test]
	fn accepts_fully_populated_in_range_bits() {
		let validators = dummy_validators(21);
		// Bits 0..21 set, bits 21..64 clear.
		let mask: u64 = (1u64 << 21) - 1;
		let header = header_with_vote_set(mask, B256::repeat_byte(1), B256::repeat_byte(2));
		let err = verify_bsc_header::<TestHost, Testnet>(&validators, update_with(header), 1000)
			.expect_err("downstream signature check must fail");
		// We made it past the supermajority/junk-bits checks; failure is
		// the header-hash mismatch (we left source_hash/target_hash as
		// constants, but the recomputed hash of the empty CodecHeader
		// won't match those). Either way, neither of the two errors we
		// guard against here.
		let msg = format!("{err}");
		assert!(!msg.contains("Vote address set has bits set beyond validator count"));
		assert!(!msg.contains("Not enough participants"));
	}

	/// Setting a bit past `current_validators.len()` must be rejected,
	/// even when it would otherwise inflate `count_ones()` over the
	/// 2/3 threshold.
	#[test]
	fn rejects_bits_set_beyond_validator_count() {
		let validators = dummy_validators(21);
		// 10 in-range bits (below the 14-vote threshold) plus 30 bits
		// in the junk range [21, 64) — pre-fix this would clear the
		// supermajority check at 40 ones; post-fix it is rejected.
		let in_range: u64 = (1u64 << 10) - 1;
		let junk: u64 = ((1u64 << 51) - 1) << 21; // bits 21..=63 (wraps to 30 bits set)
		let header =
			header_with_vote_set(in_range | junk, B256::repeat_byte(1), B256::repeat_byte(2));

		let err = verify_bsc_header::<TestHost, Testnet>(&validators, update_with(header), 1000)
			.expect_err("junk bits must be rejected");
		assert!(
			format!("{err}").contains("Vote address set has bits set beyond validator count"),
			"unexpected error: {err:?}"
		);
	}

	/// All bits are within the validator range, but fewer than 2/3 are
	/// set — the supermajority check rejects.
	#[test]
	fn rejects_too_few_in_range_participants() {
		let validators = dummy_validators(21);
		// 10 of 21 bits set (threshold is 14).
		let mask: u64 = (1u64 << 10) - 1;
		let header = header_with_vote_set(mask, B256::repeat_byte(1), B256::repeat_byte(2));

		let err = verify_bsc_header::<TestHost, Testnet>(&validators, update_with(header), 1000)
			.expect_err("under-threshold update must be rejected");
		assert!(format!("{err}").contains("Not enough participants"), "unexpected error: {err:?}");
	}
}
