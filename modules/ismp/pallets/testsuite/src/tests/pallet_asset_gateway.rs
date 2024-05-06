#![cfg(test)]

use crate::{
	relay_chain::{self, RuntimeOrigin},
	runtime::Test,
	xcm::{MockNet, ParaA, Relay},
};
use alloy_primitives::private::alloy_rlp;
use frame_support::{assert_ok, traits::fungibles::Inspect};
use ismp::{
	host::{Ethereum, StateMachine},
	module::IsmpModule,
	router::{Post, Request, Timeout},
};
use pallet_asset_gateway::{Body, Module};
use sp_core::{ByteArray, H160, H256, U256};
use staging_xcm::v3::{Junction, Junctions, MultiLocation, NetworkId, WeightLimit};
use xcm_simulator::TestExt;
use xcm_simulator_example::ALICE;

const SEND_AMOUNT: u128 = 1000;
const PARA_ID: u32 = 100;
pub type RelayChainPalletXcm = pallet_xcm::Pallet<relay_chain::Runtime>;
#[test]
fn should_dispatch_ismp_request_when_assets_are_received_from_relay_chain() {
	MockNet::reset();

	let beneficiary: MultiLocation = Junctions::X3(
		Junction::AccountId32 { network: None, id: ALICE.into() },
		Junction::AccountKey20 {
			network: Some(NetworkId::Ethereum { chain_id: 1 }),
			key: [1u8; 20],
		},
		Junction::GeneralIndex(60 * 60),
	)
	.into_location();
	let weight_limit = WeightLimit::Unlimited;

	let dest: MultiLocation = Junction::Parachain(PARA_ID).into();
	let asset_id = MultiLocation::parent();

	Relay::execute_with(|| {
		// call extrinsic
		let result = RelayChainPalletXcm::limited_reserve_transfer_assets(
			RuntimeOrigin::signed(ALICE),
			Box::new(dest.clone().into()),
			Box::new(beneficiary.clone().into()),
			Box::new((Junctions::Here, SEND_AMOUNT).into()),
			0,
			weight_limit,
		);
		assert_ok!(result);
	});

	ParaA::execute_with(|| {
		let nonce = pallet_ismp::Nonce::<Test>::get();
		// Assert that a request was dispatched by checking the nonce, it should be 1
		dbg!(nonce);
		assert_eq!(nonce, 1);

		let protocol_fees = pallet_asset_gateway::Pallet::<Test>::protocol_fee_percentage();
		let custodied_amount = SEND_AMOUNT - (protocol_fees * SEND_AMOUNT);

		dbg!(asset_id);

		let total_issuance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id);
		dbg!(total_issuance);
		let pallet_account_balance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::balance(
			asset_id,
			&pallet_asset_gateway::Pallet::<Test>::account_id(),
		);
		dbg!(pallet_account_balance);
		assert_eq!(custodied_amount, pallet_account_balance);
	});
}

#[test]
fn should_process_on_accept_module_callback_correctly() {
	MockNet::reset();

	let beneficiary: MultiLocation = Junctions::X3(
		Junction::AccountId32 { network: None, id: ALICE.into() },
		Junction::AccountKey20 {
			network: Some(NetworkId::Ethereum { chain_id: 1 }),
			key: [1u8; 20],
		},
		Junction::GeneralIndex(60 * 60),
	)
	.into_location();
	let weight_limit = WeightLimit::Unlimited;

	let dest: MultiLocation = Junction::Parachain(PARA_ID).into();
	let asset_id = MultiLocation::parent();

	let alice_balance = Relay::execute_with(|| {
		// call extrinsic
		let result = RelayChainPalletXcm::limited_reserve_transfer_assets(
			RuntimeOrigin::signed(ALICE),
			Box::new(dest.clone().into()),
			Box::new(beneficiary.clone().into()),
			Box::new((Junctions::Here, SEND_AMOUNT).into()),
			0,
			weight_limit,
		);
		assert_ok!(result);
		// return alice's account balance
		pallet_balances::Pallet::<relay_chain::Runtime>::free_balance(&ALICE)
	});

	// Parachain should receive xcm
	ParaA::execute_with(|| {
		let nonce = pallet_ismp::Nonce::<Test>::get();
		// Assert that a request was dispatched by checking the nonce, it should be 1
		assert_eq!(nonce, 1);

		let protocol_fees = pallet_asset_gateway::Pallet::<Test>::protocol_fee_percentage();
		let custodied_amount = SEND_AMOUNT - (protocol_fees * SEND_AMOUNT);

		let total_issuance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id);
		dbg!(total_issuance);
		let pallet_account_balance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::balance(
			asset_id,
			&pallet_asset_gateway::Pallet::<Test>::account_id(),
		);
		dbg!(pallet_account_balance);
		assert_eq!(custodied_amount, pallet_account_balance);
	});

	// Process on accept call back
	let transferred = ParaA::execute_with(|| {
		let amount = 990u128;
		let body = Body {
			amount: {
				let mut bytes = [0u8; 32];
				U256::from(amount).to_big_endian(&mut bytes);
				alloy_primitives::U256::from_be_bytes(bytes)
			},
			asset_id: H256::zero().0.into(),
			redeem: false,
			from: alloy_primitives::B256::from_slice(ALICE.as_slice()),
			to: alloy_primitives::B256::from_slice(ALICE.as_slice()),
		};
		let post = Post {
			source: StateMachine::Bsc,
			dest: StateMachine::Kusama(100),
			nonce: 0,
			from: H160::zero().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 0,
			data: {
				let mut encoded = alloy_rlp::encode(body);
				// Prefix with zero
				encoded.insert(0, 0);
				encoded
			},
		};

		let ismp_module = Module::<Test>::default();
		let initial_total_issuance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id);
		ismp_module.on_accept(post).unwrap();

		let total_issuance_after = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id);
		let amount =
			amount - (pallet_asset_gateway::Pallet::<Test>::protocol_fee_percentage() * amount);
		// Total issuance should have dropped
		assert_eq!(initial_total_issuance - amount, total_issuance_after);
		amount
	});

	Relay::execute_with(|| {
		// Alice's balance on relay chain should have increased by the amount transferred
		let current_balance = pallet_balances::Pallet::<relay_chain::Runtime>::free_balance(&ALICE);
		assert_eq!(current_balance, alice_balance + transferred);
	})
}

#[test]
fn should_process_on_timeout_module_callback_correctly() {
	MockNet::reset();

	let beneficiary: MultiLocation = Junctions::X3(
		Junction::AccountId32 { network: None, id: ALICE.into() },
		Junction::AccountKey20 {
			network: Some(NetworkId::Ethereum { chain_id: 1 }),
			key: [0u8; 20],
		},
		Junction::GeneralIndex(60 * 60),
	)
	.into_location();
	let weight_limit = WeightLimit::Unlimited;

	let dest: MultiLocation = Junction::Parachain(PARA_ID).into();
	let asset_id = MultiLocation::parent();

	let alice_balance = Relay::execute_with(|| {
		// call extrinsic
		let result = RelayChainPalletXcm::limited_reserve_transfer_assets(
			RuntimeOrigin::signed(ALICE),
			Box::new(dest.clone().into()),
			Box::new(beneficiary.clone().into()),
			Box::new((Junctions::Here, SEND_AMOUNT).into()),
			0,
			weight_limit,
		);
		assert_ok!(result);
		// return alice's account balance
		pallet_balances::Pallet::<relay_chain::Runtime>::free_balance(&ALICE)
	});

	// Parachain should receive xcm
	ParaA::execute_with(|| {
		let nonce = pallet_ismp::Nonce::<Test>::get();
		// Assert that a request was dispatched by checking the nonce, it should be 1
		assert_eq!(nonce, 1);

		let protocol_fees = pallet_asset_gateway::Pallet::<Test>::protocol_fee_percentage();
		let custodied_amount = SEND_AMOUNT - (protocol_fees * SEND_AMOUNT);

		let total_issuance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id);
		dbg!(total_issuance);
		let pallet_account_balance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::balance(
			asset_id,
			&pallet_asset_gateway::Pallet::<Test>::account_id(),
		);
		dbg!(pallet_account_balance);
		assert_eq!(custodied_amount, pallet_account_balance);
	});

	// Process on timeout call back
	let transferred = ParaA::execute_with(|| {
		let amount = 990u128;
		let body = Body {
			amount: {
				let mut bytes = [0u8; 32];
				U256::from(amount).to_big_endian(&mut bytes);
				alloy_primitives::U256::from_be_bytes(bytes)
			},
			asset_id: H256::zero().0.into(),
			redeem: false,
			from: alloy_primitives::B256::from_slice(ALICE.as_slice()),
			to: alloy_primitives::B256::from_slice(ALICE.as_slice()),
		};
		let post = Post {
			source: StateMachine::Kusama(100),
			dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
			nonce: 0,
			from: H160::zero().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000 + (60 * 60),
			data: {
				let mut encoded = alloy_rlp::encode(body);
				// Prefix with zero
				encoded.insert(0, 0);
				encoded
			},
		};

		let ismp_module = Module::<Test>::default();
		let initial_total_issuance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id);
		let timeout = Timeout::Request(Request::Post(post));
		ismp_module.on_timeout(timeout).unwrap();

		let total_issuance_after = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id);
		// Total issuance should have dropped
		assert_eq!(initial_total_issuance - amount, total_issuance_after);
		amount
	});

	Relay::execute_with(|| {
		// Alice's balance on relay chain should have increased by the amount transferred
		let current_balance = pallet_balances::Pallet::<relay_chain::Runtime>::free_balance(&ALICE);
		assert_eq!(current_balance, alice_balance + transferred);
	})
}
