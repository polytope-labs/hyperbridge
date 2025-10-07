#![cfg(feature = "runtime-benchmarks")]

use crate::{types::*, *};
use frame_benchmarking::v2::*;
use frame_support::{
	traits::{fungible, fungibles, Currency, EnsureOrigin},
	BoundedVec,
};
use frame_system::RawOrigin;
use ismp::host::StateMachine;
use polkadot_sdk::*;
use scale_info::prelude::collections::BTreeMap;
use sp_runtime::AccountId32;
use token_gateway_primitives::{GatewayAssetRegistration, GatewayAssetUpdate};

#[benchmarks(
	where
	<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance: From<u128>,
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	u128: From<<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance>,
	T::Balance: From<u128>,
	<T as pallet_ismp::Config>::Balance: From<<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance>,
	<<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::Balance: From<<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance>,
	<<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::Balance: From<u128>,
	[u8; 32]: From<<T as frame_system::Config>::AccountId>,
	<T as frame_system::Config>::RuntimeOrigin: From<frame_system::RawOrigin<AccountId32>>,
	<T as Config>::NativeCurrency: fungible::Mutate<T::AccountId>,
	<T as Config>::NativeCurrency: fungible::Inspect<T::AccountId>,
	<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance: Into<<<T as Config>::NativeCurrency as fungible::Inspect<T::AccountId>>::Balance>,
	T::CreateOrigin: EnsureOrigin<T::RuntimeOrigin>
)]
mod benches {
	use super::*;

	#[benchmark]
	fn create_erc6160_asset(x: Linear<1, 100>) -> Result<(), BenchmarkError> {
		let origin =
			T::CreateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let asset_details = GatewayAssetRegistration {
			name: BoundedVec::try_from(b"Spectre".to_vec()).unwrap(),
			symbol: BoundedVec::try_from(b"SPC".to_vec()).unwrap(),
			chains: vec![StateMachine::Evm(100)],
			minimum_balance: Some(10),
		};

		let mut precision = BTreeMap::new();
		for i in 0..x {
			precision.insert(StateMachine::Evm(i as u32), 18);
		}

		let asset = AssetRegistration {
			local_id: T::NativeAssetId::get(),
			reg: asset_details,
			native: true,
			precision,
		};

		#[extrinsic_call]
		_(origin, asset);

		Ok(())
	}

	#[benchmark]
	fn teleport() -> Result<(), BenchmarkError> {
		let account: T::AccountId = whitelisted_caller();
		let create_origin =
			T::CreateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let asset_id = T::NativeAssetId::get();

		let ed = T::NativeCurrency::minimum_balance();
		let teleport_amount: <<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance =
			10_000_000_000_000u128.into();
		let initial_balance = ed + teleport_amount + 1000u128.into();

		<T::NativeCurrency as fungible::Mutate<T::AccountId>>::set_balance(
			&account,
			initial_balance.into(),
		);

		Pallet::<T>::create_erc6160_asset(
			create_origin,
			AssetRegistration {
				local_id: asset_id.clone(),
				reg: GatewayAssetRegistration {
					name: BoundedVec::try_from(b"Spectre".to_vec()).unwrap(),
					symbol: BoundedVec::try_from(b"SPC".to_vec()).unwrap(),
					chains: vec![StateMachine::Evm(100)],
					minimum_balance: None,
				},
				native: true,
				precision: vec![(StateMachine::Evm(100), 18)].into_iter().collect(),
			},
		)?;

		let teleport_params = TeleportParams {
			asset_id,
			destination: StateMachine::Evm(100),
			recepient: H256::from([1u8; 32]),
			amount: 10_000_000_000_000u128.into(),
			timeout: 0,
			token_gateway: vec![1, 2, 3, 4, 5],
			relayer_fee: 0u128.into(),
			call_data: None,
			redeem: false,
		};

		#[extrinsic_call]
		_(RawOrigin::Signed(account), teleport_params);
		Ok(())
	}

	#[benchmark]
	fn set_token_gateway_addresses(x: Linear<1, 100>) -> Result<(), BenchmarkError> {
		let origin =
			T::CreateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let mut addresses = BTreeMap::new();
		for i in 0..x {
			let addr = i.to_string().as_bytes().to_vec();
			addresses.insert(StateMachine::Evm(100), addr);
		}

		#[extrinsic_call]
		_(origin, addresses);
		Ok(())
	}

	#[benchmark]
	fn update_erc6160_asset() -> Result<(), BenchmarkError> {
		let origin =
			T::CreateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let local_id = T::NativeAssetId::get();

		Pallet::<T>::create_erc6160_asset(
			origin.clone(),
			AssetRegistration {
				local_id,
				reg: GatewayAssetRegistration {
					name: BoundedVec::try_from(b"Spectre".to_vec()).unwrap(),
					symbol: BoundedVec::try_from(b"SPC".to_vec()).unwrap(),
					chains: vec![StateMachine::Evm(100)],
					minimum_balance: None,
				},
				native: true,
				precision: Default::default(),
			},
		)?;

		let asset_update = GatewayAssetUpdate {
			asset_id: sp_io::hashing::keccak_256(b"SPC".as_ref()).into(),
			add_chains: BoundedVec::try_from(vec![StateMachine::Evm(200)]).unwrap(),
			remove_chains: BoundedVec::try_from(Vec::new()).unwrap(),
			new_admins: BoundedVec::try_from(Vec::new()).unwrap(),
		};

		#[extrinsic_call]
		_(origin, asset_update);
		Ok(())
	}

	#[benchmark]
	fn update_asset_precision(x: Linear<1, 100>) -> Result<(), BenchmarkError> {
		let origin =
			T::CreateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let mut precisions = BTreeMap::new();
		for i in 0..x {
			precisions.insert(StateMachine::Evm(i as u32), 18);
		}

		let update = PrecisionUpdate { asset_id: T::NativeAssetId::get(), precisions };

		#[extrinsic_call]
		_(origin, update);
		Ok(())
	}

	#[benchmark]
	fn register_asset_locally(x: Linear<1, 100>) -> Result<(), BenchmarkError> {
		let origin =
			T::CreateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let asset_details = GatewayAssetRegistration {
			name: BoundedVec::try_from(b"Local".to_vec()).unwrap(),
			symbol: BoundedVec::try_from(b"Local".to_vec()).unwrap(),
			chains: vec![StateMachine::Evm(100)],
			minimum_balance: Some(10),
		};

		let mut precision = BTreeMap::new();
		for i in 0..x {
			precision.insert(StateMachine::Evm(i as u32), 18);
		}

		let asset = AssetRegistration {
			local_id: T::NativeAssetId::get(),
			reg: asset_details,
			native: true,
			precision,
		};

		#[extrinsic_call]
		_(origin, asset);

		Ok(())
	}
}
