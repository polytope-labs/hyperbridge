use crate::runtime::{
	new_test_ext, Assets, CollatorSelection, ReputationAssetId, RuntimeOrigin, Session, Test,
	ALICE, BOB, CHARLIE, DAVE, UNIT,
};
use frame_system::Pallet as System;
use pallet_session;
use polkadot_sdk::{
	frame_support::{
		assert_ok,
		traits::{fungibles::Mutate, OnInitialize},
	},
	sp_core::{sr25519::Pair, Pair as _},
	*,
};

fn create_reputation_asset() {
	assert_ok!(Assets::force_create(
		RuntimeOrigin::root(),
		ReputationAssetId::get(),
		ALICE,
		true,
		1,
	));
}

fn set_reputation_balance(who: &<Test as frame_system::Config>::AccountId, amount: u128) {
	Assets::set_balance(ReputationAssetId::get(), who, amount);
}

fn register_candidate(who: <Test as frame_system::Config>::AccountId) {
	let bond = 10 * UNIT;
	set_reputation_balance(&who, bond);
	assert_ok!(CollatorSelection::register_as_candidate(RuntimeOrigin::signed(who.clone())));
}

fn set_session_keys(who: <Test as frame_system::Config>::AccountId) {
	System::<Test>::inc_providers(&who);
	let pair = Pair::from_seed(who.as_ref());
	let keys = crate::runtime::SessionKeys { aura: pair.public().into() };
	assert_ok!(Session::set_keys(RuntimeOrigin::signed(who), keys, vec![]));
}

#[test]
fn test_new_collators_are_selected_based_on_reputation() {
	new_test_ext().execute_with(|| {
		create_reputation_asset();

		set_session_keys(ALICE);
		set_session_keys(BOB);
		pallet_session::Validators::<Test>::put(vec![ALICE, BOB]);
		pallet_session::QueuedKeys::<Test>::put(vec![
			(
				ALICE,
				crate::runtime::SessionKeys {
					aura: Pair::from_seed(ALICE.as_ref()).public().into(),
				},
			),
			(
				BOB,
				crate::runtime::SessionKeys { aura: Pair::from_seed(BOB.as_ref()).public().into() },
			),
		]);

		set_session_keys(CHARLIE);
		set_session_keys(DAVE);
		register_candidate(CHARLIE);
		register_candidate(DAVE);

		set_reputation_balance(&DAVE, 20 * UNIT);

		Session::on_initialize(2);
		Session::on_initialize(3);

		let mut new_collators = Session::validators();
		new_collators.sort();
		assert_eq!(new_collators, vec![CHARLIE, DAVE]);

		// Dave is now an incoming collator, and balance is now zero
		assert_eq!(Assets::balance(ReputationAssetId::get(), &DAVE), 0);
	});
}

#[test]
fn test_reuse_previous_collators_if_not_enough_candidates() {
	new_test_ext().execute_with(|| {
		create_reputation_asset();

		set_session_keys(ALICE);
		set_session_keys(BOB);
		pallet_session::Validators::<Test>::put(vec![ALICE, BOB]);
		pallet_session::QueuedKeys::<Test>::put(vec![
			(
				ALICE,
				crate::runtime::SessionKeys {
					aura: Pair::from_seed(ALICE.as_ref()).public().into(),
				},
			),
			(
				BOB,
				crate::runtime::SessionKeys { aura: Pair::from_seed(BOB.as_ref()).public().into() },
			),
		]);
		set_reputation_balance(&ALICE, 50 * UNIT);
		set_reputation_balance(&BOB, 30 * UNIT);

		set_session_keys(CHARLIE);
		register_candidate(CHARLIE);

		Session::on_initialize(2);
		Session::on_initialize(3);

		let mut new_collators = Session::validators();
		new_collators.sort();
		assert_eq!(new_collators, vec![ALICE, CHARLIE]); // Alice is chosen because the account has more
		                                           // balances than Bob
	});
}

#[test]
fn test_collator_set_does_not_change_if_no_new_candidates() {
	new_test_ext().execute_with(|| {
		create_reputation_asset();

		set_session_keys(ALICE);
		set_session_keys(BOB);
		pallet_session::Validators::<Test>::put(vec![ALICE, BOB]);
		pallet_session::QueuedKeys::<Test>::put(vec![
			(
				ALICE,
				crate::runtime::SessionKeys {
					aura: Pair::from_seed(ALICE.as_ref()).public().into(),
				},
			),
			(
				BOB,
				crate::runtime::SessionKeys { aura: Pair::from_seed(BOB.as_ref()).public().into() },
			),
		]);
		set_reputation_balance(&ALICE, 50 * UNIT);
		set_reputation_balance(&BOB, 30 * UNIT);

		Session::on_initialize(2);
		Session::on_initialize(3);

		let mut new_collators = Session::validators();
		new_collators.sort();
		assert_eq!(new_collators, vec![ALICE, BOB]); // still use existing collator set since there are no
		                                       // candidates set up
	});
}
