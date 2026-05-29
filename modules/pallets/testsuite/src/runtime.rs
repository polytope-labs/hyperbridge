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
	traits::{ConstU128, ConstU32, ConstU64, Get},
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
	router::{GetResponse, IsmpRouter, PostRequest, Request},
	Error,
};
use ismp_sync_committee::constants::sepolia::Sepolia;
use pallet_ismp::{offchain::Leaf, ModuleId};
use polkadot_sdk::{
	frame_support::{traits::FindAuthor, weights::WeightToFee},
	pallet_session::{disabling::UpToLimitDisablingStrategy, SessionHandler},
	sp_runtime::{app_crypto::AppCrypto, traits::OpaqueKeys, Weight},
	xcm_simulator::{GeneralIndex, Junctions::X3, Location, PalletInstance, Parachain},
};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{
	offchain::{testing::TestOffchainExt, OffchainDbExt, OffchainWorkerExt},
	H256,
};
use sp_runtime::{
	traits::{IdentityLookup, Keccak256},
	AccountId32, BuildStorage,
};

use ismp::consensus::IntermediateState;
use polkadot_sdk::frame_support::dispatch::DispatchClass;
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
		Balances: pallet_balances,
		Relayer: pallet_ismp_relayer,
		Fishermen: pallet_fishermen,
		HostExecutive: pallet_ismp_host_executive,
		CallCompressedExecutor: pallet_call_decompressor,
		XcmpQueue: cumulus_pallet_xcmp_queue,
		MessageQueue: pallet_message_queue,
		PalletXcm: pallet_xcm,
		Assets: pallet_assets,
		Sudo: pallet_sudo,
		IsmpSyncCommittee: ismp_sync_committee::pallet,
		IsmpBsc: ismp_bsc::pallet,
		HyperFungibleToken: pallet_hyper_fungible_token,
		Vesting: pallet_vesting,
		BridgeDrop: pallet_bridge_airdrop,
		RelayerIncentives: pallet_consensus_incentives,
		MessagingRelayerIncentives: pallet_messaging_incentives,
		IsmpGrandpa: ismp_grandpa::pallet,
		Session: pallet_session,
		CollatorSelection: pallet_collator_selection,
		CollatorManager: pallet_collator_manager,
		MsgQueue: mock_message_queue,
		Authorship: pallet_authorship,
		IsmpParachain: ismp_parachain,
		Bandwidth: pallet_bandwidth,
		StateCoprocessor: pallet_state_coprocessor,
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
	pub static CollatorSet: alloc::vec::Vec<AccountId32> = alloc::vec::Vec::new();
}

pub struct IsCollatorMock;
impl frame_support::traits::Contains<AccountId32> for IsCollatorMock {
	fn contains(t: &AccountId32) -> bool {
		CollatorSet::get().contains(t)
	}
}

impl pallet_fishermen::Config for Test {
	type IsmpHost = Ismp;
	type IsCollator = IsCollatorMock;
}

impl pallet_sudo::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = ();
}

parameter_types! {
	pub TestBlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::builder()
			.base_block(Weight::from_parts(10_000_000, 0))
			.for_class(DispatchClass::all(), |w| {
				w.base_extrinsic = Weight::from_parts(5_000_000, 0);
				w.max_total = Some(Weight::from_parts(2_000_000_000_000, u64::MAX));
			})
			.build()
			.unwrap();
}

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = ReputationCallFilter;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = Keccak256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type DbWeight = ();
	type BlockWeights = TestBlockWeights;
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
		ismp_grandpa::consensus::GrandpaConsensusClient<Test>,
		ismp_parachain::ParachainConsensusClient<Test, IsmpParachain>,
		ismp_pharos::PharosClient<Ismp, Test, pharos_primitives::Testnet>,
		ismp_beefy::consensus::BeefyConsensusClient<
			Ismp,
			Test,
			substrate_state_machine::SubstrateStateMachine<Test>,
		>,
	);
	type OffchainDB = Mmr;
	type FeeHandler = (
		pallet_consensus_incentives::Pallet<Test>,
		pallet_messaging_incentives::Pallet<Test>,
		pallet_ismp::fee_handler::WeightFeeHandler<
			AccountId32,
			Balances,
			TestWeightToFee,
			TreasuryAccount,
			true,
		>,
	);
	type MigrationWeightInfo = ();
}

impl pallet_bandwidth::Config for Test {
	type Dispatcher = Ismp;
}

impl pallet_state_coprocessor::Config for Test {
	type IsmpHost = Ismp;
	type Mmr = Mmr;
	type BandwidthGate = Bandwidth;
}

parameter_types! {
	pub const Decimals: u8 = 10;
}

pub struct HftNativeAssetId;

impl Get<H256> for HftNativeAssetId {
	fn get() -> H256 {
		sp_io::hashing::keccak_256(b"BRIDGE").into()
	}
}

impl pallet_hyper_fungible_token::Config for Test {
	type Dispatcher = Ismp;
	type Assets = Assets;
	type NativeCurrency = Balances;
	type NativeAssetId = HftNativeAssetId;
	type CreateOrigin = EnsureSigned<AccountId32>;
	type Decimals = Decimals;
	type EvmToSubstrate = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const ReputationAssetId: H256 = H256([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1]);
}
pub type ReputationAsset =
	frame_support::traits::tokens::fungible::ItemOf<Assets, ReputationAssetId, AccountId32>;

/// Mirror of the runtime `ReputationCallFilter` — rejects user-facing
/// `Assets` transfer extrinsics targeting the reputation asset so the
/// mock exercises the same soulbound semantics as nexus/gargantua.
pub struct ReputationCallFilter;
impl frame_support::traits::Contains<RuntimeCall> for ReputationCallFilter {
	fn contains(call: &RuntimeCall) -> bool {
		let rep = ReputationAssetId::get();
		match call {
			RuntimeCall::Assets(
				pallet_assets::Call::transfer { id, .. } |
				pallet_assets::Call::transfer_keep_alive { id, .. } |
				pallet_assets::Call::transfer_all { id, .. } |
				pallet_assets::Call::approve_transfer { id, .. } |
				pallet_assets::Call::transfer_approved { id, .. },
			) => *id != rep,
			_ => true,
		}
	}
}

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
	type Currency = Balances;
	type KeyDeposit = ConstU128<0>;
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const MaxCandidates: u32 = 100;
	pub const MaxInvulnerables: u32 = 20;
	pub const DesiredCollators: u32 = 2;
}

impl pallet_collator_selection::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type UpdateOrigin = EnsureRoot<AccountId32>;
	type PotId = PotId;
	type MaxCandidates = MaxCandidates;
	type MaxInvulnerables = MaxInvulnerables;
	type KickThreshold = ConstU64<1>;
	type ValidatorId = AccountId32;
	type ValidatorIdOf = ConvertInto;
	type ValidatorRegistration = CollatorManager;
	type MinEligibleCollators = DesiredCollators;
	type WeightInfo = ();
}
impl pallet_collator_manager::Config for Test {
	type ReputationAsset = ReputationAsset;
	type Balance = Balance;
	type NativeCurrency = Balances;
	type TreasuryAccount = TreasuryAccount;
	type AdminOrigin = EnsureRoot<AccountId32>;
	type IncentivesManager = MessagingRelayerIncentives;
	type UnbondingPeriod = ConstU64<10>;
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

impl ismp_parachain::Config for Test {
	type IsmpHost = Ismp;
	type WeightInfo = ();
	type RootOrigin = EnsureRoot<AccountId32>;
}

impl ismp_beefy::BeefyClientConfig for Test {
	fn is_parachain_tracked(para_id: u32) -> bool {
		ismp_parachain::Parachains::<Test>::contains_key(para_id)
	}

	fn sp1_vkey_hash() -> primitive_types::H256 {
		Default::default()
	}

	fn allowed_proof_types() -> &'static [u8] {
		&[ismp_beefy::PROOF_TYPE_NAIVE, ismp_beefy::PROOF_TYPE_SP1]
	}
}

parameter_types! {
	pub const TreasuryAccount: PalletId = PalletId(*b"treasury");
}

impl pallet_mmr_tree::Config for Test {
	const INDEXING_PREFIX: &'static [u8] = b"ISMP";
	type Hashing = Keccak256;
	type Leaf = Leaf;
	type ForkIdentifierProvider = Ismp;
}

parameter_types! {
	pub const OutboundRewardTreasury: PalletId = PalletId(*b"ob/rwrds");
}

impl pallet_ismp_relayer::Config for Test {
	type IsmpHost = Ismp;
	type RelayerOrigin = EnsureRoot<AccountId32>;
	type TreasuryPalletId = OutboundRewardTreasury;
}

impl pallet_ismp_host_executive::Config for Test {
	type IsmpHost = Ismp;
	type HostExecutiveOrigin = EnsureRoot<AccountId32>;
}

impl pallet_call_decompressor::Config for Test {
	type MaxCallSize = ConstU32<2>;
	type WeightInfo = ();
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

impl pallet_messaging_incentives::Config for Test {
	type ReputationAsset = ReputationAsset;
	type AdminOrigin = EnsureRoot<AccountId32>;
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

	fn on_response(&self, _response: GetResponse) -> Result<Weight, anyhow::Error> {
		Err(Error::InsufficientProofHeight.into())
	}

	fn on_timeout(&self, _request: Request) -> Result<Weight, anyhow::Error> {
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

	fn on_response(&self, _response: GetResponse) -> Result<Weight, anyhow::Error> {
		Ok(weight())
	}

	fn on_timeout(&self, _request: Request) -> Result<Weight, anyhow::Error> {
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
		proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), IsmpError> {
		// Allows tests to exercise consensus updates that advance no state machine
		// (e.g. a validator-set rotation during sync) by returning an empty
		// commitment map for proofs carrying this sentinel prefix.
		if proof.starts_with(b"__no_state_update__") {
			return Ok((vec![], Default::default()));
		}
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
			// Dedicated id for the request-claim EVM pipeline test: echoes the
			// proof bytes back as the receipt value so the EVM decode branch and
			// signature recovery run end to end. Other EVM ids stay on the mock.
			StateMachine::Evm(11155112) => Box::new(EchoStateMachine),
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
		_commitments: Vec<H256>,
		_root: StateCommitment,
		_proof: &Proof,
	) -> Result<(), IsmpError> {
		Ok(())
	}

	fn commitment_state_trie_key(&self, _commitments: Vec<H256>) -> Vec<Vec<u8>> {
		Default::default()
	}

	fn receipts_state_trie_key(&self, _commitments: Vec<H256>) -> Vec<Vec<u8>> {
		Default::default()
	}

	fn verify_non_membership(
		&self,
		_host: &dyn IsmpHost,
		_commitments: Vec<H256>,
		_root: StateCommitment,
		_proof: &Proof,
	) -> Result<(), IsmpError> {
		Ok(())
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		_keys: Vec<Vec<u8>>,
		_root: H256,
		_proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, IsmpError> {
		Ok(Default::default())
	}
}

/// State machine client that echoes the proof bytes back as the proven value.
/// Lets the request-claim EVM pipeline run end to end (decode branch, signature
/// recovery, payout) without hand-building a real state proof; the proof
/// verification itself is covered by the `evm-state-machine` crate tests.
pub struct EchoStateMachine;

impl StateMachineClient for EchoStateMachine {
	fn verify_membership(
		&self,
		host: &dyn IsmpHost,
		commitments: Vec<H256>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<(), IsmpError> {
		MockStateMachine.verify_membership(host, commitments, root, proof)
	}

	fn commitment_state_trie_key(&self, commitments: Vec<H256>) -> Vec<Vec<u8>> {
		MockStateMachine.commitment_state_trie_key(commitments)
	}

	fn receipts_state_trie_key(&self, commitments: Vec<H256>) -> Vec<Vec<u8>> {
		commitments.into_iter().map(|c| c.0.to_vec()).collect()
	}

	fn verify_non_membership(
		&self,
		host: &dyn IsmpHost,
		commitments: Vec<H256>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<(), IsmpError> {
		MockStateMachine.verify_non_membership(host, commitments, root, proof)
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		keys: Vec<Vec<u8>>,
		_root: H256,
		proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, IsmpError> {
		Ok(keys.into_iter().map(|key| (key, Some(proof.proof.clone()))).collect())
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
			(HyperFungibleToken::pallet_account(), INITIAL_BALANCE),
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

		let hft_contract = vec![0xABu8; 20];
		pallet_hyper_fungible_token::TokenContracts::<Test>::insert(
			StateMachine::Evm(1),
			HftNativeAssetId::get(),
			hft_contract.clone(),
		);
		pallet_hyper_fungible_token::ContractToAsset::<Test>::insert(
			StateMachine::Evm(1),
			hft_contract,
			HftNativeAssetId::get(),
		);
		pallet_hyper_fungible_token::NativeAssets::<Test>::insert(HftNativeAssetId::get(), true);
		pallet_hyper_fungible_token::Precisions::<Test>::insert(
			HftNativeAssetId::get(),
			StateMachine::Evm(1),
			18,
		);

		// Initialize BEEFY consensus state in pallet-ismp storage for outbound proofs
		pallet_ismp::ConsensusStates::<Test>::insert(*b"BEEF", vec![0u8; 32]);
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
