use crate as pallet_ismp;
use crate::*;

use crate::{primitives::ConsensusClientProvider, router::ProxyRouter};
use frame_support::traits::{ConstU32, ConstU64, Get};
use frame_system::EnsureRoot;
use ismp_rs::consensus::ConsensusClient;
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
        StateMachine::Kusama(2000)
    }
}

pub struct ConsensusProvider;

impl ConsensusClientProvider for ConsensusProvider {
    fn consensus_client(
        id: ConsensusClientId,
    ) -> Result<Box<dyn ConsensusClient>, ismp_rs::error::Error> {
        let client = match id {
            _ => Err(ismp_rs::error::Error::ImplementationSpecific(
                "Unknown consensus client".into(),
            ))?,
        };

        Ok(client)
    }

    fn challenge_period(id: ConsensusClientId) -> Duration {
        match id {
            _ => Duration::MAX,
        }
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
    type IsmpRouter = ProxyRouter<Test>;
    type ConsensusClientProvider = ConsensusProvider;
}
