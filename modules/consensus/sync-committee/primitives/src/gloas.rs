//! Types introduced by the Gloas fork (EIP-7732, enshrined proposer builder separation).
//!
//! ePBS moves the execution payload out of the beacon block and beacon state. What is left
//! behind is a builder's bid committing to an execution `block_hash`, plus a builder registry
//! and the payload timeliness committee. None of these are proven by this client, but they are
//! part of the `BeaconState` and `BeaconBlockBody` containers, so they have to be modelled for
//! the ssz hash tree roots to come out right.

use crate::{
	constants::{
		BlsPublicKey, BlsSignature, Bytes32, Epoch, ExecutionAddress, Gwei, Hash32, Root, Slot,
		ValidatorIndex,
	},
	deneb::KzgCommitment,
};
use alloc::{vec, vec::Vec};
use ssz_rs::{prelude::*, Deserialize};

/// Index into the builder registry.
pub type BuilderIndex = u64;

/// Size of the payload timeliness committee.
pub const PTC_SIZE: usize = 512;

/// Maximum payload attestations in a single block body.
pub const MAX_PAYLOAD_ATTESTATIONS: usize = 4;

pub const BUILDER_REGISTRY_LIMIT: usize = 2usize.saturating_pow(40);
pub const BUILDER_PENDING_WITHDRAWALS_LIMIT: usize = 2usize.saturating_pow(20);

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Builder {
	#[cfg_attr(feature = "std", serde(rename = "pubkey"))]
	pub pub_key: BlsPublicKey,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_hex_quantity"))]
	pub version: u8,
	pub execution_address: ExecutionAddress,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub balance: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub deposit_epoch: Epoch,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub withdrawable_epoch: Epoch,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BuilderPendingWithdrawal {
	pub fee_recipient: ExecutionAddress,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub amount: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub builder_index: BuilderIndex,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BuilderPendingPayment {
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub weight: Gwei,
	pub withdrawal: BuilderPendingWithdrawal,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub proposer_index: ValidatorIndex,
}

/// A builder's commitment to produce an execution payload with a given `block_hash`. The payload
/// itself, and with it the execution state root, is revealed later and out of band.
#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutionPayloadBid<const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize> {
	pub parent_block_hash: Hash32,
	pub parent_block_root: Root,
	pub block_hash: Hash32,
	pub prev_randao: Bytes32,
	pub fee_recipient: ExecutionAddress,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub gas_limit: u64,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub builder_index: BuilderIndex,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub slot: Slot,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub value: Gwei,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub execution_payment: Gwei,
	pub blob_kzg_commitments: List<KzgCommitment, MAX_BLOB_COMMITMENTS_PER_BLOCK>,
	pub execution_requests_root: Root,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedExecutionPayloadBid<const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize> {
	pub message: ExecutionPayloadBid<MAX_BLOB_COMMITMENTS_PER_BLOCK>,
	pub signature: BlsSignature,
}

#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PayloadAttestationData {
	pub beacon_block_root: Root,
	#[cfg_attr(feature = "std", serde(with = "serde_hex_utils::as_string"))]
	pub slot: Slot,
	pub payload_present: bool,
	pub blob_data_available: bool,
}

/// The payload timeliness committee's vote on whether a payload was revealed in time. It says
/// nothing about the payload's contents, which is why this client does not rely on it.
#[derive(Default, Debug, SimpleSerialize, codec::Encode, codec::Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PayloadAttestation {
	pub aggregation_bits: Bitvector<PTC_SIZE>,
	pub data: PayloadAttestationData,
	pub signature: BlsSignature,
}
