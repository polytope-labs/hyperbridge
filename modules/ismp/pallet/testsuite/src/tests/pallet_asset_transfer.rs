#![cfg(test)]

use crate::{
    relay_chain::{self, RuntimeOrigin},
    runtime::Test,
    xcm::{MockNet, ParaA, Relay},
};
use frame_support::{assert_ok, traits::fungibles::Inspect};
use staging_xcm::v3::{Junction, Junctions, MultiLocation, NetworkId, WeightLimit};
use xcm_simulator::TestExt;
use xcm_simulator_example::ALICE;

pub type RelayChainPalletXcm = pallet_xcm::Pallet<relay_chain::Runtime>;
#[test]
fn should_dispatch_ismp_request_when_assets_are_received_from_relay_chain() {
    MockNet::reset();

    const SEND_AMOUNT: u128 = 1000;
    const PARA_ID: u32 = 1;

    let beneficiary: MultiLocation = Junctions::X2(
        Junction::AccountId32 { network: None, id: ALICE.into() },
        Junction::AccountKey20 {
            network: Some(NetworkId::Ethereum { chain_id: 1 }),
            key: [1u8; 20],
        },
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

        let protocol_fees = <Test as pallet_asset_transfer::Config>::ProtocolFees::get();
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
            &pallet_asset_transfer::Pallet::<Test>::account_id(),
        );
        dbg!(pallet_account_balance);
        assert_eq!(custodied_amount, pallet_account_balance);
    });
}
