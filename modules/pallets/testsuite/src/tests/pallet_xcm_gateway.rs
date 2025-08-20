#![cfg(test)]

use polkadot_sdk::*;
use std::sync::Arc;

use crate::{
	init_tracing,
	relay_chain::{self, RuntimeOrigin},
	runtime,
	runtime::{Test, ALICE, BOB, Assets, PalletXcm},
	xcm::{MockNet, ParaA, ParaB, Relay},
};
use alloy_sol_types::SolValue;
use codec::Encode;
use frame_support::{assert_ok, traits::fungibles::Inspect};
use ismp::{
	host::StateMachine,
	module::IsmpModule,
	router::{PostRequest, Request, Timeout},
};
use pallet_token_gateway::{impls::convert_to_erc20, types::Body};
use pallet_xcm_gateway::Module;
use polkadot_sdk::{
	frame_support::traits::fungibles::Mutate,
	sp_runtime::traits::AccountIdConversion,
	staging_xcm::latest::AssetTransferFilter,
	xcm_simulator::{
		All, AllCounted, Asset, AssetFilter, AssetId, BuyExecution, DepositAsset, Fungibility,
		GeneralIndex, Here, InitiateTransfer, PalletInstance, ParaId, Parachain, Parent,
		Reanchorable, SetFeesMode, TransferAsset, TransferReserveAsset, VersionedXcm, Weight, Wild,
		Xcm,
	},
};
use sp_core::{crypto::AccountId32, ByteArray, H160, H256};
use staging_xcm::v5::{Junction, Junctions, Location, NetworkId, WeightLimit};
use xcm_simulator::TestExt;
use pallet_xcm_gateway::xcm_utilities::ASSET_HUB_PARA_ID;
use crate::runtime::ReputationAssetId;

const SEND_AMOUNT: u128 = 1000_000_000_0000;
const PARA_ID: u32 = crate::xcm::SIBLING_PARA_ID;
pub type RelayChainPalletXcm = pallet_xcm::Pallet<relay_chain::Runtime>;

#[test]
fn should_dispatch_ismp_request_when_assets_are_received_from_assethub() {
	init_tracing();
	MockNet::reset();
			let asset_location_on_assethub = Location::new(1, Here);

		//let asset_location_on_assethub_h256: H256 = sp_io::hashing::keccak_256(&asset_location_on_assethub.encode()).into();

		let asset_id_on_paraa: H256 =
				sp_io::hashing::keccak_256(&Location::new(1, Here).encode())
					.into();


			ParaA::execute_with(|| {
				/*assert_ok!(runtime::Assets::force_create(
                    runtime::RuntimeOrigin::root(),
                    asset_id_on_paraa.into(),
                    ALICE.into(),
                    true,
                    1
                ));*/
			});


			ParaB::execute_with(|| {
				let dest = Location::new(1, [Parachain(PARA_ID)]);
				let beneficiary: Location = Junctions::X3(Arc::new([
					Junction::AccountId32 { network: None, id: ALICE.into() },
					Junction::AccountKey20 {
						network: Some(NetworkId::Ethereum { chain_id: 97 }),
						key: [1u8; 20],
					},
					Junction::GeneralIndex(60 * 60),
				]))
					.into_location();

				let context = Junctions::X2(Arc::new([
					Junction::GlobalConsensus(NetworkId::Polkadot),
					Parachain(1000),
				]));

				let assets = Asset {
					id: AssetId(asset_location_on_assethub.clone()),
					fun: Fungibility::Fungible(SEND_AMOUNT),
				};

				let fee_asset = assets.clone().reanchored(&dest, &context).expect("should reanchor");
				let fees = fee_asset.clone();

				// let xcm = Xcm(vec![
				// 	BuyExecution { fees, weight_limit:  WeightLimit::Unlimited },
				// 	DepositAsset {
				// 		assets: Wild(All),
				// 		beneficiary,
				// 	},
				// ]);

				// let message = Xcm(vec![
				// 	SetFeesMode { jit_withdraw: true },
				// 	TransferReserveAsset {
				// 		assets: assets.into(),
				// 		dest,
				// 		xcm,
				// 	},
				// ]);

				// assert_ok!(runtime::PalletXcm::execute(
                //     runtime::RuntimeOrigin::signed(ALICE.into()),
                //     Box::new(VersionedXcm::from(message)),
                //    Weight::MAX
                // ));

				assert_ok!(runtime::PalletXcm::limited_reserve_transfer_assets(
                    runtime::RuntimeOrigin::signed(ALICE.into()),
                    Box::new(dest.into()),
                    Box::new(beneficiary.into()),
                    Box::new(vec![(asset_location_on_assethub, SEND_AMOUNT).into()].into()),
                    0,
                    WeightLimit::Unlimited,
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

				let parachain_account: ParaId =   PARA_ID.into();
				let parachain_account = parachain_account.into_account_truncating();

				let alice_balance = <runtime::Assets as Inspect<
					<Test as frame_system::Config>::AccountId,
				>>::balance(
					asset_id_on_paraa,
					&parachain_account,
				);
				dbg!(alice_balance);
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
fn should_process_on_accept_module_callback_correctly() {
	init_tracing();
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

	let asset_location = Location::new(1, Here);

	let dest = Location::new(1, [Parachain(PARA_ID)]);
	let asset_id: H256 = sp_io::hashing::keccak_256(&asset_location.encode()).into();


	let alice_balance = ParaB::execute_with(|| {
		let result = PalletXcm::limited_reserve_transfer_assets(
			runtime::RuntimeOrigin::signed(ALICE),
			Box::new(dest.clone().into()),
			Box::new(beneficiary.clone().into()),
			Box::new((asset_location, SEND_AMOUNT).into()),
			0,
			weight_limit,
		);
		assert_ok!(result);
		let alice_balance = <runtime::Assets as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::balance(asset_id, &ALICE);
		dbg!(alice_balance);
		alice_balance
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
		dbg!(initial_total_issuance);
		ismp_module.on_accept(post).unwrap();

		let total_issuance_after = <pallet_assets::Pallet<Test> as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::total_issuance(asset_id);
		// Total issuance should have dropped
		assert_eq!(initial_total_issuance - amount, total_issuance_after);
		amount
	});

	ParaB::execute_with(|| {
		// Alice's balance on asset hub should have increased by the amount transferred
		let current_balance = Assets::balance(asset_id, &ALICE);
		assert_eq!(current_balance, alice_balance + transferred);
	})
}

#[test]
fn should_process_on_timeout_module_callback_correctly() {
	init_tracing();
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

	let asset_location = Location::new(1, Here);

	let dest = Location::new(1, [Parachain(PARA_ID)]);
	let asset_id: H256 = sp_io::hashing::keccak_256(&asset_location.encode()).into();

	let alice_balance = ParaB::execute_with(|| {
		let result = PalletXcm::limited_reserve_transfer_assets(
			runtime::RuntimeOrigin::signed(ALICE),
			Box::new(dest.clone().into()),
			Box::new(beneficiary.clone().into()),
			Box::new((asset_location, SEND_AMOUNT).into()),
			0,
			weight_limit,
		);
		assert_ok!(result);
		let alice_balance = <runtime::Assets as Inspect<
			<Test as frame_system::Config>::AccountId,
		>>::balance(asset_id, &ALICE);
		dbg!(alice_balance);
		alice_balance
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

	ParaB::execute_with(|| {
		// Alice's balance on relay chain should have increased by the amount transferred
		let current_balance = Assets::balance(asset_id, &ALICE);
		assert_eq!(current_balance, alice_balance + transferred);
	})
}
