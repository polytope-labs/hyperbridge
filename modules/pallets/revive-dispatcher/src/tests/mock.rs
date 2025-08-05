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

use codec::Encode;
use frame_support::{
	derive_impl, parameter_types,
	traits::{fungible, ConstU128, ConstU32, Get},
};
use frame_system::EnsureRoot;
use hex_literal::hex;
use ismp::{host::StateMachine, module::IsmpModule, router::PostRequest};
use pallet_hyperbridge::{SubstrateHostParams, VersionedHostParams};
use pallet_revive::precompiles::alloy::primitives::Address;
use polkadot_sdk::*;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Runtime>;
pub type Balance = u128;
pub type Test = Runtime;

// Configure a mock runtime to test the pallet.
#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Runtime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(1)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(2)]
	pub type Assets = pallet_assets<Instance1>;

	#[runtime::pallet_index(3)]
	pub type Timestamp = pallet_timestamp;

	#[runtime::pallet_index(4)]
	pub type Revive = pallet_revive;

	#[runtime::pallet_index(5)]
	pub type Ismp = pallet_ismp;

	#[runtime::pallet_index(6)]
	pub type Hyperbridge = pallet_hyperbridge;
}

// The runtime macro generates all these types automatically

// TrustBackedAssets instance for ERC20 precompile
pub type TrustBackedAssetsInstance = pallet_assets::Instance1;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = Balance;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
}

parameter_types! {
	pub const AssetDeposit: Balance = 1;
	pub const ApprovalDeposit: Balance = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 1;
	pub const MetadataDepositPerByte: Balance = 1;
}

// Since we're using Instance1 for TrustBackedAssets, configure it properly
impl pallet_assets::Config<TrustBackedAssetsInstance> for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = u32;
	type AssetIdParameter = u32;
	type Currency = Balances;
	type CreateOrigin =
		frame_support::traits::AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId32>>;
	type ForceOrigin = EnsureRoot<AccountId32>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<10>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Holder = ();
	type WeightInfo = ();
	type CallbackHandle = ();
	type Extra = ();
	type RemoveItemsLimit = ConstU32<1000>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const DepositPerItem: Balance = 1;
	pub const DepositPerByte: Balance = 1;
}

#[derive_impl(pallet_revive::config_preludes::TestDefaultConfig)]
impl pallet_revive::Config for Test {
	type AddressMapper = pallet_revive::AccountId32Mapper<Self>;
	type Currency = Balances;
	type Time = Timestamp;
	type UploadOrigin = frame_system::EnsureSigned<AccountId32>;
	type InstantiateOrigin = frame_system::EnsureSigned<AccountId32>;
	type Precompiles = (
		pallet_assets::precompiles::ERC20<
			Self,
			pallet_assets::precompiles::InlineIdConfig<0x120>,
			pallet_assets::Instance1,
		>,
		crate::ReviveDispatcher<Self, pallet_hyperbridge::Pallet<Self>, FeeTokenAddress>,
	);
}

parameter_types! {
	pub const HostStateMachine: ismp::host::StateMachine = ismp::host::StateMachine::Kusama(2000);
	pub const FeeTokenId: u32 = 0x127;
}
pub struct Coprocessor;

impl Get<Option<StateMachine>> for Coprocessor {
	fn get() -> Option<StateMachine> {
		Some(HostStateMachine::get())
	}
}

impl pallet_ismp::Config for Test {
	type AdminOrigin = EnsureRoot<AccountId32>;
	type HostStateMachine = HostStateMachine;
	type Coprocessor = Coprocessor;
	type TimestampProvider = Timestamp;
	type Balance = Balance;
	type Currency = fungible::ItemOf<Assets, FeeTokenId, AccountId32>;
	type Router = MockRouter;
	type ConsensusClients = ();
	type OffchainDB = ();
	type FeeHandler = pallet_ismp::fee_handler::WeightFeeHandler<()>;
}

impl pallet_hyperbridge::Config for Test {
	type IsmpHost = pallet_ismp::Pallet<Test>;
}

// Mock ISMP Router
#[derive(Default)]
pub struct MockRouter;
impl ismp::router::IsmpRouter for MockRouter {
	fn module_for_id(
		&self,
		id: Vec<u8>,
	) -> Result<Box<dyn ismp::module::IsmpModule>, anyhow::Error> {
		return match id.as_slice() {
			pallet_hyperbridge::PALLET_HYPERBRIDGE_ID =>
				Ok(Box::new(pallet_hyperbridge::Pallet::<Runtime>::default())),
			_ => Err(ismp::Error::ModuleNotFound(id))?,
		};
	}
}

// Build genesis storage
// Helper type for fee token address parameter
parameter_types! {
	pub FeeTokenAddress: Address = Address::from(hex!("0000000000000000000000000000000001270000"));
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(AccountId32::new([1u8; 32]), 1000000),
			(AccountId32::new([2u8; 32]), 1000000),
			(AccountId32::new([3u8; 32]), 1000000),
		],
		..Default::default()
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(storage);
	ext.execute_with(|| {
		System::set_block_number(1);
		Timestamp::set_timestamp(1);

		let module = pallet_hyperbridge::Pallet::<Runtime>::default();

		module
			.on_accept(PostRequest {
				source: Coprocessor::get().unwrap(),
				dest: <Runtime as pallet_ismp::Config>::HostStateMachine::get(),
				body: pallet_hyperbridge::Message::<AccountId32, Balance>::UpdateHostParams(
					VersionedHostParams::V1(SubstrateHostParams {
						default_per_byte_fee: 100,
						..Default::default()
					}),
				)
				.encode(),
				from: vec![],
				to: vec![],
				nonce: 0,
				timeout_timestamp: 0,
			})
			.unwrap();
	});
	ext
}
