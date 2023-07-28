use crate::{
    mocks::*,
    module::{EvmIsmpModule, EVM_HOST_ADDRESS},
};
use alloy_primitives::Address;
use alloy_sol_types::{sol, SolCall, SolType};
use fp_evm::{CreateInfo, FeeCalculator, GenesisAccount};
use frame_support::{
    traits::{GenesisBuild, Get},
    weights::Weight,
};
use frame_system::EventRecord;
use hex_literal::hex;
use ismp_primitives::LeafIndexQuery;
use ismp_rs::{
    host::StateMachine,
    module::IsmpModule,
    router::{Get as GetRequest, GetResponse, Post, PostResponse, Request, Response},
    util::hash_request,
};
use pallet_evm::{runner::Runner, FixedGasWeightMapping, GasWeightMapping};
use pallet_ismp::{host::Host, Event, RequestCommitments};
use sp_core::{
    offchain::{testing::TestOffchainExt, OffchainDbExt, OffchainWorkerExt},
    H160, U256,
};
use std::collections::BTreeMap;

sol! {
    function transfer(
        address to,
        bytes memory dest,
        uint64 amount,
        uint64 timeout,
        uint64 gasLimit
    ) public;

    function dispatchGet(
        bytes memory dest,
        bytes[] memory keys,
        uint64 height,
        uint64 timeout,
        uint64 gasLimit
    ) public;

    function mintTo(address who, uint64 amount) public;

    struct Payload {
        address to;
        address from;
        uint64 amount;
    }
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    let mut accounts = BTreeMap::new();
    accounts.insert(
        H160::from(USER.0 .0),
        GenesisAccount {
            nonce: U256::from(1),
            balance: U256::max_value(),
            storage: Default::default(),
            code: vec![],
        },
    );
    accounts.insert(
        H160::from(EVM_HOST_ADDRESS), // root
        GenesisAccount {
            nonce: U256::from(1),
            balance: U256::max_value(),
            storage: Default::default(),
            code: vec![],
        },
    );

    GenesisBuild::<Test>::assimilate_storage(&pallet_evm::GenesisConfig { accounts }, &mut t)
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    register_offchain_ext(&mut ext);
    ext
}

fn register_offchain_ext(ext: &mut sp_io::TestExternalities) {
    let (offchain, _offchain_state) = TestOffchainExt::with_offchain_db(ext.offchain_db());
    ext.register_extension(OffchainDbExt::new(offchain.clone()));
    ext.register_extension(OffchainWorkerExt::new(offchain));
}

pub const EXAMPLE_CONTRACT: &str = include_str!("../solidity/IsmpDemo.bin");

const USER: Address = Address::new(hex!("d8da6bf26964af9d7eed9e03e53415d37aa96045"));
const HOST: H160 = H160(EVM_HOST_ADDRESS);

/// Verify the the last event emitted
fn assert_event_was_emitted<T: pallet_ismp::Config>(
    generic_event: <T as pallet_ismp::Config>::RuntimeEvent,
) {
    let events = frame_system::Pallet::<T>::events();
    let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
    for EventRecord { event, .. } in events {
        if event == system_event {
            return
        }
    }
    panic!("Event was not emitted")
}

fn deploy_contract(gas_limit: u64, weight_limit: Option<Weight>) -> CreateInfo {
    let info = <Test as pallet_evm::Config>::Runner::create(
        HOST,
        hex::decode(EXAMPLE_CONTRACT.trim_end()).unwrap(),
        U256::zero(),
        gas_limit,
        Some(FixedGasPrice::min_gas_price().0),
        Some(FixedGasPrice::min_gas_price().0),
        None,
        Vec::new(),
        true, // non-transactional
        true, // must be validated
        weight_limit,
        None,
        &<Test as pallet_evm::Config>::config().clone(),
    )
    .expect("Deploy succeeds");

    let call_data = mintToCall { who: USER, amount: 1_000_000_000 }.encode();

    let contract_address = info.value;

    <Test as pallet_evm::Config>::Runner::call(
        HOST,
        contract_address,
        call_data,
        U256::zero(),
        gas_limit,
        Some(FixedGasPrice::min_gas_price().0),
        Some(FixedGasPrice::min_gas_price().0),
        None,
        Vec::new(),
        true, // transactional
        true, // must be validated
        weight_limit,
        None,
        &<Test as pallet_evm::Config>::config().clone(),
    )
    .expect("call succeeds");
    info
}

#[test]
fn post_dispatch() {
    let mut ext = new_test_ext();
    let contract_address = ext.execute_with(|| {
        let gas_limit: u64 = 1_500_000_000;
        let weight_limit = FixedGasWeightMapping::<Test>::gas_to_weight(gas_limit, true);
        let result = deploy_contract(gas_limit, Some(weight_limit));

        let contract_address = result.value;

        let call_data = transferCall {
            to: USER,
            dest: StateMachine::Polkadot(1000).to_string().as_bytes().to_vec(),
            amount: 10_000,
            timeout: 223311228889,
            gasLimit: gas_limit,
        }
        .encode();

        <Test as pallet_evm::Config>::Runner::call(
            H160::from(USER.0 .0),
            contract_address,
            call_data,
            U256::zero(),
            gas_limit,
            Some(FixedGasPrice::min_gas_price().0),
            Some(FixedGasPrice::min_gas_price().0),
            None,
            Vec::new(),
            true, // transactional
            true, // must be validated
            Some(weight_limit),
            None,
            &<Test as pallet_evm::Config>::config().clone(),
        )
        .expect("call succeeds");
        // Check
        assert_event_was_emitted::<Test>(
            Event::Request {
                dest_chain: StateMachine::Polkadot(1000),
                source_chain: <Test as pallet_ismp::Config>::StateMachine::get(),
                request_nonce: 0,
            }
            .into(),
        );
        contract_address
    });

    ext.persist_offchain_overlay();

    ext.execute_with(|| {
        // Assert that the source module for the request is the contract address
        let req = pallet_ismp::Pallet::<Test>::get_request(0).unwrap();
        assert_eq!(req.source_module().to_vec(), contract_address.as_bytes().to_vec())
    })
}

#[test]
fn get_dispatch() {
    let mut ext = new_test_ext();
    let contract_address = ext.execute_with(|| {
        let gas_limit: u64 = 1_500_000_000;
        let weight_limit = FixedGasWeightMapping::<Test>::gas_to_weight(gas_limit, true);
        let result = deploy_contract(gas_limit, Some(weight_limit));

        let contract_address = result.value;

        let call_data = dispatchGetCall {
            dest: StateMachine::Polkadot(2000).to_string().as_bytes().to_vec(),
            keys: vec![vec![1u8; 64]],
            height: 10,
            timeout: 2000,
            gasLimit: gas_limit,
        }
        .encode();

        <Test as pallet_evm::Config>::Runner::call(
            H160::from(USER.0 .0),
            contract_address,
            call_data,
            U256::zero(),
            gas_limit,
            Some(FixedGasPrice::min_gas_price().0),
            Some(FixedGasPrice::min_gas_price().0),
            None,
            Vec::new(),
            true, // transactional
            true, // must be validated
            Some(weight_limit),
            None,
            &<Test as pallet_evm::Config>::config().clone(),
        )
        .expect("call succeeds");
        // Check
        assert_event_was_emitted::<Test>(
            Event::Request {
                dest_chain: StateMachine::Polkadot(2000),
                source_chain: <Test as pallet_ismp::Config>::StateMachine::get(),
                request_nonce: 0,
            }
            .into(),
        );
        contract_address
    });

    ext.persist_offchain_overlay();

    ext.execute_with(|| {
        // Assert that the source module for the request is the contract address
        let req = pallet_ismp::Pallet::<Test>::get_request(0).unwrap();
        assert_eq!(req.source_module().to_vec(), contract_address.as_bytes().to_vec())
    })
}

#[test]
fn on_accept_callback() {
    new_test_ext().execute_with(|| {
        let gas_limit: u64 = 1_500_000_000;
        let weight_limit = FixedGasWeightMapping::<Test>::gas_to_weight(gas_limit, true);
        let result = deploy_contract(gas_limit, Some(weight_limit));

        let contract_address = result.value;

        let handler = EvmIsmpModule::<Test>::default();

        let payload = Payload { to: USER, from: USER, amount: 50000 };

        let post = Post {
            source: <Test as pallet_ismp::Config>::StateMachine::get(),
            dest: StateMachine::Polkadot(2000),
            nonce: 0,
            from: contract_address.as_bytes().to_vec(),
            to: contract_address.as_bytes().to_vec(),
            timeout_timestamp: 1000,
            data: Payload::encode(&payload),
            gas_limit,
        };

        let request_commitment = hash_request::<Host<Test>>(&Request::Post(post.clone()));
        RequestCommitments::<Test>::insert(
            request_commitment.0.to_vec(),
            LeafIndexQuery { source_chain: post.source, dest_chain: post.dest, nonce: 0 },
        );

        handler.on_accept(post).expect("Call succeeds");

        assert_event_was_emitted::<Test>(
            Event::Response {
                dest_chain: <Test as pallet_ismp::Config>::StateMachine::get(),
                source_chain: StateMachine::Polkadot(2000),
                request_nonce: 0,
            }
            .into(),
        );
    })
}

#[test]
fn on_post_response() {
    new_test_ext().execute_with(|| {
        let gas_limit: u64 = 1_500_000_000;
        let weight_limit = FixedGasWeightMapping::<Test>::gas_to_weight(gas_limit, true);
        let result = deploy_contract(gas_limit, Some(weight_limit));

        let contract_address = result.value;

        let handler = EvmIsmpModule::<Test>::default();

        let payload = Payload { to: USER, from: USER, amount: 50000 };

        let post = Post {
            source: <Test as pallet_ismp::Config>::StateMachine::get(),
            dest: StateMachine::Polkadot(2000),
            nonce: 0,
            from: contract_address.as_bytes().to_vec(),
            to: contract_address.as_bytes().to_vec(),
            timeout_timestamp: 1000,
            data: Payload::encode(&payload),
            gas_limit,
        };

        let response = PostResponse { post, response: H160::from_low_u64_be(30).0.to_vec() };

        handler.on_response(Response::Post(response)).expect("Call succeeds")
    })
}

#[test]
fn on_get_response() {
    new_test_ext().execute_with(|| {
        let gas_limit: u64 = 1_500_000_000;
        let weight_limit = FixedGasWeightMapping::<Test>::gas_to_weight(gas_limit, true);
        let result = deploy_contract(gas_limit, Some(weight_limit));

        let contract_address = result.value;

        let handler = EvmIsmpModule::<Test>::default();

        let get = GetRequest {
            source: <Test as pallet_ismp::Config>::StateMachine::get(),
            dest: StateMachine::Polkadot(2000),
            nonce: 0,
            from: contract_address.as_bytes().to_vec(),
            keys: vec![
                H160::from_low_u64_be(10).as_bytes().to_vec(),
                H160::from_low_u64_be(20).as_bytes().to_vec(),
            ],
            height: 10,
            timeout_timestamp: 1000,
            gas_limit,
        };

        let mut values = BTreeMap::new();
        values.insert(
            H160::from_low_u64_be(10).as_bytes().to_vec(),
            Some(H160::from_low_u64_be(10).as_bytes().to_vec()),
        );
        values.insert(
            H160::from_low_u64_be(20).as_bytes().to_vec(),
            Some(H160::from_low_u64_be(20).as_bytes().to_vec()),
        );
        let response = GetResponse { get, values };

        handler.on_response(Response::Get(response)).expect("Call succeeds")
    })
}

#[test]
fn on_get_timeout() {
    new_test_ext().execute_with(|| {
        let gas_limit: u64 = 1_500_000_000;
        let weight_limit = FixedGasWeightMapping::<Test>::gas_to_weight(gas_limit, true);
        let result = deploy_contract(gas_limit, Some(weight_limit));

        let contract_address = result.value;

        let handler = EvmIsmpModule::<Test>::default();

        let get = GetRequest {
            source: <Test as pallet_ismp::Config>::StateMachine::get(),
            dest: StateMachine::Polkadot(2000),
            nonce: 0,
            from: contract_address.as_bytes().to_vec(),
            keys: vec![
                H160::from_low_u64_be(10).as_bytes().to_vec(),
                H160::from_low_u64_be(20).as_bytes().to_vec(),
            ],
            height: 10,
            timeout_timestamp: 1000,
            gas_limit,
        };

        handler.on_timeout(Request::Get(get)).expect("Call succeeds")
    })
}

#[test]
fn on_post_timeout() {
    new_test_ext().execute_with(|| {
        let gas_limit: u64 = 1_500_000_000;
        let weight_limit = FixedGasWeightMapping::<Test>::gas_to_weight(gas_limit, true);
        let result = deploy_contract(gas_limit, Some(weight_limit));

        let contract_address = result.value;

        let handler = EvmIsmpModule::<Test>::default();
        let payload = Payload { to: USER, from: USER, amount: 50000 };
        let post = Post {
            source: <Test as pallet_ismp::Config>::StateMachine::get(),
            dest: StateMachine::Polkadot(2000),
            nonce: 0,
            from: contract_address.as_bytes().to_vec(),
            to: contract_address.as_bytes().to_vec(),
            timeout_timestamp: 1000,
            data: Payload::encode(&payload),
            gas_limit,
        };

        handler.on_timeout(Request::Post(post)).expect("Call succeeds")
    })
}
