// Copyright (c) 2025 Polytope Labs.
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
#![allow(missing_docs, dead_code)]

extern crate alloc;
use polkadot_sdk::{frame_support::traits::WithdrawReasons, sp_runtime::traits::ConvertInto, *};

use cumulus_pallet_parachain_system::ParachainSetCode;
use frame_support::{
	derive_impl, parameter_types,
	traits::{ConstU32, ConstU64, Get},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned, EventRecord};
use polkadot_sdk::{
	pallet_session::{disabling::UpToLimitDisablingStrategy, SessionHandler},
	sp_runtime::{app_crypto::AppCrypto, traits::OpaqueKeys},
	xcm_simulator::{GeneralIndex, Junctions::X3, Location, PalletInstance, Parachain},
};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{
	offchain::{testing::TestOffchainExt, OffchainDbExt, OffchainWorkerExt},
	H160, H256, U256,
};
use sp_runtime::{
	traits::{IdentityLookup, Keccak256},
	AccountId32, BuildStorage,
};

use crate::asset_hub_runtime::sp_runtime::DispatchError;
use pallet_xcm_gateway::xcm_utilities::ASSET_HUB_PARA_ID;
use xcm_simulator::mock_message_queue;
pub const ALICE: AccountId32 = AccountId32::new([1; 32]);
pub const BOB: AccountId32 = AccountId32::new([2; 32]);
pub const CHARLIE: AccountId32 = AccountId32::new([3; 32]);
pub const DAVE: AccountId32 = AccountId32::new([4; 32]);

pub const INITIAL_BALANCE: u128 = 1_000_000_000_000_000_000;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<AssetHubTest>;
type Block = frame_system::mocking::MockBlock<AssetHubTest>;

frame_support::construct_runtime!(
	pub enum AssetHubTest {
		System: frame_system,
		ParachainSystem: cumulus_pallet_parachain_system,
		ParachainInfo: staging_parachain_info,
		Balances: pallet_balances,
		XcmpQueue: cumulus_pallet_xcmp_queue,
		MessageQueue: pallet_message_queue,
		PalletXcm: pallet_xcm,
		Assets: pallet_assets,
		MsgQueue: mock_message_queue
	}
);

/// Verify the the last event emitted
pub fn assert_last_event<T: frame_system::Config>(generic_event: T::RuntimeEvent) {
	assert_eq!(last_event::<T>(), generic_event);
}

/// Verify the the last event emitted
pub fn last_event<T: frame_system::Config>() -> T::RuntimeEvent {
	let events = frame_system::Pallet::<T>::events();
	let EventRecord { event, .. } = &events[events.len() - 1];
	event.clone()
}

/// Balance of an account.
pub type Balance = u128;
// Unit = the base number of indivisible units for balances
pub const UNIT: Balance = 1_000_000_000_000;
pub const MILLIUNIT: Balance = 1_000_000_000;
pub const MICROUNIT: Balance = 1_000_000;

/// The existential deposit. Set to 1/10 of the Connected Relay Chain.
pub const EXISTENTIAL_DEPOSIT: Balance = MILLIUNIT;

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for AssetHubTest {
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<AssetHubTest>;
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type MaxFreezes = ();
	type DoneSlashHandler = ();
}

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for AssetHubTest {
	type BaseCallFilter = frame_support::traits::Everything;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = Keccak256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type DbWeight = ();
	type BlockWeights = ();
	type RuntimeTask = ();
	type BlockLength = ();
	type Version = ();
	type Nonce = u64;
	type Block = Block;
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ParachainSetCode<AssetHubTest>;
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const Decimals: u8 = 10;
}

pub struct NativeAssetId;

impl Get<H256> for NativeAssetId {
	fn get() -> H256 {
		sp_io::hashing::keccak_256(b"BRIDGE").into()
	}
}

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: AuraId,
	}
}

pub fn register_offchain_ext(ext: &mut sp_io::TestExternalities) {
	let (offchain, _offchain_state) = TestOffchainExt::with_offchain_db(ext.offchain_db());
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
}
