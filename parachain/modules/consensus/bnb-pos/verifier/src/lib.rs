#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
use alloc::vec::Vec;
use anyhow::anyhow;
use ark_ec::AffineRepr;
use ismp::util::Keccak256;
use primitives::{parse_extra, BnbClientUpdate, VALIDATOR_BIT_SET_SIZE};
use sp_core::H256;
use sync_committee_verifier::crypto::{pairing, pubkey_to_projective};
pub mod primitives;
use bls::{
    point_to_pubkey, pubkey_to_point,
    types::{G1AffinePoint, G1ProjectivePoint, Signature},
    DST_ETHEREUM,
};
use geth_primitives::{CodecHeader, Header};
use ssz_rs::{Bitvector, Deserialize};
use sync_committee_primitives::constants::BlsPublicKey;

extern crate alloc;

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub hash: H256,
    pub finalized_header: CodecHeader,
    pub next_validators: Option<NextValidators>,
}

#[derive(Debug, Clone, Default, codec::Encode, codec::Decode)]
pub struct NextValidators {
    pub validators: Vec<BlsPublicKey>,
    pub rotation_block: u64,
}

pub fn verify_bnb_header<H: Keccak256>(
    current_validators: &Vec<BlsPublicKey>,
    update: BnbClientUpdate,
) -> Result<VerificationResult, anyhow::Error> {
    let extra_data = parse_extra::<H>(&update.attested_header.extra_data)
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

    if validators_bit_set.iter().as_bitslice().count_ones() < (2 * current_validators.len() / 3) {
        Err(anyhow!("Not enough participants"))?
    }

    let participants: Vec<BlsPublicKey> = current_validators
        .iter()
        .zip(validators_bit_set.iter())
        .filter_map(|(validator, bit)| if *bit { Some(validator.clone()) } else { None })
        .collect();

    let aggregate_public_key = aggregate_public_keys(&participants)
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("Could not aggregate public keys"))?;

    let msg = H::keccak256(alloy_rlp::encode(extra_data.vote_data.clone()).as_slice());

    let signature = extra_data.agg_signature;

    verify_aggregate_signature(&aggregate_public_key, msg.0.to_vec(), signature.to_vec().as_ref())
        .map_err(|_| anyhow!("Could not verify aggregate signature"))?;

    let source_header_hash = Header::from(&update.source_header).hash::<H>();
    let target_header_hash = Header::from(&update.target_header).hash::<H>();

    if source_header_hash.0 != extra_data.vote_data.source_hash.0 ||
        target_header_hash.0 != extra_data.vote_data.target_hash.0
    {
        Err(anyhow!("Target and Source headers do not match vote data"))?
    }

    let next_validator_addresses: Option<NextValidators> =
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
            let epoch_header_extra_data = parse_extra::<H>(&epoch_header.extra_data)
                .map_err(|_| anyhow!("could not parse extra data from epoch header"))?;
            let validators = epoch_header_extra_data
                .validators
                .into_iter()
                .map(|val| val.bls_public_key.as_slice().try_into().expect("Infallible"))
                .collect::<Vec<BlsPublicKey>>();

            if !validators.is_empty() {
                Some(NextValidators {
                    validators,
                    rotation_block: update.source_header.number.low_u64() + 12,
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

pub fn verify_aggregate_signature(
    aggregate: &BlsPublicKey,
    msg: Vec<u8>,
    signature: &Signature,
) -> anyhow::Result<()> {
    let aggregate_key_point: G1AffinePoint =
        pubkey_to_point(aggregate).map_err(|_| anyhow!("Could not convert public key to point"))?;
    let signature = bls::signature_to_point(signature).map_err(|e| anyhow!("{:?}", e))?;

    if !bls::signature_subgroup_check(signature) {
        Err(anyhow!("Signature not in subgroup"))?
    }

    let q = bls::hash_to_point(&msg, &DST_ETHEREUM.as_bytes().to_vec());
    let c1 = pairing(q, aggregate_key_point);

    // From the spec:
    // > When the signature variant is minimal-pubkey-size, P is the distinguished point P1 that
    // > generates the group G1.
    // <https://www.ietf.org/archive/id/draft-irtf-cfrg-bls-signature-05.html#section-2.2>
    let p = G1AffinePoint::generator();

    let c2 = pairing(signature, p);

    if c1 == c2 {
        Ok(())
    } else {
        Err(anyhow!("Aggregate signature verification failed"))
    }
}
