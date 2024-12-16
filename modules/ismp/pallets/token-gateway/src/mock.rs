use crate::*;
use cumulus_pallet_parachain_system::{
	consensus_hook::UnincludedSegmentCapacity, AnyRelayNumber, ConsensusHook, ParachainSetCode,
	RelayChainStateProof,
};
use cumulus_primitives_core::{relay_chain, AggregateMessageOrigin, ParaId, XcmpMessageSource};
use frame_support::{
	pallet_prelude::ConstU32,
	parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstU64, OnTimestampSet, ProcessMessage, ProcessMessageError,
	},
	weights::WeightMeter,
	BoundedVec,
};
use frame_system::EnsureRoot;
use ismp::{host::StateMachine, router::IsmpRouter};
use pallet_assets::AutoIncAssetId;
use pallet_ismp::offchain::Leaf;
use pallet_mmr::primitives::INDEXING_PREFIX;
use polkadot_core_primitives::Moment;
use polkadot_parachain_primitives::primitives::XcmpMessageHandler;
use sp_core::{crypto::AccountId32, ConstU128};
use sp_runtime::{traits::Keccak256, BuildStorage};
use std::{cell::RefCell, num::NonZeroU32};

use crate as pallet_token_gateway;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Timestamp: pallet_timestamp,
		Ismp: pallet_ismp::{Pallet, Storage, Call, Event<T>},
		Mmr: pallet_mmr,
		IsmpParachain: ismp_parachain,
		HyperBridge: pallet_hyperbridge,
		MessageQueue: pallet_message_queue,
		CumulusParachain: cumulus_pallet_parachain_system,
		Assets: pallet_assets,
		TokenGateway: pallet_token_gateway::{Pallet, Storage, Call, Event<T>}
	}
);

//mock default config implementation
pub struct TestBlockHashCount<C: Get<u32>>(core::marker::PhantomData<C>);
impl<I: From<u32>, C: Get<u32>> Get<I> for TestBlockHashCount<C> {
	fn get() -> I {
		C::get().into()
	}
}

impl frame_system::Config for Test {
	type Nonce = u32;
	type Hash = sp_core::hash::H256;
	type Hashing = sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = sp_runtime::traits::IdentityLookup<AccountId32>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type Version = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type PalletInfo = PalletInfo;
	type RuntimeTask = RuntimeTask;
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockHashCount = TestBlockHashCount<frame_support::traits::ConstU32<10>>;
	type OnSetCode = ParachainSetCode<Test>;
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
	type Block = Block;
}

impl pallet_balances::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;

	type RuntimeFreezeReason = RuntimeFreezeReason;

	type AccountStore = System;

	type Balance = u128;
	type ExistentialDeposit = ConstU128<1>;

	type ReserveIdentifier = ();
	type FreezeIdentifier = Self::RuntimeFreezeReason;

	type DustRemoval = ();

	type MaxLocks = ConstU32<100>;
	type MaxReserves = ConstU32<100>;
	type MaxFreezes = ConstU32<10>;

	type WeightInfo = ();
}

std::thread_local! {
	pub static HANDLED_DMP_MESSAGES: RefCell<Vec<Vec<u8>>> = RefCell::new(Vec::new());
	pub static HANDLED_XCMP_MESSAGES: RefCell<Vec<(ParaId, relay_chain::BlockNumber, Vec<u8>)>> = RefCell::new(Vec::new());
	pub static SENT_MESSAGES: RefCell<Vec<(ParaId, Vec<u8>)>> = RefCell::new(Vec::new());
}

pub struct SaveIntoThreadLocal;
impl XcmpMessageHandler for SaveIntoThreadLocal {
	fn handle_xcmp_messages<'a, I: Iterator<Item = (ParaId, u32, &'a [u8])>>(
		iter: I,
		_max_weight: Weight,
	) -> Weight {
		HANDLED_XCMP_MESSAGES.with(|m| {
			for (sender, sent_at, message) in iter {
				m.borrow_mut().push((sender, sent_at, message.to_vec()));
			}
			Weight::zero()
		})
	}
}

impl ProcessMessage for SaveIntoThreadLocal {
	type Origin = AggregateMessageOrigin;

	fn process_message(
		message: &[u8],
		origin: Self::Origin,
		_meter: &mut WeightMeter,
		_id: &mut [u8; 32],
	) -> Result<bool, ProcessMessageError> {
		assert_eq!(origin, Self::Origin::Parent);

		HANDLED_DMP_MESSAGES.with(|m| {
			m.borrow_mut().push(message.to_vec());
			Weight::zero()
		});
		Ok(true)
	}
}

pub struct FromThreadLocal;

impl XcmpMessageSource for FromThreadLocal {
	fn take_outbound_messages(_maximum_channels: usize) -> Vec<(ParaId, Vec<u8>)> {
		let id = ParaId::new(1000);
		let result = vec![(id, vec![])];
		result
	}
}

std::thread_local! {
	pub static CONSENSUS_HOOK: RefCell<Box<dyn Fn(&RelayChainStateProof) -> (Weight, UnincludedSegmentCapacity)>>
		= RefCell::new(Box::new(|_| (Weight::zero(), NonZeroU32::new(1).unwrap().into())));
}

pub struct TestConsensusHook;

impl ConsensusHook for TestConsensusHook {
	fn on_state_proof(s: &RelayChainStateProof) -> (Weight, UnincludedSegmentCapacity) {
		CONSENSUS_HOOK.with(|f| f.borrow_mut()(s))
	}
}

impl cumulus_pallet_parachain_system::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = ParachainId;
	type OutboundXcmpMessageSource = FromThreadLocal;
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = SaveIntoThreadLocal;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = AnyRelayNumber;
	type ConsensusHook = TestConsensusHook;
	type WeightInfo = ();
}

impl pallet_message_queue::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	// NOTE that normally for benchmarking we should use the No-OP message processor, but in this
	// case its a mocked runtime and will only be used to generate insecure default weights.
	type MessageProcessor = SaveIntoThreadLocal;
	type Size = u32;
	type QueueChangeHandler = ();
	type QueuePausedQuery = ();
	type HeapSize = sp_core::ConstU32<{ 103 * 1024 }>;
	type MaxStale = sp_core::ConstU32<8>;
	type ServiceWeight = MaxWeight;
	type IdleMaxServiceWeight = ();
	type WeightInfo = ();
}

pub struct MockOnTimestampSet;
impl OnTimestampSet<Moment> for MockOnTimestampSet {
	fn on_timestamp_set(moment: Moment) {
		CapturedMoment::mutate(|x| *x = Some(moment));
	}
}

pub(crate) fn clear_captured_moment() {
	CapturedMoment::mutate(|x| *x = None);
}

pub(crate) fn get_captured_moment() -> Option<Moment> {
	CapturedMoment::get()
}

impl pallet_timestamp::Config for Test {
	type Moment = Moment;
	type OnTimestampSet = MockOnTimestampSet;
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

impl pallet_hyperbridge::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
}

impl ismp_parachain::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
}

parameter_types! {
	// The hyperbridge parachain on Polkadot
	pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Polkadot(3367));
	// The host state machine of this pallet
	pub const HostStateMachine: StateMachine = StateMachine::Polkadot(1000); // your paraId here

	pub const ParachainId: ParaId = ParaId::new(1000);
	pub const ReservedXcmpWeight: Weight = Weight::zero();
	pub const ReservedDmpWeight: Weight = Weight::zero();
	pub const MaxWeight: Weight = Weight::MAX;
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
	pub static CapturedMoment: Option<Moment> = None;

}

impl pallet_mmr::Config for Test {
	const INDEXING_PREFIX: &'static [u8] = INDEXING_PREFIX;
	type Hashing = Keccak256;
	type Leaf = Leaf;
	type ForkIdentifierProvider = Ismp;
}

impl pallet_ismp::Config for Test {
	// configure the runtime event
	type RuntimeEvent = RuntimeEvent;
	// Permissioned origin who can create or update consensus clients
	type AdminOrigin = EnsureRoot<Self::AccountId>;
	// The state machine identifier for this state machine
	type HostStateMachine = HostStateMachine;
	// The pallet_timestamp pallet
	type TimestampProvider = Timestamp;
	// The currency implementation that is offered to relayers
	type Currency = Balances;
	// The balance type for the currency implementation
	type Balance = u128;
	// Router implementation for routing requests/responses to their respective modules
	type Router = Router;
	// Optional coprocessor for incoming requests/responses
	type Coprocessor = Coprocessor;
	// Supported consensus clients
	type ConsensusClients = (
		// as an example, the parachain consensus client
		ismp_parachain::ParachainConsensusClient<Test, IsmpParachain>,
	);
	// Offchain database implementation. Outgoing requests and responses are
	// inserted in this database, while their commitments are stored onchain.
	type OffchainDB = Mmr;
	// Weight provider for local modules
	type WeightProvider = ();
}

#[derive(Default)]
pub struct Router;
impl IsmpRouter for Router {
	fn module_for_id(&self, _id: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
		// let module = match id.as_slice() {
		//     YOUR_MODULE_ID => Box::new(()),
		//     _ => Err(ismp::Error::ModuleNotFound(id))?
		// };
		// Ok(module)
		todo!()
	}
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;

	type Dispatcher = Ismp;

	type NativeCurrency = Balances;

	type AssetAdmin = AssetAdmin;

	type Assets = Assets;

	type NativeAssetId = NativeAssetId;

	type AssetIdFactory = ();

	type Decimals = Decimals;
}

parameter_types! {
	pub const AssetAdmin: AccountId32 = AccountId32::new([0u8;32]);
	// A constant that should represent the native asset id
	pub const NativeAssetId: H256 = H256::zero();
	// Set the correct precision for the native currency
	pub const Decimals: u8 = 12;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type RemoveItemsLimit = ConstU32<5>;
	type AssetId = H256;
	type AssetIdParameter = H256;
	type AssetDeposit = ConstU128<1>;
	type AssetAccountDeposit = ConstU128<10>;
	type MetadataDepositBase = ConstU128<1>;
	type MetadataDepositPerByte = ConstU128<1>;
	type ApprovalDeposit = ConstU128<1>;
	type StringLimit = ConstU32<50>;
	type Extra = ();
	type CallbackHandle = AutoIncAssetId<Test>;
	type WeightInfo = ();
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId32>>;
	type ForceOrigin = EnsureRoot<Self::AccountId>;
	type Freezer = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let asset_id: H256 = H256::zero();

	let t = RuntimeGenesisConfig {
		system: Default::default(),
		balances: pallet_balances::GenesisConfig {
			balances: vec![(AccountId32::from([0u8; 32]), 10000)],
		},
		assets: pallet_assets::GenesisConfig {
			assets: vec![
				// id, owner, is_sufficient, min_balance
				(asset_id, AccountId32::from([0u8; 32]), true, 0),
			],
			metadata: vec![
				// id, name, symbol, decimals
				(asset_id, "Spectre".into(), "SPC".into(), 10),
			],
			accounts: vec![
				// id, account_id, balance
				(asset_id, AccountId32::from([0u8; 32]), 1000),
			],
			next_asset_id: None,
		},
	}
	.build_storage()
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
