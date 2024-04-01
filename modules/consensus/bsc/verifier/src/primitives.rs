use alloc::vec::Vec;
use alloy_primitives::{FixedBytes, B256};
use alloy_rlp::Decodable;
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use anyhow::anyhow;
use codec::{Decode, Encode};
use ethabi::ethereum_types::H160;
use geth_primitives::CodecHeader;
use ismp::util::Keccak256;
use sp_core::ConstU32;

pub const EPOCH_LENGTH: u64 = 200;
const EXTRA_VANITY_LENGTH: usize = 32;
const EXTRA_SEAL_LENGTH: usize = 65;
const BLS_PUBLIC_KEY_LENGTH: usize = 48;
const VALIDATOR_BYTES_LENGTH: usize = 20 + BLS_PUBLIC_KEY_LENGTH;
const VALIDATOR_NUMBER_SIZE: usize = 1; // // Fixed number of extra prefix bytes reserved for validator number after Luban
const ADDRESS_LENGTH: usize = 20;
pub const VALIDATOR_BIT_SET_SIZE: usize = 64;

#[derive(Debug, Encode, Decode, Clone)]
pub struct BscClientUpdate {
    /// Finalized header
    pub source_header: CodecHeader,
    /// Justified header
    pub target_header: CodecHeader,
    /// Header that contains the attestation
    pub attested_header: CodecHeader,
    /// Epoch header ancestry up to source header
    /// The Epoch header should the first header in the vector
    pub epoch_header_ancestry: sp_runtime::BoundedVec<CodecHeader, ConstU32<32>>,
}

#[derive(Debug, Clone)]
pub struct ExtraData {
    pub extra_vanity: Vec<u8>,
    pub validator_size: u8,
    pub validators: Vec<ValidatorInfo>,
    pub extra_seal: Vec<u8>,
    pub agg_signature: [u8; 96],
    pub vote_data: VoteData,
    pub vote_address_set: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorInfo {
    pub address: H160,
    pub bls_public_key: [u8; 48],
}

// Used for Encoding and Decoding of Vote Attestation
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct VoteAttestationData {
    pub vote_address_set: u64,
    pub agg_signature: FixedBytes<96>, //[u8; 96],
    pub data: VoteData,
    pub extra: alloy_primitives::Bytes,
}

// Used for Encoding and Decoding of Vote
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct VoteData {
    pub source_number: u64,
    pub source_hash: B256,
    pub target_number: u64,
    pub target_hash: B256,
}

pub fn parse_extra<H: Keccak256>(extra_data: &[u8]) -> Result<ExtraData, anyhow::Error> {
    let data = extra_data;

    let mut extra = ExtraData {
        extra_vanity: Vec::new(),
        validator_size: 0,
        validators: Vec::new(),
        extra_seal: Vec::new(),
        agg_signature: [0; 96],
        vote_data: VoteData {
            source_number: 0,
            source_hash: Default::default(),
            target_number: 0,
            target_hash: Default::default(),
        },
        vote_address_set: 0,
    };

    if extra_data.len() < EXTRA_VANITY_LENGTH + EXTRA_SEAL_LENGTH {
        Err(anyhow!("Invalid extra data"))?;
    }

    extra.extra_vanity = data[..EXTRA_VANITY_LENGTH].to_vec();
    extra.extra_seal = data[data.len() - EXTRA_SEAL_LENGTH..].to_vec();
    let mut data = &data[EXTRA_VANITY_LENGTH..data.len() - EXTRA_SEAL_LENGTH];

    let mut data_length = data.len();
    if !data.is_empty() {
        // Parse Validators
        if data[0] != 0xf8 {
            // RLP format of attestation begins with 'f8'
            let validator_num = data[0].clone() as usize;
            let validator_bytes_total_length =
                VALIDATOR_NUMBER_SIZE + validator_num * VALIDATOR_BYTES_LENGTH;
            if data_length < validator_bytes_total_length.clone() as usize {
                Err(anyhow!("Parse validator failed"))?;
            }
            extra.validator_size = validator_num.clone() as u8;
            let remaining_data = &data[VALIDATOR_NUMBER_SIZE..];

            for i in 0..validator_num {
                let mut validator_info =
                    ValidatorInfo { address: H160::default(), bls_public_key: [0; 48] };

                let address_bytes: Vec<u8> = remaining_data[i.clone() * VALIDATOR_BYTES_LENGTH..
                    i.clone() * VALIDATOR_BYTES_LENGTH + ADDRESS_LENGTH]
                    .to_vec();
                let bls_public_key_bytes: Vec<u8> =
                    remaining_data[i.clone() * VALIDATOR_BYTES_LENGTH + ADDRESS_LENGTH..
                        (i.clone() + 1) * VALIDATOR_BYTES_LENGTH]
                        .to_vec();

                validator_info.address = H160::from_slice(&address_bytes);
                validator_info.bls_public_key.copy_from_slice(&bls_public_key_bytes);

                extra.validators.push(validator_info);
            }
            extra.validators.sort_by(|a, b| a.address.0.cmp(&b.address.0));
            data = &remaining_data[validator_bytes_total_length - VALIDATOR_NUMBER_SIZE..];
            data_length = data.len();
        }

        // parse attestation
        if data_length > 0 {
            let vote_attestation_data: VoteAttestationData = VoteAttestationData::decode(&mut data)
                .map_err(|_| anyhow!("parse vote attestation failed"))?;

            extra.agg_signature = vote_attestation_data.agg_signature.0.into();
            extra.vote_data = vote_attestation_data.data.into();

            extra.vote_address_set = vote_attestation_data.vote_address_set;
        }
    }

    Ok(extra.clone())
}

pub fn compute_epoch(number: u64) -> u64 {
    number / EPOCH_LENGTH
}
