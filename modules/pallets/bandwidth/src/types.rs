// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Storage types and the gate trait consulted by the ISMP router.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use ismp::host::StateMachine;
use polkadot_sdk::frame_support::pallet_prelude::*;
use scale_info::TypeInfo;

pub type AppKey = BoundedVec<u8, ConstU32<32>>;

pub type BandwidthBytes = u128;

/// Closed enum: storage-keys are bounded by the variant set, so the
/// per-app `BoundedBTreeMap` can never exceed `MaxTiers`. Adding a
/// variant is the only way to grow the tier space.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	RuntimeDebug,
)]
pub enum TierIndex {
	TierOne = 1,
	TierTwo = 2,
	TierThree = 3,
	TierFour = 4,
}

impl TryFrom<u32> for TierIndex {
	type Error = ();
	fn try_from(value: u32) -> Result<Self, Self::Error> {
		match value {
			1 => Ok(TierIndex::TierOne),
			2 => Ok(TierIndex::TierTwo),
			3 => Ok(TierIndex::TierThree),
			4 => Ok(TierIndex::TierFour),
			_ => Err(()),
		}
	}
}

impl From<TierIndex> for u32 {
	fn from(t: TierIndex) -> u32 {
		t as u32
	}
}

/// Upper bound on `BoundedBTreeMap<TierIndex, AllowanceState>`. Keep
/// in sync with the variant count of `TierIndex`.
pub type MaxTiers = ConstU32<4>;

/// A tier is a (bytes, duration) SKU. EVM holds the price; the pallet
/// holds what you get and how long it lasts.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
	Clone,
	Copy,
	PartialEq,
	Eq,
	RuntimeDebug,
)]
pub struct TierConfig {
	pub bytes: BandwidthBytes,
	pub duration_secs: u64,
}

/// One purchase of a single tier. Same-tier re-purchases stack into
/// this row; different tiers live in their own row keyed by `tier`.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
	Clone,
	PartialEq,
	Eq,
	Default,
	RuntimeDebug,
)]
pub struct AllowanceState {
	pub remaining_bytes: BandwidthBytes,
	/// Unix seconds. Gate sweeps rows where `expires_at <= now`.
	pub expires_at: u64,
}

/// Admin payload for `force_credit` — bundled into a struct because
/// positional dispatch args beyond two get unreadable fast.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct ForceCreditParams {
	pub app_chain: StateMachine,
	pub app: AppKey,
	pub tier: TierIndex,
	pub bytes: BandwidthBytes,
	pub duration_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateError {
	NoAllowance,
	Insufficient { remaining: u128, required: u128 },
}

impl core::fmt::Display for GateError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			GateError::NoAllowance => f.write_str("no bandwidth allowance"),
			GateError::Insufficient { remaining, required } => {
				write!(f, "insufficient bandwidth: have {remaining} bytes, needed {required}")
			},
		}
	}
}

/// Atomic check-and-deduct across all of an app's tier buckets on
/// `(chain, app)`. `source` is `request.source` (= the purchase's
/// `app_chain`).
pub trait BandwidthGate {
	fn try_consume(source: &StateMachine, app: &[u8], bytes: u32) -> Result<(), GateError>;
}
