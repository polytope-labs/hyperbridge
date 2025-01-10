#![cfg(feature = "runtime-benchmarks")]

use crate::{types::*, *};
use frame_benchmarking::v2::*;
use frame_support::{
	traits::{fungible, fungibles},
	BoundedVec,
};
use frame_system::RawOrigin;
use ismp::host::StateMachine;
use scale_info::prelude::collections::BTreeMap;
use sp_runtime::AccountId32;
use token_gateway_primitives::{GatewayAssetRegistration, GatewayAssetUpdate};

fn dummy_teleport_asset<T>(
	asset_id: AssetId<T>,
) -> TeleportParams<AssetId<T>, <<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance>
where
	T: Config,
	<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance: From<u128>,
{
	TeleportParams {
		asset_id,
		destination: StateMachine::Evm(100),
		recepient: H256::from([1u8; 32]),
		amount: 1100000000u128.into(),
		timeout: 10,
		token_gateway: vec![1, 2, 3, 4, 5],
		relayer_fee: 1000000002u128.into(),
		call_data: None,
	}
}

fn create_dummy_asset<T: Config>(
	asset_details: GatewayAssetRegistration,
) -> AssetRegistration<AssetId<T>>
where
{
	let local_id = T::AssetIdFactory::create_asset_id(asset_details.symbol.to_vec()).unwrap();
	AssetRegistration { local_id, reg: asset_details, native: true }
}

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
)]
mod benches {
	use super::*;

	#[benchmark]
	fn create_erc6160_asset() -> Result<(), BenchmarkError> {
		let account: T::AccountId = whitelisted_caller();

		let asset_details = GatewayAssetRegistration {
			name: BoundedVec::try_from(b"Spectre".to_vec()).unwrap(),
			symbol: BoundedVec::try_from(b"SPC".to_vec()).unwrap(),
			chains: vec![StateMachine::Evm(100)],
			minimum_balance: Some(10),
		};
		let asset = create_dummy_asset::<T>(asset_details);

		<T::Currency as fungible::Mutate<T::AccountId>>::set_balance(&account, u128::MAX.into());

		#[extrinsic_call]
		_(RawOrigin::Signed(account), asset);

		Ok(())
	}

	#[benchmark]
	fn teleport() -> Result<(), BenchmarkError> {
		let account: T::AccountId = whitelisted_caller();

		let asset_details = GatewayAssetRegistration {
			name: BoundedVec::try_from(b"Spectre".to_vec()).unwrap(),
			symbol: BoundedVec::try_from(b"SPC".to_vec()).unwrap(),
			chains: vec![StateMachine::Evm(100)],
			minimum_balance: None,
		};
		let asset = create_dummy_asset::<T>(asset_details);

		Pallet::<T>::create_erc6160_asset(
			RawOrigin::Signed(account.clone()).into(),
			asset.clone(),
		)?;

		let dummy_teleport_params = dummy_teleport_asset::<T>(asset.local_id);

		<T::Currency as fungible::Mutate<T::AccountId>>::set_balance(&account, u128::MAX.into());

		#[extrinsic_call]
		teleport(RawOrigin::Signed(account), dummy_teleport_params);
		Ok(())
	}

	#[benchmark]
	fn set_token_gateway_addresses(x: Linear<5, 100>) -> Result<(), BenchmarkError> {
		let mut addresses = BTreeMap::new();
		for i in 0..x {
			let addr = i.to_string().as_bytes().to_vec();
			addresses.insert(StateMachine::Evm(100), addr);
		}

		#[extrinsic_call]
		_(RawOrigin::Root, addresses);
		Ok(())
	}

	#[benchmark]
	fn update_erc6160_asset() -> Result<(), BenchmarkError> {
		let acc_origin: T::AccountId = whitelisted_caller();

		let asset_details = GatewayAssetRegistration {
			name: BoundedVec::try_from(b"Spectre".to_vec()).unwrap(),
			symbol: BoundedVec::try_from(b"SPC".to_vec()).unwrap(),
			chains: vec![StateMachine::Evm(100)],
			minimum_balance: None,
		};
		let asset = create_dummy_asset::<T>(asset_details.clone());

		// set balances
		<T::Currency as fungible::Mutate<T::AccountId>>::set_balance(&acc_origin, u128::MAX.into());
		let asset_id = T::AssetIdFactory::create_asset_id(asset_details.symbol.to_vec()).unwrap();
		<T::Assets as fungibles::Create<T::AccountId>>::create(
			asset_id.into(),
			acc_origin.clone(),
			true,
			1000000000u128.into(),
		)?;

		Pallet::<T>::create_erc6160_asset(
			RawOrigin::Signed(acc_origin.clone()).into(),
			asset.clone(),
		)?;

		let asset_update = GatewayAssetUpdate {
			asset_id: H256::zero(),
			add_chains: BoundedVec::try_from(vec![StateMachine::Evm(200)]).unwrap(),
			remove_chains: BoundedVec::try_from(Vec::new()).unwrap(),
			new_admins: BoundedVec::try_from(Vec::new()).unwrap(),
		};

		#[extrinsic_call]
		_(RawOrigin::Signed(acc_origin), asset_update);
		Ok(())
	}
}
