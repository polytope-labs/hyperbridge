#![cfg(test)]

use polkadot_sdk::*;
use std::sync::Arc;

use crate::{init_tracing, relay_chain::{self, RuntimeOrigin}, runtime::{Test, ALICE}, runtime, xcm::{MockNet, ParaA, Relay}};
use alloy_sol_types::SolValue;
use codec::Encode;
use frame_support::{assert_ok, traits::fungibles::Inspect};
use polkadot_sdk::xcm_simulator::{AllCounted, Asset, AssetId, BuyExecution, DepositAsset, Fungibility, GeneralIndex, PalletInstance, Parachain, Parent, Reanchorable, SetFeesMode, TransferReserveAsset, VersionedXcm, Weight, Wild, Xcm};
use ismp::{
	host::StateMachine,
	module::IsmpModule,
	router::{PostRequest, Request, Timeout},
};
use pallet_token_gateway::{impls::convert_to_erc20, types::Body};
use pallet_xcm_gateway::Module;
use sp_core::{ByteArray, H160, H256};
use staging_xcm::v5::{Junction, Junctions, Location, NetworkId, WeightLimit};
use xcm_simulator::TestExt;
use crate::xcm::ParaB;
use polkadot_sdk::frame_support::traits::fungibles::Mutate;
use crate::runtime::BOB;


const SEND_AMOUNT: u128 = 1000_000_000_0000;
const PARA_ID: u32 = 100;
pub type RelayChainPalletXcm = pallet_xcm::Pallet<relay_chain::Runtime>;

#[test]
fn should_dispatch_ismp_request_when_assets_are_received_from_assethub() {
	init_tracing();
	MockNet::reset();
			let asset_location_on_assethub = Location::new(0, [
				PalletInstance(50),
				GeneralIndex(123),
			]);

		//let asset_location_on_assethub_h256: H256 = sp_io::hashing::keccak_256(&asset_location_on_assethub.encode()).into();

		let asset_id_on_paraa: H256 =
				sp_io::hashing::keccak_256(&Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(123)]).encode())
					.into();


			ParaA::execute_with(|| {
				assert_ok!(runtime::Assets::force_create(
                    runtime::RuntimeOrigin::root(),
                    asset_id_on_paraa.into(),
                    ALICE.into(),
                    true,
                    1
                ));
			});


			ParaB::execute_with(|| {
				let dest = Location::new(1, [Parachain(100)]);
				let beneficiary = Location::new(1, Junctions::X3(Arc::new([
					Junction::AccountId32 { network: None, id: BOB.into() },
					Junction::AccountKey20 {
						network: Some(NetworkId::Ethereum { chain_id: 97 }),
						key: [1u8; 20],
					},
					Junction::GeneralIndex(60 * 60),
				])));

				let context = cumulus_primitives_core::Junctions::X2(Arc::new([
					cumulus_primitives_core::Junction::GlobalConsensus(cumulus_primitives_core::NetworkId::Polkadot),
					cumulus_primitives_core::Junction::Parachain(1000),
				]));

				let assets = Asset {
					id: AssetId(asset_location_on_assethub),
					fun: Fungibility::Fungible(100000),
				};

				let fee_asset = assets.clone().reanchored(&dest, &context).expect("should reanchor");
				let fees = fee_asset.clone();


				let xcm = Xcm(vec![
					BuyExecution { fees, weight_limit:  WeightLimit::Unlimited },
					DepositAsset {
						assets: Wild(AllCounted(2)),
						beneficiary,
					},
				]);

				let message = Xcm(vec![
					SetFeesMode { jit_withdraw: true },
					TransferReserveAsset {
						assets: assets.into(),
						dest,
						xcm,
					},
				]);

				assert_ok!(runtime::PalletXcm::execute(
                    runtime::RuntimeOrigin::signed(ALICE.into()),
                    Box::new(VersionedXcm::from(message)),
                   Weight::from_parts(999_600_000_000_000_000, 10000),
                ));
			});


			ParaA::execute_with(|| {

				let bobs_balance = <runtime::Assets as Inspect<
					<Test as frame_system::Config>::AccountId,
				>>::balance(
					asset_id_on_paraa,
					&BOB,
				);
				dbg!(bobs_balance);

				let nonce = pallet_ismp::Nonce::<Test>::get();
				assert_eq!(nonce, 1);

				let protocol_fees = pallet_xcm_gateway::Pallet::<Test>::protocol_fee_percentage();
				let custodied_amount = SEND_AMOUNT - (protocol_fees * SEND_AMOUNT);

				let pallet_account_balance = <runtime::Assets as Inspect<
					<Test as frame_system::Config>::AccountId,
				>>::balance(
					asset_id_on_paraa.into(),
					&pallet_xcm_gateway::Pallet::<Test>::account_id(),
				);
				assert_eq!(custodied_amount, pallet_account_balance);
			});

}
#[test]
fn should_dispatch_ismp_request_when_assets_are_received_from_relay_chain() {
	MockNet::reset();

	let beneficiary: Location = Junctions::X3(Arc::new([
		Junction::AccountId32 { network: None, id: ALICE.into() },
		Junction::AccountKey20 {
			network: Some(NetworkId::Ethereum { chain_id: 97 }),
			key: [1u8; 20],
		},
		Junction::GeneralIndex(60 * 60),
	]))
	.into_location();
	let weight_limit = WeightLimit::Unlimited;

	let dest: Location = Junction::Parachain(PARA_ID).into();
	let asset_id: H256 = sp_io::hashing::keccak_256(&Location::parent().encode()).into();

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

		let protocol_fees = pallet_xcm_gateway::Pallet::<Test>::protocol_fee_percentage();
		let custodied_amount = SEND_AMOUNT - (protocol_fees * SEND_AMOUNT);

		dbg!(&asset_id);

		let total_issuance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id.clone());
		dbg!(total_issuance);
		let pallet_account_balance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::balance(
			asset_id.clone(),
			&pallet_xcm_gateway::Pallet::<Test>::account_id(),
		);
		dbg!(pallet_account_balance);
		assert_eq!(custodied_amount, pallet_account_balance);
	});
}

#[test]
fn should_process_on_accept_module_callback_correctly() {
	MockNet::reset();

	let beneficiary: Location = Junctions::X3(Arc::new([
		Junction::AccountId32 { network: None, id: ALICE.into() },
		Junction::AccountKey20 {
			network: Some(NetworkId::Ethereum { chain_id: 97 }),
			key: [1u8; 20],
		},
		Junction::GeneralIndex(60 * 60),
	]))
	.into_location();
	let weight_limit = WeightLimit::Unlimited;

	let dest: Location = Junction::Parachain(PARA_ID).into();
	let asset_id: H256 = sp_io::hashing::keccak_256(&Location::parent().encode()).into();

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

		let protocol_fees = pallet_xcm_gateway::Pallet::<Test>::protocol_fee_percentage();
		let custodied_amount = SEND_AMOUNT - (protocol_fees * SEND_AMOUNT);

		let total_issuance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id.clone());
		dbg!(total_issuance);
		let pallet_account_balance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::balance(
			asset_id.clone(),
			&pallet_xcm_gateway::Pallet::<Test>::account_id(),
		);
		dbg!(pallet_account_balance);
		assert_eq!(custodied_amount, pallet_account_balance);
	});

	// Process on accept call back
	let transferred = ParaA::execute_with(|| {
		let protocol_fees = pallet_xcm_gateway::Pallet::<Test>::protocol_fee_percentage();
		let amount = SEND_AMOUNT - (protocol_fees * SEND_AMOUNT);
		let body = Body {
			amount: {
				let bytes = convert_to_erc20(amount, 18, 10).to_big_endian();
				alloy_primitives::U256::from_be_bytes(bytes)
			},
			asset_id: pallet_xcm_gateway::Pallet::<Test>::dot_asset_id().0.into(),
			redeem: false,
			from: alloy_primitives::B256::from_slice(ALICE.as_slice()),
			to: alloy_primitives::B256::from_slice(ALICE.as_slice()),
		};
		let post = PostRequest {
			source: StateMachine::Evm(97),
			dest: StateMachine::Kusama(100),
			nonce: 0,
			from: H160::zero().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 0,
			body: {
				let mut encoded = Body::abi_encode(&body);
				// Prefix with zero
				encoded.insert(0, 0);
				encoded
			},
		};

		let ismp_module = Module::<Test>::default();
		let initial_total_issuance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id.clone());
		ismp_module.on_accept(post).unwrap();

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

#[test]
fn should_process_on_timeout_module_callback_correctly() {
	MockNet::reset();

	let beneficiary: Location = Junctions::X3(Arc::new([
		Junction::AccountId32 { network: None, id: ALICE.into() },
		Junction::AccountKey20 {
			network: Some(NetworkId::Ethereum { chain_id: 97 }),
			key: [0u8; 20],
		},
		Junction::GeneralIndex(60 * 60),
	]))
	.into_location();
	let weight_limit = WeightLimit::Unlimited;

	let dest: Location = Junction::Parachain(PARA_ID).into();
	let asset_id: H256 = sp_io::hashing::keccak_256(&Location::parent().encode()).into();

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

		let protocol_fees = pallet_xcm_gateway::Pallet::<Test>::protocol_fee_percentage();
		let custodied_amount = SEND_AMOUNT - (protocol_fees * SEND_AMOUNT);

		let total_issuance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id.clone());
		dbg!(total_issuance);
		let pallet_account_balance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::balance(
			asset_id.clone(),
			&pallet_xcm_gateway::Pallet::<Test>::account_id(),
		);
		dbg!(pallet_account_balance);
		assert_eq!(custodied_amount, pallet_account_balance);
	});

	// Process on timeout call back
	let transferred = ParaA::execute_with(|| {
		let protocol_fees = pallet_xcm_gateway::Pallet::<Test>::protocol_fee_percentage();
		let amount = SEND_AMOUNT - (protocol_fees * SEND_AMOUNT);
		println!("Refund {amount}");

		let body = Body {
			amount: {
				let bytes = convert_to_erc20(amount, 18, 10).to_big_endian();
				alloy_primitives::U256::from_be_bytes(bytes)
			},
			asset_id: pallet_xcm_gateway::Pallet::<Test>::dot_asset_id().0.into(),
			redeem: false,
			from: alloy_primitives::FixedBytes::<32>::from_slice(ALICE.as_slice()),
			to: alloy_primitives::FixedBytes::<32>::from_slice(&[0u8; 32]),
		};
		let post = PostRequest {
			source: StateMachine::Kusama(100),
			dest: StateMachine::Evm(97),
			nonce: 0,
			from: H160::zero().0.to_vec(),
			to: H160::zero().0.to_vec(),
			timeout_timestamp: 1000 + (60 * 60),
			body: {
				let mut encoded = Body::abi_encode(&body);
				// Prefix with zero
				encoded.insert(0, 0);
				encoded
			},
		};

		let ismp_module = Module::<Test>::default();
		let initial_total_issuance = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id.clone());
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
