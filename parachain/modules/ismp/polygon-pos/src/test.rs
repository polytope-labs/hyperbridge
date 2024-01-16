use codec::Encode;
use frame_support::crypto::ecdsa::ECDSAExt;

use crate::{ConsensusState, PolygonClient};
use frame_support::traits::{ConstU32, ConstU64, Get};
use frame_system::EnsureRoot;
use geth_primitives::{CodecHeader, Header};
use ismp::{
    consensus::{ConsensusClient, ConsensusClientId},
    host::StateMachine,
    module::IsmpModule,
    router::IsmpRouter,
};
use pallet_ismp::{
    host::Host,
    mocks::{
        mocks::{MockConsensusClient, MockModule},
        ExistentialDeposit,
    },
    primitives::ConsensusClientProvider,
};
use sp_core::{Pair, H256};
use sp_runtime::traits::{IdentityLookup, Keccak256};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Ismp: pallet_ismp::{Pallet, Storage, Call, Event<T>},
        IsmpPolygonPos: crate::pallet,
        Balances: pallet_balances,
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
    ) -> Result<Box<dyn ConsensusClient>, ismp::error::Error> {
        Ok(Box::new(MockConsensusClient))
    }
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Hash = H256;
    type Hashing = Keccak256;
    type AccountId = sp_core::sr25519::Public;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Version = ();
    type Nonce = u64;
    type Block = Block;
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<pallet_ismp::mocks::Balance>;
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

impl pallet_balances::Config for Test {
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
    /// The type for recording an account's balance.
    type Balance = pallet_ismp::mocks::Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type ReserveIdentifier = [u8; 8];
    type FreezeIdentifier = ();
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type MaxHolds = ConstU32<1>;
    type MaxFreezes = ();
}

impl pallet_ismp::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    const INDEXING_PREFIX: &'static [u8] = b"ISMP";
    type AdminOrigin = EnsureRoot<sp_core::sr25519::Public>;
    type StateMachine = pallet_ismp::mocks::StateMachineProvider;
    type TimeProvider = Timestamp;
    type IsmpRouter = pallet_ismp::mocks::ModuleRouter;
    type ConsensusClientProvider = pallet_ismp::mocks::ConsensusProvider;
    type WeightInfo = ();
    type WeightProvider = ();
}

impl crate::pallet::Config for Test {}

#[derive(Default)]
pub struct ModuleRouter;

impl IsmpRouter for ModuleRouter {
    fn module_for_id(&self, _bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, ismp::error::Error> {
        Ok(Box::new(MockModule))
    }
}

#[test]
fn verify_fraud_proof() {
    let mut validators = vec![];
    for _ in 0..5u64 {
        let pair = sp_core::ecdsa::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
        // let key = pair.public();
        validators.push((pair.public().to_eth_address().unwrap().into(), pair))
    }

    let consensus_state = ConsensusState {
        frozen_height: None,
        finalized_hash: Default::default(),
        finalized_validators: validators.iter().map(|(signer, _)| *signer).collect(),
        forks: vec![],
        ismp_contract_address: Default::default(),
    };

    let header = CodecHeader {
        parent_hash: H256::random(),
        uncle_hash: H256::random(),
        coinbase: Default::default(),
        state_root: H256::random(),
        transactions_root: H256::random(),
        receipts_root: H256::random(),
        logs_bloom: Default::default(),
        difficulty: Default::default(),
        number: 200u64.into(),
        gas_limit: 30_000_000,
        gas_used: 20_000_000,
        timestamp: 1000,
        extra_data: vec![0; 32],
        mix_hash: Default::default(),
        nonce: Default::default(),
        base_fee_per_gas: None,
        withdrawals_hash: None,
    };

    // Fraud Proof Scenario 1: Different blocks same signer
    let mut header_1 = header.clone();
    header_1.parent_hash = H256::random();
    let rlp_header_1 = Header::from(&header_1);
    let mut header_2 = header.clone();
    header_2.parent_hash = H256::random();
    let rlp_header_2 = Header::from(&header_2);
    let signature_1 = {
        let msg_1 = rlp_header_1.hash::<Host<Test>>();
        let signer = &validators[0].1;
        signer.sign_prehashed(&msg_1.0).0
    };

    let signature_2 = {
        let msg_1 = rlp_header_2.hash::<Host<Test>>();
        let signer = &validators[0].1;
        signer.sign_prehashed(&msg_1.0).0
    };

    header_1.extra_data.extend_from_slice(&signature_1);
    header_2.extra_data.extend_from_slice(&signature_2);

    let client = PolygonClient::<Test, Host<Test>>::default();
    let host = Host::<Test>::default();

    assert!(client
        .verify_fraud_proof(&host, consensus_state.encode(), header_1.encode(), header_2.encode())
        .is_ok());

    // Fraud proof scenario 2: in turn difficulty in two competing headers
    let mut header_1 = header.clone();
    header_1.gas_used = 10_000_000;
    header_1.difficulty = (consensus_state.finalized_validators.len() as u64).into();
    let rlp_header_1 = Header::from(&header_1);
    let mut header_2 = header.clone();
    header_2.gas_used = 15_000_000;
    header_2.difficulty = (consensus_state.finalized_validators.len() as u64).into();
    let rlp_header_2 = Header::from(&header_2);
    let signature_1 = {
        let msg_1 = rlp_header_1.hash::<Host<Test>>();
        let signer = &validators[0].1;
        signer.sign_prehashed(&msg_1.0).0
    };

    let signature_2 = {
        let msg_1 = rlp_header_2.hash::<Host<Test>>();
        let signer = &validators[1].1;
        signer.sign_prehashed(&msg_1.0).0
    };

    header_1.extra_data.extend_from_slice(&signature_1);
    header_2.extra_data.extend_from_slice(&signature_2);

    assert!(client
        .verify_fraud_proof(&host, consensus_state.encode(), header_1.encode(), header_2.encode())
        .is_ok());
}
