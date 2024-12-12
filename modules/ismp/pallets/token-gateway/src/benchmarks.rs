#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use ismp::host::StateMachine;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn teleport() {
		let dummy_teleport_params = TeleportParams::<u128, u128> {
			asset_id: 1,
			destination: StateMachine::Evm(100),
			recepient: H256::from([1u8; 32]),
			amount: 100,
			timeout: 10,
			token_gateway: vec![1, 2, 3, 4, 5],
			relayer_fee: 10,
		};

		#[extrinsic_call]
		_(RawOrigin::Signed([0u8, 32]), dummy_teleport_params);
	}

	#[benchmark]
	fn set_token_gateway_addresses() -> Result<(), BenchmarkError> {
		let mut addresses = BTreeMap::new();
		for i in 0..50 {
			let addr = i.to_string().as_bytes().to_vec();
			addresses.insert(StateMachine::Evm(100), addr);
		}

		#[extrinsic_call]
		_(RawOrigin::Root, addresses);
		Ok(())
	}

	#[benchmark]
	fn create_erc6160_asset() -> Result<(), BenchmarkError> {
		let asset_details = GatewayAssetRegistration {
			name: b"Spectre".into(),

			symbol: b"SPC".into(),

			chains: bounded_vec![StateMachine::Evm(100)],
			minimum_balance: Some(10),
		};
		let asset = AssetRegistration::<u128> { local_id: 2, reg: asset_details };

		#[extrinsic_call]
		_(RawOrigin::Root, asset);

		Ok(())
	}

	#[benchmark]
	fn update_erc6160_asset() -> Result<(), BenchmarkError> {
		let asset_update = GatewayAssetUpdate {
			asset_id: H256,
			add_chains: bounded_vec![StateMachine::Evm(200)],
			remove_chains: bounded_vec![],
			new_admins: bounded_vec![],
		};

		#[extrinsic_call]
		_(RawOrigin::Root, asset_update);
		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::tests::Test);
}
