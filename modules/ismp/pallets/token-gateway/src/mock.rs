use std::cell::RefCell;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use cumulus_pallet_parachain_system::{AnyRelayNumber, ConsensusHook, RelayChainStateProof};
use cumulus_pallet_parachain_system::consensus_hook::UnincludedSegmentCapacity;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId, relay_chain, XcmpMessageSource};
use crate::*;
use frame_support::{assert_ok, derive_impl, dispatch::{DispatchInfo, GetDispatchInfo}, parameter_types, traits::{ConstU64, OnInitialize}};
use frame_support::traits::{ProcessMessage, ProcessMessageError};
use frame_support::weights::WeightMeter;
use frame_system::EnsureRoot;
use polkadot_parachain_primitives::primitives::XcmpMessageHandler;
use ismp::host::StateMachine;
use ismp::router::IsmpRouter;
use sp_runtime::traits::{BlakeTwo256, IdentityLookup, Keccak256};
use hyperbridge_client_machine::HyperbridgeClientMachine;
use pallet_ismp::offchain::Leaf;
use pallet_mmr::primitives::INDEXING_PREFIX;


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
        // HostExecutive: pallet_ismp_host_executive
	}
);


//mock default config implementation


#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type Hash = H256;
    type RuntimeCall = RuntimeCall;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type AccountStore = System;
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
    fn take_outbound_messages(maximum_channels: usize) -> Vec<(ParaId, Vec<u8>)> {
        let id = ParaId::new(1000);
        let result = vec![(id,vec![])];
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


impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
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

}

impl pallet_mmr::Config for Test {
    const INDEXING_PREFIX: &'static [u8] = INDEXING_PREFIX;
    type Hashing = Keccak256;
    type Leaf = Leaf;
    type ForkIdentifierProvider = Ismp;
}

// impl pallet_ismp_host_executive::Config for Test {
//     type RuntimeEvent = RuntimeEvent;
//     type IsmpHost = Ismp;
// }
impl pallet_ismp::Config for Test
{
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
    type Balance = u64;
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
    fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
        // let module = match id.as_slice() {
        //     YOUR_MODULE_ID => Box::new(()),
        //     _ => Err(ismp::Error::ModuleNotFound(id))?
        // };
        // Ok(module)
        todo!()
    }
}