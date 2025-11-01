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

use alloc::collections::BTreeMap;
use cumulus_pallet_parachain_system::ParachainSetCode;
use frame_support::{
	derive_impl, parameter_types,
	traits::{ConstU32, ConstU64, Get},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned, EventRecord};
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, StateCommitment, StateMachineClient,
		StateMachineHeight, StateMachineId, VerifiedCommitments,
	},
	error::Error as IsmpError,
	handlers,
	host::{IsmpHost, StateMachine},
	messaging::{CreateConsensusState, Proof, StateCommitmentHeight},
	module::IsmpModule,
	router::{IsmpRouter, PostRequest, RequestResponse, Response, Timeout},
	Error,
};
use ismp_sync_committee::constants::sepolia::Sepolia;
use pallet_ismp::{offchain::Leaf, ModuleId};
use pallet_token_governor::GatewayParams;
use polkadot_sdk::{
	frame_support::{
		traits::{FindAuthor, LockIdentifier},
		weights::WeightToFee,
	},
	pallet_session::{disabling::UpToLimitDisablingStrategy, SessionHandler},
	sp_runtime::{app_crypto::AppCrypto, traits::OpaqueKeys, Weight},
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

use crate::runtime::sp_runtime::DispatchError;
use hyperbridge_client_machine::HyperbridgeClientMachine;
use ismp::consensus::IntermediateState;
use pallet_messaging_fees::types::PriceOracle;
use substrate_state_machine::SubstrateStateMachine;
use xcm_simulator::mock_message_queue;
pub const ALICE: AccountId32 = AccountId32::new([1; 32]);
pub const BOB: AccountId32 = AccountId32::new([2; 32]);
pub const CHARLIE: AccountId32 = AccountId32::new([3; 32]);
pub const DAVE: AccountId32 = AccountId32::new([4; 32]);

pub const INITIAL_BALANCE: u128 = 1_000_000_000_000_000_000;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		ParachainSystem: cumulus_pallet_parachain_system,
		ParachainInfo: staging_parachain_info,
		Timestamp: pallet_timestamp,
		Mmr: pallet_mmr_tree,
		Ismp: pallet_ismp,
		Hyperbridge: pallet_hyperbridge,
		Balances: pallet_balances,
		Relayer: pallet_ismp_relayer,
		Fishermen: pallet_fishermen,
		HostExecutive: pallet_ismp_host_executive,
		CallCompressedExecutor: pallet_call_decompressor,
		XcmpQueue: cumulus_pallet_xcmp_queue,
		MessageQueue: pallet_message_queue,
		PalletXcm: pallet_xcm,
		Assets: pallet_assets,
		Gateway: pallet_xcm_gateway,
		TokenGovernor: pallet_token_governor,
		Sudo: pallet_sudo,
		IsmpSyncCommittee: ismp_sync_committee::pallet,
		IsmpBsc: ismp_bsc::pallet,
		TokenGateway: pallet_token_gateway,
		TokenGatewayInspector: pallet_token_gateway_inspector,
		Vesting: pallet_vesting,
		BridgeDrop: pallet_bridge_airdrop,
		RelayerIncentives: pallet_consensus_incentives,
		MessagingRelayerIncentives: pallet_messaging_fees,
		IsmpGrandpa: ismp_grandpa::pallet,
		Session: pallet_session,
		CollatorSelection: pallet_collator_selection,
		CollatorManager: pallet_collator_manager,
		MsgQueue: mock_message_queue,
		Authorship: pallet_authorship
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

pub struct StateMachineProvider;

impl Get<StateMachine> for StateMachineProvider {
	fn get() -> StateMachine {
		StateMachine::Kusama(100)
	}
}

/// Balance of an account.
pub type Balance = u128;
// Unit = the base number of indivisible units for balances
pub const UNIT: Balance = 1_000_000_000_000;
pub const MILLIUNIT: Balance = 1_000_000_000;
pub const MICROUNIT: Balance = 1_000_000;

/// The existential deposit. Set to 0.0000001
pub const EXISTENTIAL_DEPOSIT: Balance = 100_000;

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for Test {
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
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

parameter_types! {
	pub const CollatorBondLockId: LockIdentifier = *b"collbond";
}

impl pallet_fishermen::Config for Test {
	type IsmpHost = Ismp;
	type FishermenOrigin = EnsureRoot<AccountId32>;
}

impl pallet_sudo::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = ();
}

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
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
	type OnSetCode = ParachainSetCode<Test>;
	type MaxConsumers = ConstU32<16>;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}

parameter_types! {
	pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Polkadot(3367));
}

pub struct TestWeightToFee;
impl WeightToFee for TestWeightToFee {
	type Balance = Balance;
	fn weight_to_fee(_weight: &Weight) -> Self::Balance {
		50 * UNIT
	}
}

impl pallet_ismp::Config for Test {
	type AdminOrigin = EnsureRoot<AccountId32>;
	type HostStateMachine = StateMachineProvider;
	type Coprocessor = Coprocessor;
	type TimestampProvider = Timestamp;
	type Router = ModuleRouter;
	type Balance = Balance;
	type Currency = Balances;
	type ConsensusClients = (
		MockConsensusClient,
		ismp_sync_committee::SyncCommitteeConsensusClient<Ismp, Sepolia, Test, ()>,
		ismp_bsc::BscClient<Ismp, Test, ismp_bsc::Testnet>,
		ismp_grandpa::consensus::GrandpaConsensusClient<
			Test,
			HyperbridgeClientMachine<Test, Ismp, MessagingRelayerIncentives>,
		>,
	);
	type OffchainDB = Mmr;
	type FeeHandler = (
		pallet_consensus_incentives::Pallet<Test>,
		pallet_messaging_fees::Pallet<Test>,
		pallet_ismp::fee_handler::WeightFeeHandler<
			AccountId32,
			Balances,
			TestWeightToFee,
			TreasuryAccount,
			true,
		>,
	);
}

impl pallet_hyperbridge::Config for Test {
	type IsmpHost = Ismp;
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

pub struct AssetAdmin;

impl Get<<Test as frame_system::Config>::AccountId> for AssetAdmin {
	fn get() -> <Test as frame_system::Config>::AccountId {
		TokenGateway::pallet_account()
	}
}

impl pallet_token_gateway::Config for Test {
	type Dispatcher = Ismp;
	type Assets = Assets;
	type NativeCurrency = Balances;
	type NativeAssetId = NativeAssetId;
	type CreateOrigin = EnsureSigned<AccountId32>;
	type Decimals = Decimals;
	type AssetAdmin = AssetAdmin;
	type EvmToSubstrate = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const ReputationAssetId: H256 = H256([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1]);
}
pub type ReputationAsset =
	frame_support::traits::tokens::fungible::ItemOf<Assets, ReputationAssetId, AccountId32>;

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: AuraId,
	}
}

pub struct TestSessionHandler;
impl SessionHandler<AccountId32> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[AuraId::ID];

	fn on_genesis_session<T: OpaqueKeys>(_validators: &[(AccountId32, T)]) {}

	fn on_new_session<T: OpaqueKeys>(
		_changed: bool,
		_validators: &[(AccountId32, T)],
		_queued_validators: &[(AccountId32, T)],
	) {
	}

	fn on_disabled(_validator_index: u32) {}

	fn on_before_session_ending() {}
}

impl pallet_session::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId32;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = pallet_session::PeriodicSessions<ConstU64<1>, ConstU64<0>>;
	type NextSessionRotation = pallet_session::PeriodicSessions<ConstU64<1>, ConstU64<0>>;
	type SessionManager = CollatorManager;
	type SessionHandler = TestSessionHandler;
	type Keys = SessionKeys;
	type DisablingStrategy = UpToLimitDisablingStrategy;
	type WeightInfo = ();
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const MaxCandidates: u32 = 100;
	pub const MaxInvulnerables: u32 = 20;
	pub const DesiredCollators: u32 = 2;
}

impl pallet_collator_selection::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = CollatorManager;
	type UpdateOrigin = EnsureRoot<AccountId32>;
	type PotId = PotId;
	type MaxCandidates = MaxCandidates;
	type MaxInvulnerables = MaxInvulnerables;
	type KickThreshold = ConstU64<1>;
	type ValidatorId = AccountId32;
	type ValidatorIdOf = ConvertInto;
	type ValidatorRegistration = Session;
	type MinEligibleCollators = DesiredCollators;
	type WeightInfo = ();
}
impl pallet_collator_manager::Config for Test {
	type ReputationAsset = ReputationAsset;
	type Balance = Balance;
	type NativeCurrency = Balances;
	type LockId = CollatorBondLockId;
	type TreasuryAccount = TreasuryAccount;
	type AdminOrigin = EnsureRoot<AccountId32>;
	type IncentivesManager = MessagingRelayerIncentives;
	type WeightInfo = ();
}

pub struct MockFindAuthor;
impl FindAuthor<AccountId32> for MockFindAuthor {
	fn find_author<'a, I>(_digests: I) -> Option<AccountId32>
	where
		I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
	{
		None
	}
}
impl pallet_authorship::Config for Test {
	type FindAuthor = MockFindAuthor;
	type EventHandler = CollatorManager;
}

impl pallet_token_gateway_inspector::Config for Test {
	type GatewayOrigin = EnsureRoot<AccountId32>;
}

impl ismp_sync_committee::pallet::Config for Test {
	type AdminOrigin = EnsureRoot<AccountId32>;
	type IsmpHost = Ismp;
}

impl ismp_bsc::pallet::Config for Test {
	type AdminOrigin = EnsureRoot<AccountId32>;
	type IsmpHost = Ismp;
}

impl ismp_grandpa::Config for Test {
	type IsmpHost = Ismp;
	type WeightInfo = ();
	type RootOrigin = EnsureRoot<AccountId32>;
}

parameter_types! {
	pub const TreasuryAccount: PalletId = PalletId(*b"treasury");
}
impl pallet_token_governor::Config for Test {
	type Dispatcher = Ismp;
	type TreasuryAccount = TreasuryAccount;
	type GovernorOrigin = EnsureRoot<AccountId32>;
}

impl pallet_mmr_tree::Config for Test {
	const INDEXING_PREFIX: &'static [u8] = b"ISMP";
	type Hashing = Keccak256;
	type Leaf = Leaf;
	type ForkIdentifierProvider = Ismp;
}

impl pallet_ismp_relayer::Config for Test {
	type IsmpHost = Ismp;
	type RelayerOrigin = EnsureRoot<AccountId32>;
}

impl pallet_ismp_host_executive::Config for Test {
	type IsmpHost = Ismp;
	type HostExecutiveOrigin = EnsureRoot<AccountId32>;
}

impl pallet_call_decompressor::Config for Test {
	type MaxCallSize = ConstU32<2>;
}

impl pallet_bridge_airdrop::Config for Test {
	type Currency = Balances;
	type BridgeDropOrigin = EnsureRoot<AccountId32>;
}

impl pallet_consensus_incentives::Config for Test {
	type IsmpHost = Ismp;
	type TreasuryAccount = TreasuryAccount;
	type WeightInfo = ();
	type IncentivesOrigin = EnsureRoot<AccountId32>;
	type ReputationAsset = ReputationAsset;
}

impl pallet_messaging_fees::Config for Test {
	type IsmpHost = Ismp;
	type TreasuryAccount = TreasuryAccount;
	type IncentivesOrigin = EnsureRoot<AccountId32>;
	type PriceOracle = MockPriceOracle;
	type WeightInfo = ();
	type ReputationAsset = ReputationAsset;
}

pub struct MockPriceOracle;

impl PriceOracle for MockPriceOracle {
	fn get_bridge_price() -> Result<U256, DispatchError> {
		// return 0.05 with 18 decimals: 0.05 * 10^18
		Ok(U256::from(50_000_000_000_000_000u128))
	}
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 256 * 2;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Test {
	type BlockNumberToBalance = ConvertInto;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	const MAX_VESTING_SCHEDULES: u32 = 3;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = ();
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
}

#[derive(Default)]
pub struct ErrorModule;

impl IsmpModule for ErrorModule {
	fn on_accept(&self, _request: PostRequest) -> Result<Weight, anyhow::Error> {
		Err(Error::InsufficientProofHeight.into())
	}

	fn on_response(&self, _response: Response) -> Result<Weight, anyhow::Error> {
		Err(Error::InsufficientProofHeight.into())
	}

	fn on_timeout(&self, _request: Timeout) -> Result<Weight, anyhow::Error> {
		Err(Error::InsufficientProofHeight.into())
	}
}

#[derive(Default)]
pub struct ModuleRouter;

pub const ERROR_MODULE_ID: &'static [u8] = &[12, 24, 36, 48];

impl IsmpRouter for ModuleRouter {
	fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
		return match id.as_slice() {
			ERROR_MODULE_ID => Ok(Box::new(ErrorModule)),
			_ => Ok(Box::new(MockModule)),
		};
	}
}

/// Mock consensus state id
pub const MOCK_CONSENSUS_STATE_ID: [u8; 4] = *b"mock";

pub const MOCK_CONSENSUS_CLIENT_ID: [u8; 4] = [1u8; 4];

/// module id for the mock benchmarking module
pub const MODULE_ID: ModuleId = ModuleId::Pallet(PalletId(*b"__mock__"));

pub fn set_timestamp<T: pallet_timestamp::Config>(value: u64)
where
	<T as pallet_timestamp::Config>::Moment: From<u64>,
{
	pallet_timestamp::Pallet::<T>::set_timestamp(value.into());
}

/// Mock module
#[derive(Default)]
pub struct MockModule;

fn weight() -> Weight {
	Weight::from_parts(0, 0)
}

impl IsmpModule for MockModule {
	fn on_accept(&self, _request: PostRequest) -> Result<Weight, anyhow::Error> {
		Ok(weight())
	}

	fn on_response(&self, _response: Response) -> Result<Weight, anyhow::Error> {
		Ok(weight())
	}

	fn on_timeout(&self, _request: Timeout) -> Result<Weight, anyhow::Error> {
		Ok(weight())
	}
}

/// A mock consensus client for benchmarking
#[derive(Default)]
pub struct MockConsensusClient;

impl ConsensusClient for MockConsensusClient {
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		_cs_id: ismp::consensus::ConsensusStateId,
		_trusted_consensus_state: Vec<u8>,
		_proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), IsmpError> {
		let verified_commitments: BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> =
			mock_state_commitments();
		Ok((vec![], verified_commitments))
	}

	fn verify_fraud_proof(
		&self,
		_host: &dyn IsmpHost,
		_trusted_consensus_state: Vec<u8>,
		_proof_1: Vec<u8>,
		_proof_2: Vec<u8>,
	) -> Result<(), IsmpError> {
		Ok(())
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		MOCK_CONSENSUS_CLIENT_ID
	}

	fn state_machine(&self, _id: StateMachine) -> Result<Box<dyn StateMachineClient>, IsmpError> {
		let state_machine: Box<dyn StateMachineClient> = match _id {
			StateMachine::Kusama(2000) | StateMachine::Kusama(2001) =>
				Box::new(SubstrateStateMachine::<Test>::default()),
			_ => Box::new(MockStateMachine),
		};
		Ok(state_machine)
	}
}

/// Mock State Machine
pub struct MockStateMachine;

impl StateMachineClient for MockStateMachine {
	fn verify_membership(
		&self,
		_host: &dyn IsmpHost,
		_item: RequestResponse,
		_root: StateCommitment,
		_proof: &Proof,
	) -> Result<(), IsmpError> {
		Ok(())
	}

	fn receipts_state_trie_key(&self, _request: RequestResponse) -> Vec<Vec<u8>> {
		Default::default()
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		_keys: Vec<Vec<u8>>,
		_root: StateCommitment,
		_proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, IsmpError> {
		Ok(Default::default())
	}
}

/// Mock client setup
pub fn setup_mock_client<H: IsmpHost, T: pallet_timestamp::Config>(host: &H) -> StateMachineHeight
where
	<T as pallet_timestamp::Config>::Moment: From<u64>,
{
	let number = frame_system::Pallet::<T>::block_number() + 1u32.into();

	frame_system::Pallet::<T>::reset_events();
	frame_system::Pallet::<T>::initialize(&number, &Default::default(), &Default::default());
	frame_system::Pallet::<T>::finalize();
	set_timestamp::<T>(1000_000);
	let intermediate_state = IntermediateState {
		height: StateMachineHeight {
			id: StateMachineId {
				state_id: StateMachine::Evm(1),
				consensus_state_id: MOCK_CONSENSUS_STATE_ID,
			},
			height: 3,
		},
		commitment: StateCommitment {
			timestamp: 1000,
			overlay_root: None,
			state_root: Default::default(),
		},
	};
	handlers::create_client(
		host,
		CreateConsensusState {
			consensus_state: vec![],
			consensus_client_id: MOCK_CONSENSUS_CLIENT_ID,
			consensus_state_id: MOCK_CONSENSUS_STATE_ID,
			unbonding_period: 1_000_000,
			challenge_periods: vec![(StateMachine::Evm(1), 0)].into_iter().collect(),
			state_machine_commitments: vec![(
				intermediate_state.height.id,
				StateCommitmentHeight { commitment: intermediate_state.commitment, height: 3 },
			)],
		},
	)
	.unwrap();
	host.store_state_machine_update_time(
		intermediate_state.height,
		core::time::Duration::from_millis(1000_000),
	)
	.unwrap();
	host.store_consensus_state(MOCK_CONSENSUS_STATE_ID, vec![]).unwrap();
	host.store_consensus_state_id(
		MOCK_CONSENSUS_STATE_ID,
		ismp_testsuite::mocks::MOCK_CONSENSUS_CLIENT_ID,
	)
	.unwrap();
	host.store_state_machine_commitment(intermediate_state.height, intermediate_state.commitment)
		.unwrap();

	set_timestamp::<T>(1000_000_000);
	intermediate_state.height
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let _ = env_logger::builder().is_test(true).try_init();

	let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(ALICE, INITIAL_BALANCE),
			(TokenGateway::pallet_account(), INITIAL_BALANCE),
			(BridgeDrop::account_id(), INITIAL_BALANCE * 5000),
		],
		..Default::default()
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(storage);
	register_offchain_ext(&mut ext);

	ext.execute_with(|| {
		System::set_block_number(1);
		let protocol_params =
			pallet_token_governor::Params::<Balance> { registration_fee: Default::default() };

		pallet_token_governor::ProtocolParams::<Test>::put(protocol_params);
		pallet_token_gateway::SupportedAssets::<Test>::insert(NativeAssetId::get(), H256::zero());
		pallet_token_gateway::NativeAssets::<Test>::insert(NativeAssetId::get(), true);
		pallet_token_gateway::LocalAssets::<Test>::insert(H256::zero(), NativeAssetId::get());
		pallet_token_gateway::Precisions::<Test>::insert(
			NativeAssetId::get(),
			StateMachine::Evm(1),
			18,
		);
		pallet_token_gateway::TokenGatewayAddresses::<Test>::insert(
			StateMachine::Evm(1),
			H160::zero().0.to_vec(),
		);
		pallet_token_governor::StandaloneChainAssets::<Test>::insert(
			StateMachine::Kusama(100),
			H256::zero(),
			true,
		);

		let params = GatewayParams {
			address: H160::zero(),
			host: H160::zero(),
			call_dispatcher: H160::random(),
		};
		pallet_token_governor::TokenGatewayParams::<Test>::insert(StateMachine::Evm(1), params);
	});
	ext
}

pub fn register_offchain_ext(ext: &mut sp_io::TestExternalities) {
	let (offchain, _offchain_state) = TestOffchainExt::with_offchain_db(ext.offchain_db());
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
}

fn mock_state_commitments() -> BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> {
	let mut map = BTreeMap::new();

	let state_commitment = StateCommitment {
		timestamp: 1_600_000_000,
		overlay_root: Some(H256::repeat_byte(1)),
		state_root: H256::repeat_byte(2),
	};

	let height_entry = StateCommitmentHeight { commitment: state_commitment, height: 42 };

	map.insert(
		StateMachineId {
			state_id: StateMachine::Polkadot(1000),
			consensus_state_id: MOCK_CONSENSUS_STATE_ID,
		},
		vec![height_entry.clone()],
	);

	map
}
