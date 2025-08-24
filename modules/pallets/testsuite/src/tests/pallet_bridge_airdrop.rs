#![cfg(test)]

use codec::Encode;
use pallet_bridge_airdrop::{
	KeccakHasher, MerkleRoot, Proof, EIGHTEEN_MONTHS, ETHEREUM_MESSAGE_PREFIX, SIX_MONTHS,
};
use polkadot_sdk::{
	frame_support::{assert_err, assert_noop, crypto::ecdsa::ECDSAExt},
	frame_system, pallet_vesting,
	sp_runtime::{Permill, TokenError},
};
use rs_merkle::MerkleTree;
use sp_core::{crypto::AccountId32, keccak_256, Pair, H160, H256};

use crate::runtime::{new_test_ext, Balances, RuntimeOrigin, Test};

struct ProofGen {
	root: H256,
	proof_items: Vec<H256>,
	leaf: (Vec<u8>, u128),
}

fn generate_merkle_tree_and_proof(leaf_count: usize, leaf_index: usize, who: Vec<u8>) -> ProofGen {
	let mut tree = MerkleTree::<KeccakHasher>::new();
	let amount = 3500_000_000_000_000u128;
	for i in 0..leaf_count {
		if leaf_index == i {
			let encoded_leaf = if who.len() == 20 {
				(H160::from_slice(&who), amount).encode()
			} else {
				(H256::from_slice(&who), amount).encode()
			};
			let leaf_hash = keccak_256(&encoded_leaf);
			tree.insert(leaf_hash).commit();
		} else {
			let temp = H160::random();
			let leaf_hash = keccak_256(&(temp, amount).encode());
			tree.insert(leaf_hash).commit();
		}
	}

	let proof = tree.proof(&[leaf_index]);
	let proof_items = proof.proof_hashes().into_iter().map(|val| val.into()).collect();
	ProofGen { root: tree.root().unwrap().into(), proof_items, leaf: (who, amount) }
}

#[test]
fn should_claim_airdrop_correctly() {
	new_test_ext().execute_with(|| {
		let leaf_count = 500usize;
		let leaf_index = 250usize;
		frame_system::Pallet::<Test>::set_block_number(0);

		let pair = sp_core::ecdsa::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
		let eth_address = pair.public().to_eth_address().unwrap().to_vec();

		let proof_gen = generate_merkle_tree_and_proof(leaf_count, leaf_index, eth_address.clone());

		let beneficiary = AccountId32::new(H256::random().0);

		let payload = beneficiary.encode();

		let preimage = vec![
			format!("{ETHEREUM_MESSAGE_PREFIX}{}", payload.len()).as_bytes().to_vec(),
			payload,
		]
		.concat();
		let message = keccak_256(&preimage);

		let signature = pair.sign_prehashed(&message).0;

		let params = Proof {
			who: H160::from_slice(&eth_address),
			beneficiary: beneficiary.clone(),
			signature: signature.to_vec(),
			proof_items: proof_gen.proof_items,
			leaf_index: leaf_index as u64,
			amount: proof_gen.leaf.1,
		};

		MerkleRoot::<Test>::put((proof_gen.root, leaf_count as u64));

		pallet_bridge_airdrop::Pallet::<Test>::claim_tokens(RuntimeOrigin::none(), params.clone())
			.unwrap();

		let account_data = frame_system::Account::<Test>::get(beneficiary.clone());

		let initial_unlocked = Permill::from_parts(250_000) * params.amount;
		let locked = params.amount.saturating_sub(initial_unlocked);

		assert_eq!(account_data.data.free, params.amount);

		// transfer above unlocked balance should fail
		let res = Balances::transfer_keep_alive(
			RuntimeOrigin::signed(beneficiary.clone()),
			AccountId32::new(H256::random().0),
			initial_unlocked.saturating_add(1),
		);

		assert_noop!(res, TokenError::Frozen);

		// transfer below unlocked balance should succeed
		Balances::transfer_keep_alive(
			RuntimeOrigin::signed(beneficiary.clone()),
			AccountId32::new(H256::random().0),
			initial_unlocked,
		)
		.unwrap();
		let current_locked = account_data.data.frozen;

		let vested_days = EIGHTEEN_MONTHS / 5;

		frame_system::Pallet::<Test>::set_block_number(vested_days);

		pallet_vesting::Pallet::<Test>::vest(RuntimeOrigin::signed(beneficiary.clone())).unwrap();

		let account_data = frame_system::Account::<Test>::get(beneficiary);

		let unlock_per_block = locked / EIGHTEEN_MONTHS as u128;

		let unlocked = unlock_per_block * vested_days as u128;

		assert_eq!(account_data.data.frozen, current_locked - unlocked);

		let res = pallet_bridge_airdrop::Pallet::<Test>::claim_tokens(
			RuntimeOrigin::none(),
			params.clone(),
		);

		assert_err!(res, pallet_bridge_airdrop::pallet::Error::<Test>::AlreadyClaimed);
	})
}

#[test]
fn should_allocate_iro_correctly() {
	new_test_ext().execute_with(|| {
		frame_system::Pallet::<Test>::set_block_number(0);

		let beneficiary = AccountId32::new(H256::random().0);
		let amount = 3500_000_000_000_000u128;
		let bonus_amount = 3500_000_000_000u128;

		pallet_bridge_airdrop::Pallet::<Test>::allocate_iro_tokens(
			RuntimeOrigin::root(),
			beneficiary.clone(),
			amount,
			bonus_amount,
		)
		.unwrap();

		let account_data = frame_system::Account::<Test>::get(beneficiary.clone());

		let initial_unlocked = Permill::from_parts(250_000) * amount;
		let locked = amount.saturating_sub(initial_unlocked).saturating_add(bonus_amount);

		assert_eq!(account_data.data.free, amount.saturating_add(bonus_amount));

		// transfer above unlocked balance should fail
		let res = Balances::transfer_keep_alive(
			RuntimeOrigin::signed(beneficiary.clone()),
			AccountId32::new(H256::random().0),
			initial_unlocked.saturating_add(1),
		);

		assert_noop!(res, TokenError::Frozen);

		// transfer below unlocked balance should succeed
		Balances::transfer_keep_alive(
			RuntimeOrigin::signed(beneficiary.clone()),
			AccountId32::new(H256::random().0),
			initial_unlocked,
		)
		.unwrap();
		let current_locked = account_data.data.frozen;

		let vested_days = EIGHTEEN_MONTHS / 5;

		frame_system::Pallet::<Test>::set_block_number(vested_days);

		pallet_vesting::Pallet::<Test>::vest(RuntimeOrigin::signed(beneficiary.clone())).unwrap();

		let account_data = frame_system::Account::<Test>::get(beneficiary.clone());

		let unlock_per_block = locked / EIGHTEEN_MONTHS as u128;

		let unlocked = unlock_per_block * vested_days as u128;

		assert_eq!(account_data.data.frozen, current_locked - unlocked);

		let res = pallet_bridge_airdrop::Pallet::<Test>::allocate_iro_tokens(
			RuntimeOrigin::root(),
			beneficiary.clone(),
			amount.clone(),
			bonus_amount,
		);

		assert_err!(res, pallet_bridge_airdrop::pallet::Error::<Test>::AlreadyAllocated);
	})
}

#[test]
fn should_allocate_crowdloan_correctly() {
	new_test_ext().execute_with(|| {
		frame_system::Pallet::<Test>::set_block_number(0);

		let beneficiary = AccountId32::new(H256::random().0);
		let amount = 3500_000_000_000_000u128;

		pallet_bridge_airdrop::Pallet::<Test>::allocate_crowdloan_tokens(
			RuntimeOrigin::root(),
			beneficiary.clone(),
			amount,
		)
		.unwrap();

		let _schedule = pallet_vesting::Vesting::<Test>::get(beneficiary.clone());

		let account_data = frame_system::Account::<Test>::get(beneficiary.clone());

		let locked = amount;

		assert_eq!(account_data.data.frozen, amount);

		// transfer above unlocked balance should fail
		let res = Balances::transfer_keep_alive(
			RuntimeOrigin::signed(beneficiary.clone()),
			AccountId32::new(H256::random().0),
			1u128,
		);

		assert_noop!(res, TokenError::Frozen);

		let current_locked = account_data.data.frozen;

		let today = SIX_MONTHS as u64 + (EIGHTEEN_MONTHS / 5);

		frame_system::Pallet::<Test>::set_block_number(today);

		pallet_vesting::Pallet::<Test>::vest(RuntimeOrigin::signed(beneficiary.clone())).unwrap();

		let account_data = frame_system::Account::<Test>::get(beneficiary.clone());

		let unlock_per_block = locked / EIGHTEEN_MONTHS as u128;

		let vested_days = today - SIX_MONTHS as u64;
		let unlocked = unlock_per_block * vested_days as u128;

		assert_eq!(account_data.data.frozen, current_locked - unlocked);

		let res = pallet_bridge_airdrop::Pallet::<Test>::allocate_crowdloan_tokens(
			RuntimeOrigin::root(),
			beneficiary.clone(),
			amount,
		);

		assert_err!(res, pallet_bridge_airdrop::pallet::Error::<Test>::AlreadyAllocated);
	})
}
