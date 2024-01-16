use alloy_primitives::{Address, B256, U256};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use codec::{Decode, Encode};
use ismp::{host::StateMachine, messaging::Proof};
use sp_core::H256;

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
    pub fee: U256,
    pub sender: Address,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct WithdrawalInputData<Balance> {
    /// Signature data to prove account ownership
    pub signature: Signature,
    /// Chain to withdraw funds from
    pub dest_chain: StateMachine,
    /// Amount to withdraw
    pub amount: Balance,
}

pub enum Signature {
    Ecdsa(Vec<u8>),
    Sr25519 { public_key: Vec<u8>, signature: Vec<u8> },
    Ed25519 { public_key: Vec<u8>, signature: Vec<u8> },
}

#[derive(Debug, Clone, scale_info::TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct WithdrawalParams {
    pub beneficiary_address: Address,
    pub amount: u128,
}
