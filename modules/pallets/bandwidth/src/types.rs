// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Storage types and the gating trait the runtime's ISMP router consults.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use ismp::host::StateMachine;
use polkadot_sdk::frame_support::pallet_prelude::*;
use scale_info::TypeInfo;

/// Raw `request.from` bytes — 20 for EVM, 32 for substrate `AccountId`s.
/// Stored as-is so identity matches without per-chain massaging.
pub type AppKey = BoundedVec<u8, ConstU32<32>>;

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
    /// Monotonic — never decremented. For analytics only.
    pub purchased_lifetime: u128,
    pub last_consumed_at: u64,
}

/// Phased rollout state — see §6 of `miniIssue.md`.
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
    /// Gate is a no-op.
    Disabled,
    /// Always returns `Ok`, but emits `WouldReject` on shortfall so apps
    /// can see what the gate would do once flipped to `Enforce`.
    Observe,
    /// Insufficient allowance returns `Err`; success deducts.
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
            GateError::Insufficient { remaining, required } => write!(
                f,
                "insufficient bandwidth: have {remaining} bytes, needed {required}"
            ),
        }
    }
}

/// Atomic check-and-deduct against an app's bandwidth balance. The
/// runtime's ISMP router calls this for every non-protocol message and
/// refuses to process it on `Err`.
pub trait BandwidthGate {
    fn try_consume(source: &StateMachine, app: &[u8], bytes: u32) -> Result<(), GateError>;
}
