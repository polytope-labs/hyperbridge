use codec::{Decode, Encode};
use ismp::{host::StateMachine, messaging::Proof};
use sp_core::H256;

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum Key {
    Request(H256),
    Response((H256, H256)),
}
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct WithdrawalProof {
    pub commitments: Vec<Key>,
    pub source_proof: Proof,
    pub dest_proof: Proof,
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
