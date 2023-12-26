use alloc::{collections::BTreeSet, vec::Vec};
use alloy_primitives::{Address, FixedBytes, B256};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use anyhow::anyhow;
use ethabi::ethereum_types::{Bloom, H160, H256, H64, U256};
use ismp::util::Keccak256;

const EXTRA_VANITY_LENGTH: usize = 32;
const EXTRA_SEAL_LENGTH: usize = 65;
pub const SPAN_LENGTH: u64 = 400 * 16;
//https://github.com/maticnetwork/bor/blob/2ee39192bd5c60f9fd6baa946ae774c6d629e714/core/types/block.go#L74
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
    pub withdrawals_hash: Option<B256>,
    pub excess_data_gas: Option<alloy_primitives::U256>,
}

#[derive(codec::Encode, codec::Decode, Debug, Clone, scale_info::TypeInfo)]
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
    pub withdrawals_hash: Option<H256>,
    pub excess_data_gas: Option<U256>,
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
            withdrawals_hash: value.withdrawals_hash.map(|val| val.0.into()),
            excess_data_gas: value.excess_data_gas.map(|val| {
                let mut bytes = [0u8; 32];
                val.to_big_endian(&mut bytes);
                alloy_primitives::U256::from_be_bytes(bytes)
            }),
        }
    }
}

impl Header {
    pub fn hash_without_sig<H: Keccak256>(mut self) -> Result<H256, anyhow::Error> {
        if self.extra_data.len() < (EXTRA_VANITY_LENGTH + EXTRA_SEAL_LENGTH) {
            Err(anyhow!("Invalid extra data"))?
        }
        let slice = self.extra_data.len() - EXTRA_SEAL_LENGTH;
        *self.extra_data = {
            let bytes = self.extra_data[..slice].to_vec();
            bytes.into()
        };
        self.excess_data_gas = None;
        self.withdrawals_hash = None;
        let encoding = alloy_rlp::encode(self);
        Ok(H::keccak256(&encoding))
    }

    pub fn hash<H: Keccak256>(self) -> Result<H256, anyhow::Error> {
        if self.extra_data.len() < (EXTRA_VANITY_LENGTH + EXTRA_SEAL_LENGTH) {
            Err(anyhow!("Invalid extra data"))?
        }
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

pub fn parse_validators(extra_data: &[u8]) -> Result<Option<BTreeSet<H160>>, anyhow::Error> {
    if extra_data.len() < (EXTRA_VANITY_LENGTH + EXTRA_SEAL_LENGTH) {
        Err(anyhow!("Invalid extra data"))?
    }

    let slice = &extra_data[EXTRA_VANITY_LENGTH..(extra_data.len() - EXTRA_SEAL_LENGTH)];

    if slice.len() == 0 {
        return Ok(None)
    }

    if slice.len() % 20 != 0 {
        Err(anyhow!("Invalid block extra data"))?
    }
    let mut validators = BTreeSet::new();
    for chunk in slice.chunks(20) {
        let address = H160::from_slice(&chunk[..]);
        validators.insert(address);
    }
    Ok(Some(validators))
}
