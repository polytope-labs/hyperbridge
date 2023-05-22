use crate as pallet_ismp;
use crate::*;

use crate::{primitives::ConsensusClientProvider, router::ProxyRouter};
use frame_support::traits::{ConstU32, ConstU64, Get};
use frame_system::EnsureRoot;
use ismp_rs::{
    consensus::ConsensusClient,
    router::{DispatchResult, DispatchSuccess},
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{IdentityLookup, Keccak256},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
            Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
            Ismp: pallet_ismp::{Pallet, Storage, Call, Event<T>},
        }
);

pub struct StateMachineProvider;

impl Get<StateMachine> for StateMachineProvider {
    fn get() -> StateMachine {
        StateMachine::Kusama(100)
    }
}

pub struct ConsensusProvider;

impl ConsensusClientProvider for ConsensusProvider {
    fn consensus_client(
        _id: ConsensusClientId,
    ) -> Result<Box<dyn ConsensusClient>, ismp_rs::error::Error> {
        Ok(Box::new(ismp_testsuite::mocks::MockClient))
    }

    fn challenge_period(_id: ConsensusClientId) -> Duration {
        Duration::from_secs(60 * 60)
    }
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = Keccak256;
    type AccountId = sp_core::sr25519::Public;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<1>;
    type WeightInfo = ();
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    const INDEXING_PREFIX: &'static [u8] = b"ISMP";
    type AdminOrigin = EnsureRoot<sp_core::sr25519::Public>;
    type StateMachine = StateMachineProvider;
    type TimeProvider = Timestamp;
    type IsmpRouter = Router;
    type ConsensusClientProvider = ConsensusProvider;
    type WeightInfo = ();
    type WeightProvider = ();
}

#[derive(Default)]
pub struct ModuleRouter;

impl ISMPRouter for ModuleRouter {
    fn dispatch(&self, request: Request) -> DispatchResult {
        let dest = request.dest_chain();
        let source = request.source_chain();
        let nonce = request.nonce();

        Ok(DispatchSuccess { dest_chain: dest, source_chain: source, nonce })
    }

    fn dispatch_timeout(&self, request: Request) -> DispatchResult {
        let dest = request.dest_chain();
        let source = request.source_chain();
        let nonce = request.nonce();
        Ok(DispatchSuccess { dest_chain: dest, source_chain: source, nonce })
    }

    fn write_response(&self, response: Response) -> DispatchResult {
        let request = &response.request();
        let dest = request.dest_chain();
        let source = request.source_chain();
        let nonce = request.nonce();
        Ok(DispatchSuccess { dest_chain: dest, source_chain: source, nonce })
    }
}

pub struct Router {
    inner: ProxyRouter<Test>,
}

impl Default for Router {
    fn default() -> Self {
        Self { inner: ProxyRouter::<Test>::new(ModuleRouter::default()) }
    }
}

impl ISMPRouter for Router {
    fn dispatch(&self, request: Request) -> DispatchResult {
        self.inner.dispatch(request)
    }

    fn dispatch_timeout(&self, request: Request) -> DispatchResult {
        self.inner.dispatch_timeout(request)
    }

    fn write_response(&self, response: Response) -> DispatchResult {
        self.inner.write_response(response)
    }
}
