// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Storage types and the gate trait consulted by the ISMP router.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use ismp::host::StateMachine;
use polkadot_sdk::frame_support::pallet_prelude::*;
use scale_info::TypeInfo;

pub type AppKey = BoundedVec<u8, ConstU32<32>>;

pub type BandwidthBytes = u128;

/// Hard cap on the subscription list per `(chain, app)`. Pushes
/// beyond this evict the oldest entry (FIFO).
pub const MAX_SUBSCRIPTIONS: u32 = 1024;
pub type MaxSubscriptions = ConstU32<MAX_SUBSCRIPTIONS>;

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

/// One purchase, immutable across its lifetime: `remaining_bytes`
/// drains via the gate, `expires_at` is fixed at purchase time and
/// never extends. Repurchases append a new row instead of stacking.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
)]
pub struct Subscription {
	pub tier: TierIndex,
	pub remaining_bytes: BandwidthBytes,
	/// Unix seconds. Gate sweeps entries where `expires_at <= now`.
	pub expires_at: u64,
	/// Unix seconds at insertion — fixes FIFO order under same-block buys.
	pub purchased_at: u64,
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

/// Atomic check-and-deduct across all of an app's live subscriptions
/// on `(chain, app)`. `source` is `request.source` (= the purchase's
/// `app_chain`). Drains FIFO by insertion order.
pub trait BandwidthGate {
	fn try_consume(source: &StateMachine, app: &[u8], bytes: u32) -> Result<(), GateError>;
}
