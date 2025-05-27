use crate::constants::{
	BlsPublicKey, BlsSignature, Bytes32, Epoch, ExecutionAddress, Gwei, Slot, ValidatorIndex,
};
use alloc::{vec, vec::Vec};
use ssz_rs::{prelude::*, Deserialize};

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct DepositRequest {
	#[cfg_attr(feature = "std", serde(rename = "pubkey"))]
	pub pub_key: BlsPublicKey,
	pub withdrawal_credentials: Bytes32,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub amount: Gwei,
	pub signature: BlsSignature,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub index: u64,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawalRequest {
	pub source_address: ExecutionAddress,
	pub validator_pubkey: BlsPublicKey,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub amount: Gwei,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ConsolidationRequest {
	pub source_address: ExecutionAddress,
	pub source_pubkey: BlsPublicKey,
	pub target_pubkey: BlsPublicKey,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutionRequests<
	const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize,
	const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize,
	const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize,
> {
	pub deposits: List<DepositRequest, MAX_DEPOSIT_REQUESTS_PER_PAYLOAD>,
	pub withdrawals: List<WithdrawalRequest, MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD>,
	pub consolidations: List<ConsolidationRequest, MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD>,
}

#[derive(Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PendingPartialWithdrawal {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub validator_index: ValidatorIndex,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub amount: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub withdrawable_epoch: Epoch,
}

#[derive(Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PendingConsolidation {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub source_index: ValidatorIndex,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub target_index: ValidatorIndex,
}

#[derive(Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PendingDeposit {
	pub pubkey: BlsPublicKey,
	pub withdrawal_credentials: Bytes32,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub amount: Gwei,
	pub signature: BlsSignature,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub slot: Slot,
}
