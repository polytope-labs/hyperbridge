use crate::runtime::{
	new_test_ext, Assets, Balance, Balances, CollatorBondLockId, CollatorManager,
	CollatorSelection, ReputationAssetId, RuntimeOrigin, Session, Sudo, Test, Vesting, ALICE, BOB,
	CHARLIE, DAVE, INITIAL_BALANCE, UNIT,
};
use frame_system::Pallet as System;
use pallet_collator_manager::Error;
use pallet_session;
use pallet_vesting::VestingInfo;
use polkadot_sdk::{
	frame_support::{
		assert_err, assert_ok,
		traits::{
			fungible::Mutate as BalanceMutate, fungibles::Mutate, OnInitialize, ReservableCurrency,
			ValidatorRegistration,
		},
	},
	pallet_authorship::EventHandler,
	pallet_balances::BalanceLock,
	sp_core::{sr25519::Pair, Pair as _},
	sp_runtime::traits::AccountIdConversion,
	*,
};
use sp_core::crypto::AccountId32;

fn create_reputation_asset() {
	assert_ok!(Assets::force_create(
		RuntimeOrigin::root(),
		ReputationAssetId::get(),
		ALICE,
		true,
		1,
	));
}

fn set_vesting_schedule(who: &<Test as frame_system::Config>::AccountId, amount: Balance) {
	Balances::set_balance(&BOB, INITIAL_BALANCE);
	let vesting_info = VestingInfo::new(amount, amount / 10, 100);
	assert_ok!(Vesting::vested_transfer(RuntimeOrigin::signed(BOB), who.clone(), vesting_info));
}

fn get_collator_bond_lock(
	who: &<Test as frame_system::Config>::AccountId,
) -> Option<BalanceLock<Balance>> {
	Balances::locks(who)
		.into_iter()
		.find(|lock| lock.id == CollatorBondLockId::get())
}

fn set_reputation_balance(who: &<Test as frame_system::Config>::AccountId, amount: u128) {
	Assets::set_balance(ReputationAssetId::get(), who, amount);
}

fn register_candidate(who: <Test as frame_system::Config>::AccountId) {
	let bond = 10 * UNIT;
	set_reputation_balance(&who, bond);
	assert_ok!(CollatorSelection::register_as_candidate(RuntimeOrigin::signed(who.clone())));
}

/// Two-step pairing helper: the controller publishes its consent, then the
/// stash consumes it via `register`. Centralises the new approval flow so the
/// pre-existing tests don't have to repeat the pattern at every call site.
fn link_stash_to_controller(
	stash: <Test as frame_system::Config>::AccountId,
	controller: <Test as frame_system::Config>::AccountId,
) {
	assert_ok!(CollatorManager::approve_controller(
		RuntimeOrigin::signed(controller.clone()),
		stash.clone(),
	));
	assert_ok!(CollatorManager::register(RuntimeOrigin::signed(stash), controller));
}

/// Rotation helper: the new controller publishes consent, then the stash
/// completes the rotation via `set_controller`.
fn rotate_controller(
	stash: <Test as frame_system::Config>::AccountId,
	new_controller: <Test as frame_system::Config>::AccountId,
) {
	assert_ok!(CollatorManager::approve_controller(
		RuntimeOrigin::signed(new_controller.clone()),
		stash.clone(),
	));
	assert_ok!(CollatorManager::set_controller(
		RuntimeOrigin::signed(stash),
		new_controller,
	));
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
		let charlie_stash = AccountId32::new([13; 32]);
		Balances::set_balance(&charlie_stash, INITIAL_BALANCE);

		let dave_stash = AccountId32::new([14; 32]);
		Balances::set_balance(&dave_stash, INITIAL_BALANCE);

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

		link_stash_to_controller(charlie_stash.clone(), CHARLIE);
		link_stash_to_controller(dave_stash.clone(), DAVE);

		set_session_keys(CHARLIE);
		set_session_keys(DAVE);
		register_candidate(charlie_stash.clone());
		register_candidate(dave_stash.clone());

		assert_ok!(CollatorManager::reserve(&charlie_stash, 100 * UNIT));

		assert_ok!(CollatorManager::reserve(&dave_stash, 100 * UNIT));

		set_reputation_balance(&CHARLIE, 20 * UNIT);
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

		let charlie_stash = AccountId32::new([13; 32]);
		Balances::set_balance(&charlie_stash, INITIAL_BALANCE);

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
		set_reputation_balance(&CHARLIE, 40 * UNIT);

		// Previous collators are only reused while their stash stays bonded, so back
		// both Alice and Bob with a bonded stash. Alice then wins the carried-over slot
		// on reputation.
		let alice_stash = AccountId32::new([11; 32]);
		Balances::set_balance(&alice_stash, INITIAL_BALANCE);
		link_stash_to_controller(alice_stash.clone(), ALICE);
		assert_ok!(CollatorManager::reserve(&alice_stash, 100 * UNIT));

		let bob_stash = AccountId32::new([12; 32]);
		Balances::set_balance(&bob_stash, INITIAL_BALANCE);
		link_stash_to_controller(bob_stash.clone(), BOB);
		assert_ok!(CollatorManager::reserve(&bob_stash, 100 * UNIT));

		link_stash_to_controller(charlie_stash.clone(), CHARLIE);

		set_session_keys(CHARLIE);
		register_candidate(charlie_stash.clone());

		assert_ok!(CollatorManager::reserve(&charlie_stash, 100 * UNIT));

		Session::on_initialize(2);
		Session::on_initialize(3);

		let mut new_collators = Session::validators();
		new_collators.sort();
		assert_eq!(new_collators, vec![ALICE, CHARLIE]); // Alice is chosen because the account has more
		                                           // balances than Bob
	});
}

#[test]
fn test_unbonded_previous_collators_are_not_reused() {
	new_test_ext().execute_with(|| {
		create_reputation_asset();

		// Alice and Bob are last session's collators. Only Alice is still bonded; Bob
		// has no bonded stash. With no fresh candidates, the next set should reuse Alice
		// and drop Bob, even though Bob still holds reputation.
		let alice_stash = AccountId32::new([11; 32]);
		Balances::set_balance(&alice_stash, INITIAL_BALANCE);

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

		link_stash_to_controller(alice_stash.clone(), ALICE);
		assert_ok!(CollatorManager::reserve(&alice_stash, 100 * UNIT));

		Session::on_initialize(2);
		Session::on_initialize(3);

		assert_eq!(Session::validators(), vec![ALICE]);
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
	new_test_ext().execute_with(|| {
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
		assert_err!(
			CollatorManager::reserve(&CHARLIE, bond_amount),
			Error::<Test>::InsufficientBalance
		);
	})
}

#[test]
fn test_collator_candidate_bonding_works_with_vesting_tokens() {
	new_test_ext().execute_with(|| {
		let bond_amount = 10_000_000_000_000;
		assert_ok!(Sudo::sudo(
			RuntimeOrigin::root(),
			Box::new(crate::runtime::RuntimeCall::CollatorSelection(
				pallet_collator_selection::Call::set_candidacy_bond { bond: bond_amount }
			))
		));
		set_vesting_schedule(&CHARLIE, bond_amount * 2);
		assert_eq!(pallet_collator_selection::CandidateList::<Test>::get().len(), 0);

		link_stash_to_controller(CHARLIE, DAVE);
		set_session_keys(DAVE);
		assert_ok!(CollatorSelection::register_as_candidate(RuntimeOrigin::signed(CHARLIE)));
		assert_eq!(pallet_collator_selection::CandidateList::<Test>::get().len(), 1);
		assert_eq!(CollatorManager::reserved_balance(&CHARLIE), bond_amount);
		let lock = get_collator_bond_lock(&CHARLIE).expect("collator bond lock should exist");
		assert_eq!(lock.amount, bond_amount);
	});
}

#[test]
fn set_collator_reward_works() {
	new_test_ext().execute_with(|| {
		let new_reward = 100 * UNIT;
		assert_ne!(CollatorManager::collator_reward(), new_reward);

		assert_ok!(CollatorManager::set_collator_reward(RuntimeOrigin::root(), new_reward));

		assert_eq!(CollatorManager::collator_reward(), new_reward);
	});
}

#[test]
fn note_author_pays_reward_from_treasury() {
	new_test_ext().execute_with(|| {
		let reward_amount = 50 * UNIT;
		let treasury = <Test as pallet_collator_manager::Config>::TreasuryAccount::get()
			.into_account_truncating();

		Balances::set_balance(&treasury, 1000 * UNIT);
		assert_ok!(CollatorManager::set_collator_reward(RuntimeOrigin::root(), reward_amount));

		let author_initial_balance = Balances::free_balance(&ALICE);
		let treasury_initial_balance = Balances::free_balance(&treasury);

		CollatorManager::note_author(ALICE);

		let author_final_balance = Balances::free_balance(&ALICE);
		let treasury_final_balance = Balances::free_balance(&treasury);

		assert_eq!(author_final_balance, author_initial_balance + reward_amount);
		assert_eq!(treasury_final_balance, treasury_initial_balance - reward_amount);
	});
}

#[test]
fn register_controller_works() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;
		let controller = BOB;

		assert_ok!(CollatorManager::reserve(&stash, 100 * UNIT));

		link_stash_to_controller(stash.clone(), controller.clone());

		assert_eq!(
			pallet_collator_manager::Controller::<Test>::get(&stash),
			Some(controller.clone())
		);
		assert_eq!(pallet_collator_manager::Stash::<Test>::get(&controller), Some(stash));
	});
}

#[test]
fn set_controller_works() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;
		let old_controller = BOB;
		let new_controller = CHARLIE;
		assert_ok!(CollatorManager::reserve(&stash, 100 * UNIT));
		link_stash_to_controller(stash.clone(), old_controller.clone());

		rotate_controller(stash.clone(), new_controller.clone());

		assert_eq!(
			pallet_collator_manager::Controller::<Test>::get(&stash),
			Some(new_controller.clone())
		);
		assert_eq!(pallet_collator_manager::Stash::<Test>::get(&old_controller), None);
		assert_eq!(pallet_collator_manager::Stash::<Test>::get(&new_controller), Some(stash));
	});
}

#[test]
fn deregister_works() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;
		let controller = BOB;
		assert_ok!(CollatorManager::reserve(&stash, 100 * UNIT));
		link_stash_to_controller(stash.clone(), controller.clone());

		assert_ok!(CollatorManager::deregister(RuntimeOrigin::signed(stash.clone())));

		assert_eq!(pallet_collator_manager::Controller::<Test>::get(&stash), None);
		assert_eq!(pallet_collator_manager::Stash::<Test>::get(&controller), None);
	});
}

#[test]
fn validator_registration_returns_false_when_no_controller() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;

		assert!(!CollatorManager::is_registered(&stash));
	});
}

#[test]
fn validator_registration_returns_false_when_controller_has_no_session_keys() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;
		let controller = BOB;

		assert_ok!(CollatorManager::reserve(&stash, 100 * UNIT));
		link_stash_to_controller(stash.clone(), controller.clone());

		assert!(!CollatorManager::is_registered(&stash));
	});
}

#[test]
fn validator_registration_returns_true_when_controller_has_session_keys() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;
		let controller = BOB;

		assert_ok!(CollatorManager::reserve(&stash, 100 * UNIT));
		link_stash_to_controller(stash.clone(), controller.clone());

		set_session_keys(controller.clone());

		assert!(CollatorManager::is_registered(&stash));
	});
}

#[test]
fn validator_registration_returns_false_after_controller_changed_without_new_keys() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;
		let old_controller = BOB;
		let new_controller = CHARLIE;

		assert_ok!(CollatorManager::reserve(&stash, 100 * UNIT));
		link_stash_to_controller(stash.clone(), old_controller.clone());
		set_session_keys(old_controller.clone());
		assert!(CollatorManager::is_registered(&stash));

		rotate_controller(stash.clone(), new_controller.clone());

		assert!(!CollatorManager::is_registered(&stash));

		set_session_keys(new_controller.clone());
		assert!(CollatorManager::is_registered(&stash));
	});
}

#[test]
fn update_bond_fails_when_new_deposit_exceeds_account_balance() {
	new_test_ext().execute_with(|| {
		let stash = CHARLIE;
		let controller = DAVE;

		let account_balance = 100 * UNIT;
		let initial_bond = 50 * UNIT;
		let first_update = 100 * UNIT;
		let over_balance = 150 * UNIT;

		Balances::set_balance(&stash, account_balance);

		assert_ok!(Sudo::sudo(
			RuntimeOrigin::root(),
			Box::new(crate::runtime::RuntimeCall::CollatorSelection(
				pallet_collator_selection::Call::set_candidacy_bond { bond: initial_bond }
			))
		));

		link_stash_to_controller(stash.clone(), controller.clone());
		set_session_keys(controller);

		assert_ok!(CollatorSelection::register_as_candidate(RuntimeOrigin::signed(stash.clone())));
		assert_eq!(CollatorManager::reserved_balance(&stash), initial_bond);

		assert_ok!(CollatorSelection::update_bond(
			RuntimeOrigin::signed(stash.clone()),
			first_update
		));
		assert_eq!(CollatorManager::reserved_balance(&stash), first_update);

		assert_err!(
			CollatorSelection::update_bond(RuntimeOrigin::signed(stash.clone()), over_balance),
			Error::<Test>::InsufficientBalance
		);

		assert_eq!(CollatorManager::reserved_balance(&stash), first_update);
		let lock = get_collator_bond_lock(&stash).expect("bond lock should exist");
		assert_eq!(lock.amount, first_update);
		assert_eq!(Balances::free_balance(&stash), account_balance);
	});
}

#[test]
fn take_candidate_slot_replaces_a_fully_bonded_candidate() {
	new_test_ext().execute_with(|| {
		let min_bond = 50 * UNIT;
		let target_balance = 100 * UNIT;
		let challenger_balance = 120 * UNIT;
		let over_balance = 150 * UNIT;

		assert_ok!(Sudo::sudo(
			RuntimeOrigin::root(),
			Box::new(crate::runtime::RuntimeCall::CollatorSelection(
				pallet_collator_selection::Call::set_candidacy_bond { bond: min_bond }
			))
		));

		let target_stash = AccountId32::new([31u8; 32]);
		let target_controller = AccountId32::new([32u8; 32]);
		let challenger_stash = AccountId32::new([33u8; 32]);
		let challenger_controller = AccountId32::new([34u8; 32]);

		Balances::set_balance(&target_stash, target_balance);
		Balances::set_balance(&challenger_stash, challenger_balance);

		link_stash_to_controller(target_stash.clone(), target_controller.clone());
		set_session_keys(target_controller);

		assert_ok!(CollatorSelection::register_as_candidate(RuntimeOrigin::signed(
			target_stash.clone()
		)));
		assert_ok!(CollatorSelection::update_bond(
			RuntimeOrigin::signed(target_stash.clone()),
			target_balance
		));

		assert_err!(
			CollatorSelection::update_bond(
				RuntimeOrigin::signed(target_stash.clone()),
				over_balance
			),
			Error::<Test>::InsufficientBalance
		);

		let target_info = pallet_collator_selection::CandidateList::<Test>::get()
			.into_iter()
			.find(|info| info.who == target_stash)
			.expect("target should still be a candidate");
		assert_eq!(target_info.deposit, target_balance);

		link_stash_to_controller(challenger_stash.clone(), challenger_controller.clone());
		set_session_keys(challenger_controller);

		assert_ok!(CollatorSelection::take_candidate_slot(
			RuntimeOrigin::signed(challenger_stash.clone()),
			challenger_balance,
			target_stash.clone()
		));

		let final_candidates = pallet_collator_selection::CandidateList::<Test>::get();
		assert!(final_candidates
			.iter()
			.any(|info| info.who == challenger_stash && info.deposit == challenger_balance));
		assert!(!final_candidates.iter().any(|info| info.who == target_stash));
	});
}

#[test]
fn register_fails_without_controller_approval() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;
		let controller = BOB;

		assert_err!(
			CollatorManager::register(RuntimeOrigin::signed(stash), controller),
			Error::<Test>::ControllerApprovalMissing,
		);
	});
}

#[test]
fn register_fails_when_approval_is_for_a_different_stash() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;
		let other_stash = CHARLIE;
		let controller = BOB;

		// Approval is recorded for `(other_stash, controller)`, not for `(stash, controller)`.
		assert_ok!(CollatorManager::approve_controller(
			RuntimeOrigin::signed(controller.clone()),
			other_stash,
		));

		assert_err!(
			CollatorManager::register(RuntimeOrigin::signed(stash), controller),
			Error::<Test>::ControllerApprovalMissing,
		);
	});
}

#[test]
fn approval_is_single_use_and_cleared_by_register() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;
		let controller = BOB;

		link_stash_to_controller(stash.clone(), controller.clone());

		// Approval must be consumed by the successful register.
		assert!(pallet_collator_manager::ControllerApprovals::<Test>::get(&stash, &controller)
			.is_none());

		// Deregister the pair, then attempt to re-register without a fresh approval.
		assert_ok!(CollatorManager::deregister(RuntimeOrigin::signed(stash.clone())));
		assert_err!(
			CollatorManager::register(
				RuntimeOrigin::signed(stash.clone()),
				controller.clone(),
			),
			Error::<Test>::ControllerApprovalMissing,
		);
	});
}

#[test]
fn set_controller_fails_without_new_controller_approval() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;
		let old_controller = BOB;
		let new_controller = CHARLIE;

		link_stash_to_controller(stash.clone(), old_controller);

		assert_err!(
			CollatorManager::set_controller(
				RuntimeOrigin::signed(stash),
				new_controller,
			),
			Error::<Test>::ControllerApprovalMissing,
		);
	});
}

#[test]
fn revoke_controller_approval_works() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;
		let controller = BOB;

		// Controller grants then revokes consent.
		assert_ok!(CollatorManager::approve_controller(
			RuntimeOrigin::signed(controller.clone()),
			stash.clone(),
		));
		assert_ok!(CollatorManager::revoke_controller_approval(
			RuntimeOrigin::signed(controller.clone()),
			stash.clone(),
		));

		// Revoked approval must be cleared from storage.
		assert!(pallet_collator_manager::ControllerApprovals::<Test>::get(&stash, &controller)
			.is_none());

		// Subsequent `register` from the stash now fails.
		assert_err!(
			CollatorManager::register(RuntimeOrigin::signed(stash.clone()), controller.clone()),
			Error::<Test>::ControllerApprovalMissing,
		);

		// A second revoke with nothing to revoke is rejected.
		assert_err!(
			CollatorManager::revoke_controller_approval(
				RuntimeOrigin::signed(controller),
				stash,
			),
			Error::<Test>::NoPendingApproval,
		);
	});
}

#[test]
fn repeated_reserve_calls_respect_total_balance() {
	new_test_ext().execute_with(|| {
		Balances::set_balance(&CHARLIE, 100 * UNIT);

		assert_ok!(CollatorManager::reserve(&CHARLIE, 60 * UNIT));
		assert_ok!(CollatorManager::reserve(&CHARLIE, 40 * UNIT));

		assert_err!(CollatorManager::reserve(&CHARLIE, 1), Error::<Test>::InsufficientBalance);

		assert_eq!(CollatorManager::reserved_balance(&CHARLIE), 100 * UNIT);
		assert_eq!(Balances::free_balance(&CHARLIE), 100 * UNIT);
	});
}
