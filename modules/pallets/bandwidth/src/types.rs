// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Storage types and the gate trait consulted by the ISMP router.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use ismp::host::StateMachine;
use polkadot_sdk::frame_support::pallet_prelude::*;
use scale_info::TypeInfo;

/// Recipient app identifier on the credit chain. Bounded so it fits
/// inline in storage; usually a 20-byte EVM address right-padded.
pub type AppKey = BoundedVec<u8, ConstU32<32>>;

/// Byte balance unit. `u128` so a single subscription can carry
/// multi-month allowances without overflow concerns.
pub type BandwidthBytes = u128;

/// Hard cap on the subscription list per `(chain, app)`. Pushes
/// beyond this evict the oldest entry (FIFO).
pub const MAX_SUBSCRIPTIONS: u32 = 1024;
pub type MaxSubscriptions = ConstU32<MAX_SUBSCRIPTIONS>;

/// Closed enum of available tier SKUs. Discriminants are stable —
/// must match the EVM side's `tier` field in `BandwidthPurchaseMsg`.
/// Adding a tier means adding a variant on both sides.
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
	Debug,
)]
pub enum TierIndex {
	/// Entry tier — discriminant `1` on the wire.
	TierOne = 1,
	/// Discriminant `2` on the wire.
	TierTwo = 2,
	/// Discriminant `3` on the wire.
	TierThree = 3,
	/// Discriminant `4` on the wire.
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
	Debug,
)]
pub struct TierConfig {
	/// Bytes credited per single-month purchase of this tier. Multi-month
	/// purchases scale this linearly.
	pub bytes: BandwidthBytes,
	/// Window length per single-month purchase, in seconds. Multi-month
	/// purchases scale this linearly.
	pub duration_secs: u64,
}

/// One purchase, immutable across its lifetime: `remaining_bytes`
/// drains via the gate, `expires_at` is fixed at purchase time and
/// never extends. Repurchases append a new row instead of stacking.
#[derive(
	Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Clone, PartialEq, Eq, Debug,
)]
pub struct Subscription {
	/// SKU this subscription was bought against; for analytics/events
	/// only — the gate doesn't look at it during drain.
	pub tier: TierIndex,
	/// Bytes left to spend. Decrements as the gate drains; the entry
	/// is popped once this hits zero.
	pub remaining_bytes: BandwidthBytes,
	/// Unix seconds. Gate sweeps entries where `expires_at <= now`.
	pub expires_at: u64,
	/// Unix seconds at insertion — fixes FIFO order under same-block buys.
	pub purchased_at: u64,
}

/// Admin payload for `force_credit` — bundled into a struct because
/// positional dispatch args beyond two get unreadable fast.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug)]
pub struct ForceCreditParams {
	/// Chain whose `(chain, app)` bucket gets the new subscription.
	pub app_chain: StateMachine,
	/// Recipient app on `app_chain`.
	pub app: AppKey,
	/// Tier label recorded on the subscription; doesn't have to match
	/// a configured `TierConfig` (this is the admin escape hatch).
	pub tier: TierIndex,
	/// Bytes to credit on the new subscription.
	pub bytes: BandwidthBytes,
	/// Window length in seconds — `expires_at = now + duration_secs`.
	pub duration_secs: u64,
}

/// Why the gate refused a request. Surfaces back to the ISMP router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateError {
	/// The app has no live subscriptions on `(chain, app)`.
	NoAllowance,
	/// Live subscriptions exist but the sum of `remaining_bytes` is
	/// short of what the message needs. The gate makes no mutation in
	/// this case — the caller can retry after a top-up.
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
