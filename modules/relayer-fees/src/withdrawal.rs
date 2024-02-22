use alloy_primitives::{Address, B256};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use codec::{Decode, Encode};
use ismp::{host::StateMachine, messaging::Proof};
use sp_core::{H256, U256};
use sp_std::prelude::*;

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub enum Key {
    Request(H256),
    Response { request_commitment: H256, response_commitment: H256 },
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct WithdrawalProof {
    /// Request and response commitments delivered from source to destination
    pub commitments: Vec<Key>,
    /// Request and response commitments on source chain
    pub source_proof: Proof,
    /// Request and response receipts on destination chain
    pub dest_proof: Proof,
}

#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct ResponseReceipt {
    pub response_commitment: B256,
    pub relayer: Address,
}

#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct FeeMetadata {
    pub fee: alloy_primitives::U256,
    pub sender: Address,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct WithdrawalInputData {
    /// Signature data to prove account ownership
    pub signature: Signature,
    /// Chain to withdraw funds from
    pub dest_chain: StateMachine,
    /// Amount to withdraw
    pub amount: U256,
    /// gas limit for evm withdrawals
    pub gas_limit: u64,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub enum Signature {
    /// An Ethereum Address and signature
    Ethereum { address: Vec<u8>, signature: Vec<u8> },
    /// An Sr25519 public key and signature
    Sr25519 { public_key: Vec<u8>, signature: Vec<u8> },
    /// An Ed25519 public key and signature
    Ed25519 { public_key: Vec<u8>, signature: Vec<u8> },
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct WithdrawalParams {
    pub beneficiary_address: Vec<u8>,
    pub amount: U256,
}

impl WithdrawalParams {
    pub fn abi_encode(&self) -> Vec<u8> {
        let mut data = vec![0];
        data.extend_from_slice(&self.beneficiary_address);
        let mut bytes = [0u8; 32];
        self.amount.to_big_endian(&mut bytes);
        data.extend_from_slice(&bytes);
        data
    }
}
