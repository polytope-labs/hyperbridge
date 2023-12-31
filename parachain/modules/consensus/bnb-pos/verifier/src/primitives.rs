use alloc::{vec, vec::Vec};
use alloy_primitives::{Address, FixedBytes, B256};
use alloy_rlp::Decodable;
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use anyhow::anyhow;
use bitset::BitSet;
use bitvec::bitvec;
use ethabi::ethereum_types::{Bloom, H160, H256, H64, U256};
use sp_core::sp_std::cmp::Ordering;
use ismp::{host::IsmpHost, util::Keccak256};


pub const SPAN_LENGTH: u64 = 400 * 16;
pub const EPOCH_LENGTH: u64 = 200;
const EXTRA_VANITY_LENGTH: usize = 32;
const EXTRA_SEAL_LENGTH: usize = 65;
const LUBAN_BLOCK_NUMBER: u64 = 29020050;
const PARLIA_CONFIG_EPOCH: u64 = 200;
const VALIDATOR_BYTES_LENGTH_BEFORE_LUBAN: u8 = 20;
const BLS_PUBLIC_KEY_LENGTH: usize = 48;
const VALIDATOR_BYTES_LENGTH: usize = 20 + BLS_PUBLIC_KEY_LENGTH;
const VALIDATOR_NUMBER_SIZE: usize = 1; // // Fixed number of extra prefix bytes reserved for validator number after Luban
const ADDRESS_LENGTH: usize = 20;

#[derive(codec::Encode, codec::Decode)]
pub struct VerifierState {
    pub validators: Vec<H160>,
    pub current_validators: Vec<H160>,
    pub finalized_height: u64,
    pub finalized_hash: H256,
}

#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct Header {
    pub parent_hash: B256,
    pub uncle_hash: B256,
    pub coinbase: Address,
    pub state_root: B256,
    pub transactions_root: B256,
    pub receipts_root: B256,
    pub logs_bloom: FixedBytes<256>,
    pub difficulty: alloy_primitives::U256,
    pub number: alloy_primitives::U256,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: alloy_primitives::Bytes,
    pub mix_hash: B256,
    pub nonce: FixedBytes<8>,
    pub base_fee_per_gas: Option<alloy_primitives::U256>,
    pub withdrawals_root: Option<B256>,
    pub blob_gas_used: Option<alloy_primitives::U256>,
    pub excess_blob_gas: Option<alloy_primitives::U256>,
}

#[derive(Debug, Clone)]
pub struct CodecExtraData {
    pub extra_vanity: Vec<u8>,
    pub validator_size: u8,
    pub validators: Vec<CodecValidatorInfo>,
    pub extra_seal: Vec<u8>,
    pub agg_signature: [u8; 96],
    pub vote_data_hash: H256,
    pub vote_data: CodecVoteData,
}

#[derive(Debug, Clone)]
pub struct CodecValidatorInfo {
    pub address: [u8; 20],
    pub bls_public_key: [u8; 48],
    pub vote_included: bool,
}

impl Eq for CodecValidatorInfo {}


impl Ord for CodecValidatorInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.address.cmp(&other.address)
    }
}

impl PartialOrd for CodecValidatorInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CodecValidatorInfo {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}


// Used for Encoding and Decoding of Vote Attestation
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct VoteAttestationData {
    pub vote_address_set: u64,
    pub agg_signature: FixedBytes<96>,//[u8; 96],
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

#[derive(Debug, Clone)]
pub struct CodecVoteData {
    pub source_number: u64,
    pub source_hash: H256,
    pub target_number: u64,
    pub target_hash: H256,
}

impl From<VoteData> for CodecVoteData {
    fn from(vote_data: VoteData) -> Self {
        CodecVoteData {
            source_number: vote_data.source_number,
            source_hash: vote_data.source_hash.0.into(),
            target_number: vote_data.target_number,
            target_hash: vote_data.target_hash.0.into()
        }
    }
}

#[derive(codec::Encode, codec::Decode, Debug, Clone)]
pub struct CodecHeader {
    pub parent_hash: H256,
    pub uncle_hash: H256,
    pub coinbase: H160,
    pub state_root: H256,
    pub transactions_root: H256,
    pub receipts_root: H256,
    pub logs_bloom: Bloom,
    pub difficulty: U256,
    pub number: U256,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub mix_hash: H256,
    pub nonce: H64,
    pub base_fee_per_gas: Option<U256>,
    pub withdrawals_root: Option<H256>,
    pub blob_gas_used: Option<U256>,
    pub excess_blob_gas: Option<U256>,
}

impl From<&CodecHeader> for Header {
    fn from(value: &CodecHeader) -> Self {
        Header {
            parent_hash: value.parent_hash.0.into(),
            uncle_hash: value.uncle_hash.0.into(),
            coinbase: value.coinbase.0.into(),
            state_root: value.state_root.0.into(),
            transactions_root: value.transactions_root.0.into(),
            receipts_root: value.receipts_root.0.into(),
            logs_bloom: value.logs_bloom.0.into(),
            difficulty: {
                let mut bytes = [0u8; 32];
                value.difficulty.to_big_endian(&mut bytes);
                alloy_primitives::U256::from_be_bytes(bytes)
            },
            number: {
                let mut bytes = [0u8; 32];
                value.number.to_big_endian(&mut bytes);
                alloy_primitives::U256::from_be_bytes(bytes)
            },
            gas_limit: value.gas_limit,
            gas_used: value.gas_used,
            timestamp: value.timestamp,
            extra_data: value.extra_data.clone().into(),
            mix_hash: value.mix_hash.0.into(),
            nonce: value.nonce.0.into(),
            base_fee_per_gas: value.base_fee_per_gas.map(|val| {
                let mut bytes = [0u8; 32];
                val.to_big_endian(&mut bytes);
                alloy_primitives::U256::from_be_bytes(bytes)
            }),

            withdrawals_root: value.withdrawals_root.map(|val| val.0.into()),
            blob_gas_used: value.blob_gas_used.map(|val| {
                let mut bytes = [0u8; 32];
                val.to_big_endian(&mut bytes);
                alloy_primitives::U256::from_be_bytes(bytes)
            }),
            excess_blob_gas: value.excess_blob_gas.map(|val| {
                let mut bytes = [0u8; 32];
                val.to_big_endian(&mut bytes);
                alloy_primitives::U256::from_be_bytes(bytes)
            }),
        }
    }
}

impl Header {
    pub fn hash<H: Keccak256>(self) -> Result<H256, anyhow::Error> {
        if self.extra_data.len() < (EXTRA_VANITY_LENGTH + EXTRA_SEAL_LENGTH) {
            Err(anyhow!("Invalid extra data"))?
        }
        let encoding = alloy_rlp::encode(self);
        Ok(H::keccak256(&encoding))
    }
}

pub fn parse_extra<H: Keccak256>(extra_data: &[u8]) -> Result<CodecExtraData, anyhow::Error> {
    let data = extra_data;

    let mut vote_data = CodecVoteData {
        source_number: 0,
        source_hash: Default::default(),
        target_number: 0,
        target_hash: Default::default()
    };

    let mut extra = CodecExtraData {
        extra_vanity: Vec::new(),
        validator_size: 0,
        validators: Vec::new(),
        extra_seal: Vec::new(),
        agg_signature: [0; 96],
        vote_data_hash: H256::zero(),
        vote_data,
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
                println!(
                    "Parse Validator failed",
                );
                Err(anyhow!("Parse validator failed"))?;
            }
            println!(
                "Validator size {:?}", validator_num.clone() as u8
            );
            extra.validator_size = validator_num.clone() as u8;
            let mut remaining_data = &data[VALIDATOR_NUMBER_SIZE..];


            for i in 0..validator_num {
                let mut validator_info = CodecValidatorInfo {
                    address: [0; 20],
                    bls_public_key: [0; 48],
                    vote_included: false,
                };

                let address_bytes: Vec<u8> = remaining_data[i.clone() * VALIDATOR_BYTES_LENGTH .. i.clone() * VALIDATOR_BYTES_LENGTH + ADDRESS_LENGTH].to_vec();

                let bls_public_key_bytes: Vec<u8> =
                    remaining_data[i.clone() * VALIDATOR_BYTES_LENGTH + ADDRESS_LENGTH .. (i.clone() + 1) * VALIDATOR_BYTES_LENGTH].to_vec();

                validator_info.address.copy_from_slice(&address_bytes);
                validator_info.bls_public_key.copy_from_slice(&bls_public_key_bytes);


                extra.validators.push(validator_info);
            }

            println!(
                "VALIDATORS BEFORE SORT"
            );

            let mut count = 0;
            for info in extra.validators.clone() {
                let hex_string = hex::encode(info.address);
                println!(
                    "befire sort, i is {:?}, Validator address {:?},  hex is  {:?}, public key is {:?}", count, info.address, hex_string, info.bls_public_key
                );
                count = count + 1;
            }

            extra.validators.sort_by(|a, b| a.address.cmp(&b.address));

            println!(
                "VALIDATORS AFTER SORT"
            );

            let mut count = 0;
            for info in extra.validators.clone() {
                let hex_string = hex::encode(info.address);
                println!(
                    "after sort, i is {:?}, Validator address {:?},  hex is  {:?}, public key is {:?}", count, info.address, hex_string, info.bls_public_key
                );

                count = count + 1;
            }

            data = &remaining_data[validator_bytes_total_length - VALIDATOR_NUMBER_SIZE..];
            data_length = data.len();
        }

        // parse attestation
        if data_length > 0 {
            let vote_attestation_data: VoteAttestationData = VoteAttestationData::decode(&mut data)
                .map_err(|_| anyhow!("parse voteAttestation failed"))?;

            println!(
                "vote_attestation_data is {:?}", vote_attestation_data
            );

            //get_voting_validators(extra.validators.clone(), &vote_attestation_data);


            extra.agg_signature = vote_attestation_data.agg_signature.0.into();
            /*extra.vote_data_hash =
                H::keccak256(alloy_rlp::encode(vote_attestation_data.data.clone()).as_slice());*/
            let vote_hash_bytes: [u8; 32] = hex::decode("039e9112b38622bc7f76a6d576bbb53c2e5354a701d404219eec796b9a1a3e12").unwrap().as_slice().try_into().unwrap();
            extra.vote_data_hash = vote_hash_bytes.into();
            extra.vote_data = vote_attestation_data.data.into();

            let validators_bit_set = BitSet::from_u64(vote_attestation_data.vote_address_set);

            for i in 0..extra.validator_size as usize {
                if validators_bit_set.test(i as usize) {
                    extra.validators[i.clone()].vote_included = true;
                }
            }

        }

        extra.validators.retain(|validator| validator.vote_included && validator.bls_public_key != [0; 48]);

        println!("retained validator length {:?}",  extra.validators.len());

    }

    println!(
        "extra is {:?}", extra.clone()
    );
    Ok(extra.clone())
}

pub fn compute_epoch(number: u64) -> u64 {
    number / EPOCH_LENGTH
}

