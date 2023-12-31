#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
use anyhow::anyhow;
use bitvec::vec::BitVec;
use ismp::util::Keccak256;
use primitives::{parse_extra, BnbClientUpdate, CodecHeader, Header};
use sp_core::{H160, H256};
use sync_committee_verifier::crypto::{pubkey_to_projective, verify_aggregate_signature};
pub mod primitives;
use bls::{point_to_pubkey, types::G1ProjectivePoint};
use sync_committee_primitives::constants::BlsPublicKey;

extern crate alloc;

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub hash: H256,
    pub finalized_header: CodecHeader,
    pub next_validators: Option<NextValidators>,
}


#[derive(Debug, Clone)]
pub struct NextValidators {
    pub validators: Vec<BlsPublicKey>,
    pub aggregate_public_key: BlsPublicKey,
    pub rotation_block: u64,
}

pub fn verify_bnb_header<H: Keccak256>(
    aggregate_public_key: BlsPublicKey,
    current_validators: &Vec<BlsPublicKey>,
    update: BnbClientUpdate,
) -> Result<VerificationResult, anyhow::Error> {
    let extra_data = parse_extra::<H>(&update.attested_header.extra_data)
        .map_err(|_| anyhow!("could not parse extra data from header"))?;

    let validators_bit_set = BitVec::<_>::from_element(extra_data.vote_address_set);
    dbg!(validators_bit_set.as_bitslice());

    if validators_bit_set.count_ones() < (2 * current_validators.len() / 3) {
        Err(anyhow!("Not enough participants"))?
    }
    let non_participants: Vec<BlsPublicKey> = current_validators
        .iter()
        .zip(validators_bit_set.iter())
        .filter_map(|(validator, bit)| if !(*bit) { Some(validator.clone()) } else { None })
        .collect();

    let msg = H::keccak256(alloy_rlp::encode(extra_data.vote_data.clone()).as_slice());

    let signature = extra_data.agg_signature;

    verify_aggregate_signature(
        &aggregate_public_key,
        &non_participants,
        msg.0.to_vec(),
        signature.to_vec().as_ref(),
    )
    .map_err(|_| anyhow!("Could not verify aggregate signature"))?;

    let source_header_hash = Header::from(&update.source_header).hash::<H>()?;
    let target_header_hash = Header::from(&update.target_header).hash::<H>()?;

    if source_header_hash.0 != extra_data.vote_data.source_hash.0 ||
        target_header_hash.0 != target_header_hash.0
    {
        Err(anyhow!("Target and Source headers do not match vote data"))?
    }

    let next_validator_addresses: Option<NextValidators> = {
        let validators = extra_data
            .validators
            .into_iter()
            .map(|val| val.bls_public_key.as_slice().try_into().expect("Infallible"))
            .collect::<Vec<BlsPublicKey>>();
        let aggregate_public_key = aggregate_public_keys(&validators).as_slice().try_into()?;
        if !validators.is_empty() {
            Some(NextValidators {
                validators,
                aggregate_public_key,
                rotation_block: update.attested_header.number.low_u64() +
                    current_validators.len() as u64 / 2,
            })
        } else {
            None
        }
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
