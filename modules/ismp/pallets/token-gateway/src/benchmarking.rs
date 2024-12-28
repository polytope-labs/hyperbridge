#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use frame_benchmarking::v2::*;
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use ismp::host::StateMachine;
use pallet_balances::AdjustmentDirection;
use scale_info::prelude::collections::BTreeMap;
use sp_runtime::{traits::StaticLookup, AccountId32};
use token_gateway_primitives::{GatewayAssetRegistration, GatewayAssetUpdate};

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
        amount: 1100000000u128.into(),
        timeout: 10,
        token_gateway: vec![1, 2, 3, 4, 5],
        relayer_fee: 1000000002u128.into(),
    }
}

fn create_dummy_asset<T: Config>(
    asset_details: GatewayAssetRegistration,
) -> AssetRegistration<AssetId<T>>
    where
        <<T as Config>::Assets as fungibles::Inspect<T::AccountId>>::AssetId: From<H256>,
{
    AssetRegistration { local_id: H256::zero().into(), reg: asset_details }
}

#[benchmarks(
    where
    T: pallet_balances::Config<Balance = u128>,
    T: pallet_assets::Config<AssetIdParameter = sp_core::H256,Balance = u128>,
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
        let account: T::AccountId = whitelisted_caller();

        let asset_details = GatewayAssetRegistration {
            name: BoundedVec::try_from(b"Spectre".to_vec()).unwrap(),
            symbol: BoundedVec::try_from(b"SPC".to_vec()).unwrap(),
            chains: vec![StateMachine::Evm(100)],
            minimum_balance: Some(10),
        };
        let asset = create_dummy_asset::<T>(asset_details);

        // Set balances
        let ed = <T as pallet_balances::Config>::ExistentialDeposit::get();

        // Adjust total issuance
        pallet_balances::Pallet::<T>::force_adjust_total_issuance(
            RawOrigin::Root.into(),
            AdjustmentDirection::Increase,
            ed * 1000,
        )?;

        let acc = <<T as frame_system::Config>::Lookup as StaticLookup>::unlookup(account.clone());

        pallet_balances::Pallet::<T>::force_set_balance(RawOrigin::Root.into(), acc, ed * 100u128)?;

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

        Pallet::<T>::create_erc6160_asset(RawOrigin::Signed(account.clone()).into(), asset)?;

        let dummy_teleport_params = dummy_teleport_asset::<T>();

        // Set balances
        let ed = <T as pallet_balances::Config>::ExistentialDeposit::get();

        // Adjust total issuance
        pallet_balances::Pallet::<T>::force_adjust_total_issuance(
            RawOrigin::Root.into(),
            AdjustmentDirection::Increase,
            ed * 1000,
        )?;

        let acc = <<T as frame_system::Config>::Lookup as StaticLookup>::unlookup(account.clone());

        pallet_balances::Pallet::<T>::force_set_balance(
            RawOrigin::Root.into(),
            acc.clone(),
            ed * 100u128,
        )?;

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
        let asset = create_dummy_asset::<T>(asset_details);
        let asset_id: H256 = sp_io::hashing::keccak_256(asset.reg.symbol.as_ref()).into();

        // set balances

        let acc_o =
            <<T as frame_system::Config>::Lookup as StaticLookup>::unlookup(acc_origin.clone());
        let ed = <T as pallet_balances::Config>::ExistentialDeposit::get();
        pallet_balances::Pallet::<T>::force_set_balance(
            RawOrigin::Root.into(),
            acc_o.clone(),
            ed * 100u128,
        )?;

        // set asset balance
        pallet_assets::Pallet::<T>::create(
            RawOrigin::Signed(acc_origin.clone()).into(),
            H256::zero().into(),
            acc_o.clone(),
            1000000000,
        )?;

        Pallet::<T>::create_erc6160_asset(
            RawOrigin::Signed(acc_origin.clone()).into(),
            asset.clone(),
        )?;

        let asset_update = GatewayAssetUpdate {
            asset_id,
            add_chains: BoundedVec::try_from(vec![StateMachine::Evm(200)]).unwrap(),
            remove_chains: BoundedVec::try_from(Vec::new()).unwrap(),
            new_admins: BoundedVec::try_from(Vec::new()).unwrap(),
        };

        #[extrinsic_call]
        _(RawOrigin::Signed(acc_origin), asset_update);
        Ok(())
    }
}
