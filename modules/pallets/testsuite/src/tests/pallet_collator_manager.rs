use crate::runtime::{new_test_ext, Assets, CollatorSelection, CollatorManager, ReputationAssetId, RuntimeOrigin, Session, Test, ALICE, BOB, CHARLIE, DAVE, UNIT, Balance, Balances, INITIAL_BALANCE, Vesting,
					 CollatorBondLockId, EXISTENTIAL_DEPOSIT, Sudo, RuntimeCall};
use polkadot_sdk::frame_support::traits::fungible::Mutate as BalanceMutate;
use frame_system::Pallet as System;
use pallet_session;
use polkadot_sdk::{
	frame_support::{
		assert_ok, assert_err,
		traits::{fungibles::Mutate, OnInitialize, Currency, LockableCurrency, ReservableCurrency},
	},
	sp_core::{sr25519::Pair, Pair as _},
	*,
};
use pallet_collator_manager::Error;
use pallet_vesting::VestingInfo;
use polkadot_sdk::pallet_balances::BalanceLock;

fn create_reputation_asset() {
	assert_ok!(Assets::force_create(
		RuntimeOrigin::root(),
		ReputationAssetId::get(),
		ALICE,
		true,
		1,
	));
}

fn set_vesting_schedule(who: &<Test as frame_system::Config>::AccountId, amount:Balance) {
	Balances::set_balance(&BOB, INITIAL_BALANCE);
	let vesting_info = VestingInfo::new(amount, amount / 10, 100);
	assert_ok!(Vesting::vested_transfer(RuntimeOrigin::signed(BOB), who.clone(), vesting_info));
}

fn get_collator_bond_lock(who: &<Test as frame_system::Config>::AccountId) -> Option<BalanceLock<Balance>> {
	Balances::locks(who).into_iter().find(|lock| lock.id == CollatorBondLockId::get())
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

#[test]
fn reserve_from_free_balance_should_work() {
	new_test_ext().execute_with( || {
		let bond_amount = 100;
		assert_ok!(CollatorManager::reserve(&ALICE, bond_amount));

		assert_eq!(CollatorManager::reserved_balance(&ALICE), bond_amount);

		let lock = get_collator_bond_lock(&ALICE).expect("lock should exist");
		assert_eq!(lock.amount, bond_amount);
	});
}

#[test]
fn reserve_from_locked_vesting_balance_should_work() {
	new_test_ext().execute_with(|| {
		Balances::set_balance(&CHARLIE, INITIAL_BALANCE);

		let vesting_amount = 5000;
		set_vesting_schedule(&CHARLIE, vesting_amount);

		assert_eq!(Balances::locks(&CHARLIE).len(), 1);
		assert_eq!(Balances::locks(&CHARLIE)[0].amount, vesting_amount);

		let bond_amount = INITIAL_BALANCE + 5000;
		assert_ok!(CollatorManager::reserve(&CHARLIE, bond_amount));

		assert_eq!(CollatorManager::reserved_balance(&CHARLIE), bond_amount);
		let lock = get_collator_bond_lock(&CHARLIE).expect("collator bond lock should exist");
		assert_eq!(lock.amount, bond_amount);
	});
}

#[test]
fn reserve_fails_if_not_enough_total_balance() {
	new_test_ext().execute_with(|| {
		let bond_amount = INITIAL_BALANCE;
		assert_err!(CollatorManager::reserve(&CHARLIE, bond_amount), Error::<Test>::InsufficientBalance);
	})
}

#[test]
fn test_collator_candidate_bonding_works_with_vesting_tokens() {
	new_test_ext().execute_with( || {
		let bond_amount = 10_000_000_000_000;
		assert_ok!(Sudo::sudo(
			RuntimeOrigin::root(),
			Box::new(crate::runtime::RuntimeCall::CollatorSelection(
				pallet_collator_selection::Call::set_candidacy_bond { bond: bond_amount }
			))
		));
		set_vesting_schedule(&CHARLIE, bond_amount * 2);
		assert_eq!(pallet_collator_selection::CandidateList::<Test>::get().len(), 0);

		set_session_keys(CHARLIE);
		assert_ok!(CollatorSelection::register_as_candidate(RuntimeOrigin::signed(CHARLIE)));
		assert_eq!(pallet_collator_selection::CandidateList::<Test>::get().len(), 1);
		assert_eq!(CollatorManager::reserved_balance(&CHARLIE), bond_amount);
		let lock = get_collator_bond_lock(&CHARLIE).expect("collator bond lock should exist");
		assert_eq!(lock.amount, bond_amount);
	});
}
