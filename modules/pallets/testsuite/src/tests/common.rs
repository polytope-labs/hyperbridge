use crate::runtime::{Assets, ReputationAssetId, RuntimeOrigin, ALICE};
use polkadot_sdk::frame_support::{
	assert_ok,
	traits::fungibles::{Inspect, Mutate},
};
use sp_core::crypto::AccountId32;

pub fn setup_relayer_and_asset(relayer_account: &AccountId32) {
	let asset_id = ReputationAssetId::get();
	if !Assets::asset_exists(asset_id) {
		assert_ok!(Assets::force_create(RuntimeOrigin::root(), asset_id, ALICE, true, 1,));
	}

	Assets::set_balance(asset_id, relayer_account, 0);
}
