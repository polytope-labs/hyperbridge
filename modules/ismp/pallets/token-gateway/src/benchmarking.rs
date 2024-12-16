#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use frame_benchmarking::v2::*;
use frame_support::{traits::EnsureOrigin, BoundedVec};
use frame_system::RawOrigin;
use ismp::host::StateMachine;
use scale_info::prelude::collections::BTreeMap;
use sp_runtime::{traits::StaticLookup, AccountId32};
use token_gateway_primitives::{GatewayAssetRegistration, GatewayAssetUpdate};

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

fn dummy_teleport_asset<T>(
) -> TeleportParams<AssetId<T>, <<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance>
where
	T: Config,
	<<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::AssetId: From<H256>,
	<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance: From<u128>,
{
	TeleportParams {
		asset_id: <<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::AssetId::from(
			H256::zero(),
		),
		destination: StateMachine::Evm(100),
		recepient: H256::from([1u8; 32]),
		amount: 2u128.into(),
		timeout: 10,
		token_gateway: vec![1, 2, 3, 4, 5],
		relayer_fee: 1u128.into(),
	}
}

fn create_dummy_asset<T: Config>(
	asset_details: GatewayAssetRegistration,
) -> AssetRegistration<AssetId<T>>
where
	<<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::AssetId: From<H256>,
{
	AssetRegistration {
		local_id: <<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::AssetId::from(
			H256::zero(),
		),
		reg: asset_details,
	}
}

#[benchmarks(
	where
	T: pallet_balances::Config<Balance = u128>,
	T: pallet_assets::Config<AssetIdParameter = sp_core::H256>,
	<<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::AssetId: From<H256>,
	<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance: From<u128>,
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	u128: From<<<T as Config>::NativeCurrency as Currency<T::AccountId>>::Balance>,
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
		let asset_details = GatewayAssetRegistration {
			name: BoundedVec::try_from(b"Spectre".to_vec()).unwrap(),
			symbol: BoundedVec::try_from(b"SPC".to_vec()).unwrap(),
			chains: vec![StateMachine::Evm(100)],
			minimum_balance: Some(10),
		};
		let asset = create_dummy_asset::<T>(asset_details);

		#[extrinsic_call]
		_(RawOrigin::Signed(AccountId32::from([0u8; 32])), asset);

		Ok(())
	}

	#[benchmark]
	fn teleport() -> Result<(), BenchmarkError> {
		let asset_details = GatewayAssetRegistration {
			name: BoundedVec::try_from(b"Spectre".to_vec()).unwrap(),
			symbol: BoundedVec::try_from(b"SPC".to_vec()).unwrap(),
			chains: vec![StateMachine::Evm(100)],
			minimum_balance: None,
		};
		let asset = create_dummy_asset::<T>(asset_details);

		Pallet::<T>::create_erc6160_asset(
			RawOrigin::Signed(AccountId32::from([0u8; 32])).into(),
			asset,
		)?;

		let dummy_teleport_params = dummy_teleport_asset::<T>();

		#[extrinsic_call]
		teleport(RawOrigin::Signed(AccountId32::from([0u8; 32])), dummy_teleport_params);
		Ok(())
	}

	#[benchmark]
	fn set_token_gateway_addresses() -> Result<(), BenchmarkError> {
		let mut addresses = BTreeMap::new();
		for i in 0..50 {
			let addr = i.to_string().as_bytes().to_vec();
			addresses.insert(StateMachine::Evm(100), addr);
		}

		#[extrinsic_call]
		_(RawOrigin::Signed(AccountId32::from([0u8; 32])), addresses);
		Ok(())
	}

	#[benchmark]
	fn update_erc6160_asset() -> Result<(), BenchmarkError> {
		let origin = RawOrigin::Signed(AccountId32::from([0u8; 32]));

		let asset_details = GatewayAssetRegistration {
			name: BoundedVec::try_from(b"Spectre".to_vec()).unwrap(),
			symbol: BoundedVec::try_from(b"SPC".to_vec()).unwrap(),
			chains: vec![StateMachine::Evm(100)],
			minimum_balance: None,
		};
		let asset = create_dummy_asset::<T>(asset_details);

		let asset_id: H256 = sp_io::hashing::keccak_256(asset.reg.symbol.as_ref()).into();

		let owner =
			<T::Assets as fungibles::roles::Inspect<T::AccountId>>::admin(H256::zero().into());
		log::info!("owner: {owner:?}");

		// set balances
		let account = T::AccountId::from([0u8; 32]);
		let acc = <<T as frame_system::Config>::Lookup as StaticLookup>::unlookup(account.clone());
		pallet_balances::Pallet::<T>::force_set_balance(RawOrigin::Root.into(), acc, 1000u128)?;

		let bal = <T::NativeCurrency as Currency<T::AccountId>>::total_balance(&account.clone());
		log::info!("bal: {bal:?}");
		// set asset balance

		Pallet::<T>::create_erc6160_asset(
			RawOrigin::Signed(AccountId32::from([0u8; 32])).into(),
			asset.clone(),
		)?;

		let asset_update = GatewayAssetUpdate {
			asset_id,
			add_chains: BoundedVec::try_from(vec![StateMachine::Evm(200)]).unwrap(),
			remove_chains: BoundedVec::try_from(Vec::new()).unwrap(),
			new_admins: BoundedVec::try_from(Vec::new()).unwrap(),
		};

		#[extrinsic_call]
		_(origin, asset_update);
		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ex(), crate::mock::Test);
}
