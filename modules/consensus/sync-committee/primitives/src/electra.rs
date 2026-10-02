use crate::constants::{BlsPublicKey, BlsSignature, Bytes32, Epoch, Gwei, Slot, ValidatorIndex};

#[derive(
	Default,
	Debug,
	Clone,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	PartialEq,
	Eq,
	codec::Encode,
	codec::Decode,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PendingPartialWithdrawal {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub validator_index: ValidatorIndex,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub amount: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub withdrawable_epoch: Epoch,
}

#[derive(
	Default,
	Debug,
	Clone,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	PartialEq,
	Eq,
	codec::Encode,
	codec::Decode,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PendingConsolidation {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub source_index: ValidatorIndex,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub target_index: ValidatorIndex,
}

#[derive(
	Default,
	Debug,
	Clone,
	ssz_derive::Encode,
	ssz_derive::Decode,
	tree_hash_derive::TreeHash,
	PartialEq,
	Eq,
	codec::Encode,
	codec::Decode,
)]
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
