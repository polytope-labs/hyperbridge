// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Storage types and the gate trait consulted by the ISMP router.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use ismp::host::StateMachine;
use polkadot_sdk::frame_support::pallet_prelude::*;
use scale_info::TypeInfo;

pub type AppKey = BoundedVec<u8, ConstU32<32>>;

pub type TierIndex = u32;
pub type BandwidthBytes = u128;

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
	pub remaining_bytes: u128,
	/// Monotonic, never decremented. Analytics only.
	pub purchased_lifetime: u128,
	pub last_consumed_at: u64,
}

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
pub enum EnforcementMode {
	/// Gate no-ops.
	Disabled,
	/// Always `Ok`, but emits `WouldReject` on shortfall so apps can
	/// preview what `Enforce` will reject.
	Observe,
	/// Shortfall returns `Err`; success deducts.
	Enforce,
}

impl Default for EnforcementMode {
	fn default() -> Self {
		EnforcementMode::Disabled
	}
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

/// Atomic check-and-deduct on an app's balance. `source` is
/// `request.source` (= the purchase's `app_chain`).
pub trait BandwidthGate {
	fn try_consume(source: &StateMachine, app: &[u8], bytes: u32) -> Result<(), GateError>;
}
