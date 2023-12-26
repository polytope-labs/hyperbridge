#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
use anyhow::anyhow;
use ismp::util::Keccak256;
use primitives::{parse_extra, CodecHeader, Header};
use sp_core::{H160, H256};
use sync_committee_verifier::crypto::verify_aggregate_signature;
pub mod primitives;
use crate::primitives::BlockExtraData;
use alloc::collections::BTreeSet;
use ark_bls12_381::G1Projective;
use bls::{point_to_pubkey, pubkey_to_point};
use core::ops::{Add, AddAssign};
use sync_committee_primitives::constants::BlsPublicKey;

extern crate alloc;

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub hash: H256,
    pub header: CodecHeader,
    pub next_validators: Option<BTreeSet<H160>>,
}

pub fn verify_bnb_header<I: Keccak256>(
    validators: &BTreeSet<H160>,
    header: CodecHeader,
) -> Result<VerificationResult, anyhow::Error> {
    let rlp_header: Header = (&header).into();

    let parse_extra_data = parse_extra::<I>(&rlp_header.extra_data.0.as_ref())
        .map_err(|_| anyhow!("could not parse extra data from header"))?;

    let bls_public_keys: Vec<[u8; 48]> = parse_extra_data
        .validators
        .iter()
        .map(|validator| validator.bls_public_key)
        .collect();

    let aggregate_public_key = aggregate_public_keys(bls_public_keys.clone())?;

    let msg = parse_extra_data.vote_data_hash;

    let signature = parse_extra_data.agg_signature;

    let bls_public_keys_as_byte_vectors: Vec<BlsPublicKey> = bls_public_keys
        .iter()
        .map(|&bytes| BlsPublicKey::try_from(&bytes[..]).expect("Failed to convert to ByteVector"))
        .collect();

    let aggregate_public_key_byte_vector =
        BlsPublicKey::try_from(&aggregate_public_key[..]).expect("Failed to convert to ByteVector");

    verify_aggregate_signature(
        &aggregate_public_key_byte_vector,
        bls_public_keys_as_byte_vectors.as_slice(),
        msg.0.to_vec(),
        signature.to_vec().as_ref(),
    )
    .map_err(|_| anyhow!("Could not verify aggregate signature"))?;

    let hash = rlp_header.hash::<I>()?;

    let validator_addresses: Option<BTreeSet<H160>> = Some(
        parse_extra_data
            .validators
            .iter()
            .map(|info| H160::from_slice(&info.address))
            .collect(),
    );

    Ok(VerificationResult { hash, header, next_validators: validator_addresses })
}

fn aggregate_public_keys(keys: Vec<[u8; 48]>) -> Result<Vec<u8>, anyhow::Error> {
    let mut aggregate = pubkey_to_point(keys[0].to_vec().as_ref())
        .map_err(|_| anyhow!("could not convert index 0 public key to point"))?;

    for key in keys.iter().skip(1) {
        let next = pubkey_to_point(&key.to_vec())
            .map_err(|_| anyhow!("could not convert public key to point"))?;
        aggregate = aggregate.add(next).into();
    }

    Ok(point_to_pubkey(aggregate))
}
