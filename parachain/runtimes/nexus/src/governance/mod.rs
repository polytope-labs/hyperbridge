// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;

mod origins;
mod tracks;
use crate::frame_support::traits::fungible::HoldConsideration;
use crate::frame_support::traits::EitherOf;
use crate::frame_support::traits::EitherOfDiverse;
use crate::frame_support::traits::LinearStoragePrice;
use crate::frame_support::traits::MapSuccess;
use crate::frame_support::traits::TryMapSuccess;
use crate::sp_core::TypedGet;
use crate::sp_runtime::morph_types;
use crate::sp_runtime::traits::CheckedSub;
use crate::sp_runtime::traits::ConstU16;
use crate::sp_runtime::traits::Replace;
use crate::sp_runtime::traits::ReplaceWithDefault;
use crate::Preimage;
pub use origins::{
	custom_origins, FellowshipAdmin, ReferendumCanceller, ReferendumKiller, WhitelistedCaller, *,
};
pub use tracks::TracksInfo;

impl origins::custom_origins::Config for Runtime {}

parameter_types! {
	pub const VoteLockingPeriod: BlockNumber = 1 * DAYS;
}

parameter_types! {
	pub const PreimageBaseDeposit: Balance = 5 * UNIT;
	pub const PreimageByteDeposit: Balance = 5 * UNIT;
	pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
	>;
}

impl pallet_conviction_voting::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type VoteLockingPeriod = VoteLockingPeriod;
	type MaxVotes = ConstU32<512>;
	type MaxTurnout =
		frame_support::traits::tokens::currency::ActiveIssuanceOf<Balances, Self::AccountId>;
	type Polls = Referenda;
	type BlockNumberProvider = System;
	type VotingHooks = ();
}

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = 1 * 3 * MILLIUNIT;
	pub const UndecidingTimeout: BlockNumber = 14 * DAYS;
}

parameter_types! {
	pub const MaxBalance: Balance = Balance::max_value();
}
pub type TreasurySpender = EitherOf<EnsureRootWithSuccess<AccountId, MaxBalance>, Spender>;

impl pallet_whitelist::Config for Runtime {
	type WeightInfo = ();
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WhitelistOrigin = EitherOfDiverse<EnsureRoot<Self::AccountId>, FellowshipAdmin>;
	type DispatchWhitelistedOrigin = EitherOf<EnsureRoot<Self::AccountId>, WhitelistedCaller>;
	type Preimages = Preimage;
}

parameter_types! {
	pub MaximumSchedulerWeight: frame_support::weights::Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = ();
	type OriginPrivilegeCmp = frame_support::traits::EqualPrivilegeOnly;
	type Preimages = Preimage;
	type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

impl pallet_referenda::Config for Runtime {
	type WeightInfo = ();
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	type SubmitOrigin = frame_system::EnsureSigned<AccountId>;
	type CancelOrigin = EitherOf<EnsureRoot<AccountId>, ReferendumCanceller>;
	type KillOrigin = EitherOf<EnsureRoot<AccountId>, ReferendumKiller>;
	type Slash = Treasury;
	type Votes = pallet_conviction_voting::VotesOf<Runtime>;
	type Tally = pallet_conviction_voting::TallyOf<Runtime>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = TracksInfo;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}

pub type FellowshipReferendaInstance = pallet_referenda::Instance2;

impl pallet_referenda::Config<FellowshipReferendaInstance> for Runtime {
	type WeightInfo = ();
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	type SubmitOrigin =
		pallet_ranked_collective::EnsureMember<Runtime, FellowshipCollectiveInstance, 1>;
	type CancelOrigin = Fellows;
	type KillOrigin = Fellows;
	type Slash = Treasury;
	type Votes = pallet_ranked_collective::Votes;
	type Tally = pallet_ranked_collective::TallyOf<Runtime, FellowshipCollectiveInstance>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = TracksInfo;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}

pub type FellowshipCollectiveInstance = pallet_ranked_collective::Instance1;

morph_types! {
	/// A `TryMorph` implementation to reduce a scalar by a particular amount, checking for
	/// underflow.
	pub type CheckedReduceBy<N: TypedGet>: TryMorph = |r: N::Type| -> Result<N::Type, ()> {
		r.checked_sub(&N::get()).ok_or(())
	} where N::Type: CheckedSub;
}

impl pallet_ranked_collective::Config<FellowshipCollectiveInstance> for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	// Promotion is by any of:
	// - Root can demote arbitrarily.
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote by the rank *above* the new rank.
	type PromoteOrigin = EitherOf<
		frame_system::EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>,
		EitherOf<
			MapSuccess<FellowshipAdmin, Replace<ConstU16<3>>>,
			TryMapSuccess<origins::EnsureFellowship, CheckedReduceBy<ConstU16<1>>>,
		>,
	>;
	// Demotion is by any of:
	// - Root can demote arbitrarily.
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote by the rank two above the current rank.
	type DemoteOrigin = EitherOf<
		frame_system::EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>,
		EitherOf<
			MapSuccess<FellowshipAdmin, Replace<ConstU16<3>>>,
			TryMapSuccess<origins::EnsureFellowship, CheckedReduceBy<ConstU16<2>>>,
		>,
	>;
	type Polls = FellowshipReferenda;
	type MinRankOfClass = sp_runtime::traits::Identity;
	type VoteWeight = pallet_ranked_collective::Geometric;
	type MemberSwappedHandler = ();
	type ExchangeOrigin = EitherOfDiverse<FellowshipAdmin, EnsureRoot<AccountId>>;
	type AddOrigin = MapSuccess<Self::PromoteOrigin, ReplaceWithDefault<()>>;
	type RemoveOrigin = Self::DemoteOrigin;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = ();
	type MaxMemberCount = ();
}
