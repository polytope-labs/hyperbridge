use alloc::{vec, vec::Vec};
use alloy_primitives::{Address, FixedBytes, B256};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use anyhow::anyhow;
use ethabi::ethereum_types::{Bloom, H160, H256, H64, U256};
use ismp::host::IsmpHost;

const EXTRA_VANITY_LENGTH: usize = 32;
const EXTRA_SEAL_LENGTH: usize = 65;
const LUBAN_BLOCK_NUMBER:  u64 = 29020050; // need to confirm the LUBAN fork block number
const PARLIA_CONFIG_EPOCH:  u64 = 200;
const VALIDATOR_BYTES_LENGTH_BEFORE_LUBAN: u8 = 20;
const BLS_PUBLIC_KEY_LENGTH: u8 = 48;
const VALIDATOR_BYTES_LENGTH: u8 = 20 + BLS_PUBLIC_KEY_LENGTH;
const VALIDATOR_NUMBER_SIZE: u8 = 1;
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

// Used for Encoding and Decoding of the Extra Data Field
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct BlockExtraData {
    pub extra_vanity: alloy_primitives::Bytes,
    pub validator_size: Option<u8>,
    pub validators: Option<Vec<ValidatorInfo>>,
    pub vote_attestation: VoteAttestationData,
    pub extra_seal: alloy_primitives::Bytes
}

// Used for Encoding and Decoding of Vote
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct ValidatorInfo {
    pub address: Address,
    pub bls_public_key:[u8;48],
    pub vote_included: bool
}

// Used for Encoding and Decoding of Vote Attestation
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct VoteAttestationData {
    pub extra_vanity: alloy_primitives::Bytes,
    pub validator_size:Option<u8>,
    pub agg_signature: [u8; 96],
    pub data: VoteData,
    pub extra: alloy_primitives::Bytes
}

// Used for Encoding and Decoding of Vote
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct VoteData {
    pub source_number: u64,
    pub source_hash:B256,
    pub target_number: u64,
    pub target_hash:B256,
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
    pub fn hash<H: IsmpHost>(mut self) -> Result<H256, anyhow::Error> {
        if self.extra_data.len() < (EXTRA_VANITY_LENGTH + EXTRA_SEAL_LENGTH) {
            Err(anyhow!("Invalid extra data"))?
        }
        let slice = self.extra_data.len() - EXTRA_SEAL_LENGTH;
        *self.extra_data = self.extra_data[..slice].to_vec().into();
        let encoding = alloy_rlp::encode(self);
        Ok(H::keccak256(&encoding))
    }
}

pub fn get_signature(extra_data: &[u8]) -> Result<[u8; EXTRA_SEAL_LENGTH], anyhow::Error> {
    if extra_data.len() < (EXTRA_VANITY_LENGTH + EXTRA_SEAL_LENGTH) {
        Err(anyhow!("Invalid extra data"))?
    }

    let mut sig = [0u8; 65];
    sig.copy_from_slice(&extra_data[extra_data.len() - EXTRA_SEAL_LENGTH..]);
    Ok(sig)
}
