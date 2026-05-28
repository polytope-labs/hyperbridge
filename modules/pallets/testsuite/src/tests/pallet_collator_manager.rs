use crate::runtime::{
	new_test_ext, Assets, Balances, CollatorManager, CollatorSelection, ReputationAssetId,
	RuntimeOrigin, Session, Test, ALICE, BOB, CHARLIE, DAVE, INITIAL_BALANCE, UNIT,
};
use frame_system::Pallet as System;
use pallet_collator_manager::Error;
use pallet_session;
use polkadot_sdk::{
	frame_support::{
		assert_err, assert_ok,
		traits::{
			fungible::Mutate as BalanceMutate, fungibles::Mutate, OnInitialize, ReservableCurrency,
			ValidatorRegistration,
		},
	},
	pallet_authorship::EventHandler,
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
	assert_ok!(CollatorManager::set_controller(RuntimeOrigin::signed(stash), new_controller,));
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
fn test_active_collator_that_is_still_a_candidate_is_reselected() {
	new_test_ext().execute_with(|| {
		create_reputation_asset();

		let alice_stash = AccountId32::new([11; 32]);
		Balances::set_balance(&alice_stash, INITIAL_BALANCE);
		let charlie_stash = AccountId32::new([13; 32]);
		Balances::set_balance(&charlie_stash, INITIAL_BALANCE);

		// Alice is last session's collator and is still a registered candidate; Charlie is a
		// fresh one. Both are selected, so already sitting in the active set no longer keeps a
		// candidate out.
		set_session_keys(ALICE);
		pallet_session::Validators::<Test>::put(vec![ALICE]);
		pallet_session::QueuedKeys::<Test>::put(vec![(
			ALICE,
			crate::runtime::SessionKeys { aura: Pair::from_seed(ALICE.as_ref()).public().into() },
		)]);

		link_stash_to_controller(alice_stash.clone(), ALICE);
		register_candidate(alice_stash.clone());
		set_reputation_balance(&ALICE, 50 * UNIT);

		link_stash_to_controller(charlie_stash.clone(), CHARLIE);
		set_session_keys(CHARLIE);
		register_candidate(charlie_stash.clone());
		set_reputation_balance(&CHARLIE, 40 * UNIT);

		Session::on_initialize(2);
		Session::on_initialize(3);

		let mut new_collators = Session::validators();
		new_collators.sort();
		assert_eq!(new_collators, vec![ALICE, CHARLIE]);
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

		link_stash_to_controller(stash.clone(), controller.clone());

		assert!(!CollatorManager::is_registered(&stash));
	});
}

#[test]
fn validator_registration_returns_true_when_controller_has_session_keys() {
	new_test_ext().execute_with(|| {
		let stash = ALICE;
		let controller = BOB;

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
			CollatorManager::register(RuntimeOrigin::signed(stash.clone()), controller.clone(),),
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
			CollatorManager::set_controller(RuntimeOrigin::signed(stash), new_controller,),
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
			CollatorManager::revoke_controller_approval(RuntimeOrigin::signed(controller), stash,),
			Error::<Test>::NoPendingApproval,
		);
	});
}

/// Make `stash` a bonded candidate paired to `controller` with session keys.
fn setup_bonded_collator(
	stash: <Test as frame_system::Config>::AccountId,
	controller: <Test as frame_system::Config>::AccountId,
) {
	Balances::set_balance(&stash, INITIAL_BALANCE);
	link_stash_to_controller(stash.clone(), controller.clone());
	set_session_keys(controller);
	assert_ok!(CollatorSelection::register_as_candidate(RuntimeOrigin::signed(stash)));
}

#[test]
fn unbond_fails_when_not_a_candidate() {
	new_test_ext().execute_with(|| {
		create_reputation_asset();
		assert_err!(CollatorManager::unbond(RuntimeOrigin::signed(ALICE)), Error::<Test>::NoBond);
	});
}

#[test]
fn unbond_stops_the_collator_being_selected() {
	new_test_ext().execute_with(|| {
		create_reputation_asset();
		let alice_stash = AccountId32::new([11; 32]);
		let bob_stash = AccountId32::new([12; 32]);
		setup_bonded_collator(alice_stash.clone(), ALICE);
		setup_bonded_collator(bob_stash.clone(), BOB);
		set_reputation_balance(&ALICE, 50 * UNIT);
		set_reputation_balance(&BOB, 40 * UNIT);

		let mut selected =
			<CollatorManager as pallet_session::SessionManager<AccountId32>>::new_session(0)
				.unwrap();
		selected.sort();
		assert_eq!(selected, vec![ALICE, BOB]);

		assert_ok!(CollatorManager::unbond(RuntimeOrigin::signed(alice_stash.clone())));
		assert!(pallet_collator_manager::Unbonding::<Test>::contains_key(&alice_stash));

		let selected =
			<CollatorManager as pallet_session::SessionManager<AccountId32>>::new_session(1)
				.unwrap();
		assert_eq!(selected, vec![BOB]);
	});
}

#[test]
fn withdraw_unbonded_fails_before_the_delay() {
	new_test_ext().execute_with(|| {
		create_reputation_asset();
		let alice_stash = AccountId32::new([11; 32]);
		setup_bonded_collator(alice_stash.clone(), ALICE);

		assert_ok!(CollatorManager::unbond(RuntimeOrigin::signed(alice_stash.clone())));
		assert_err!(
			CollatorManager::withdraw_unbonded(RuntimeOrigin::signed(alice_stash)),
			Error::<Test>::UnbondingPeriodNotElapsed
		);
	});
}

#[test]
fn withdraw_unbonded_releases_the_bond_after_the_delay() {
	new_test_ext().execute_with(|| {
		create_reputation_asset();
		let bond = 100 * UNIT;
		pallet_collator_selection::CandidacyBond::<Test>::put(bond);

		// `leave_intent` (used internally) needs more than `MinEligibleCollators` candidates, so
		// three are bonded and only one unbonds.
		let alice_stash = AccountId32::new([11; 32]);
		setup_bonded_collator(alice_stash.clone(), ALICE);
		setup_bonded_collator(AccountId32::new([12; 32]), BOB);
		setup_bonded_collator(AccountId32::new([13; 32]), CHARLIE);
		assert_eq!(Balances::reserved_balance(&alice_stash), bond);

		assert_ok!(CollatorManager::unbond(RuntimeOrigin::signed(alice_stash.clone())));
		let withdrawable_at =
			pallet_collator_manager::Unbonding::<Test>::get(&alice_stash).unwrap();
		System::<Test>::set_block_number(withdrawable_at);

		assert_ok!(CollatorManager::withdraw_unbonded(RuntimeOrigin::signed(alice_stash.clone())));

		assert_eq!(Balances::reserved_balance(&alice_stash), 0);
		assert!(pallet_collator_manager::Unbonding::<Test>::get(&alice_stash).is_none());
		assert!(!pallet_collator_selection::CandidateList::<Test>::get()
			.iter()
			.any(|candidate| candidate.who == alice_stash));
	});
}

#[test]
fn root_can_remove_and_reinstate_a_validator() {
	new_test_ext().execute_with(|| {
		create_reputation_asset();
		let alice_stash = AccountId32::new([11; 32]);
		setup_bonded_collator(alice_stash.clone(), ALICE);
		set_reputation_balance(&ALICE, 50 * UNIT);
		pallet_session::Validators::<Test>::put(vec![ALICE]);

		assert_ok!(CollatorManager::remove_validator(RuntimeOrigin::root(), ALICE));
		assert!(pallet_collator_manager::RemovedValidators::<Test>::contains_key(&ALICE));
		// `Validators` is intentionally left alone — mutating it mid-session would shift
		// the indices `FindAccountFromAuthorIndex` reads, mis-attributing block rewards
		// for the remainder of the session. The removal only takes effect at the next
		// session boundary, via the `new_session` filter below.
		assert!(pallet_session::Validators::<Test>::get().contains(&ALICE));
		// A removed validator is skipped even though it is still a bonded candidate.
		assert!(<CollatorManager as pallet_session::SessionManager<AccountId32>>::new_session(0)
			.is_none());

		assert_ok!(CollatorManager::reinstate_validator(RuntimeOrigin::root(), ALICE));
		assert!(!pallet_collator_manager::RemovedValidators::<Test>::contains_key(&ALICE));
		assert_eq!(
			<CollatorManager as pallet_session::SessionManager<AccountId32>>::new_session(1),
			Some(vec![ALICE])
		);
	});
}
