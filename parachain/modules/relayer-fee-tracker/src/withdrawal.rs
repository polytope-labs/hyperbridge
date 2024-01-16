use alloy_primitives::{Address, B256};
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use codec::{Decode, Encode};
use ismp::{host::StateMachine, messaging::Proof};
use sp_core::H256;

// Define a struct for Request commitment
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct RequestCommitment {
    pub request: H256,
}

// Define a struct for Response commitment
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct ResponseCommitment {
    pub response_1: H256,
    pub response_2: H256,
}

// WithdrawalProof using the struct variants
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct WithdrawalProof {
    pub request_commitments: Vec<RequestCommitment>,
    pub response_commitments: Vec<ResponseCommitment>,
    pub source_proof: Proof,
    pub dest_proof: Proof,
}

impl From<RequestCommitment> for Vec<u8> {
    fn from(commitment: RequestCommitment) -> Self {
        commitment.request.as_fixed_bytes().to_vec()
    }
}

impl From<ResponseCommitment> for Vec<u8> {
    fn from(commitment: ResponseCommitment) -> Self {
        commitment
            .response_1
            .as_fixed_bytes()
            .to_vec()
            .into_iter()
            .chain(commitment.response_2.as_fixed_bytes().to_vec().into_iter())
            .collect()
    }
}

impl WithdrawalProof {
    // Convert request_commitments to Vec<Vec<u8>>
    pub fn to_request_commitments_bytes(&self) -> Vec<Vec<u8>> {
        self.request_commitments
            .iter()
            .map(|commitment| Vec::from(commitment.clone()))
            .collect()
    }

    // Convert response_commitments to Vec<Vec<u8>>
    pub fn to_response_commitments_bytes(&self) -> Vec<Vec<u8>> {
        self.response_commitments
            .iter()
            .map(|commitment| Vec::from(commitment.clone()))
            .collect()
    }
}

#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct ResponseReceipt {
    pub response_commitments: B256,
    pub relayer: Address,
}

#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct FeeMetadata {
    pub fee: u64,
    pub sender: Address,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct WithdrawalInputData {
    pub beneficiary_address: Vec<u8>,
    pub source_chain: StateMachine,
    pub relayer_public_key: Vec<u8>,
    pub amount: u128,
    pub nonce: u64,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct WithdrawalOutputData {
    pub beneficiary_address: Vec<u8>,
    pub amount: u128,
}
