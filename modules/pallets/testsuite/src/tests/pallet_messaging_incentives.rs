#![cfg(test)]

//! Integration tests for the slim `pallet-messaging-incentives`. The pallet
//! mints reputation tokens to relayers per byte delivered; the rate
//! is set by governance and zero disables minting.

use codec::Encode;
use polkadot_sdk::{
	frame_support::{
		dispatch::GetDispatchInfo,
		traits::{
			fungible::Mutate as FungibleMutate,
			fungibles::{Inspect, Mutate},
			tokens::{Fortitude, Precision, Preservation},
		},
	},
	frame_system, pallet_assets,
	sp_runtime::{traits::Dispatchable, Weight},
};
use sp_core::{crypto::AccountId32, keccak_256, sr25519, ByteArray, Pair};

use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::{Message, MessageWithWeight, Proof, RequestMessage},
	router::PostRequest,
};
use pallet_ismp::fee_handler::FeeHandler;
use pallet_ismp_relayer::withdrawal::Signature;
use pallet_messaging_incentives::MintPerByte;

use crate::{
	runtime::{
		new_test_ext, Assets, MessagingRelayerIncentives, ReputationAsset, ReputationAssetId,
		RuntimeCall, RuntimeOrigin, Test, BOB,
	},
	tests::common::setup_relayer_and_asset,
};

const SOURCE: StateMachine = StateMachine::Evm(1);
const DEST: StateMachine = StateMachine::Polkadot(1000);

/// Builds a `MessageWithWeight` that the slim pallet's `on_executed`
/// will treat as relayer-signed — the relayer account derives from
/// the sr25519 signature on the encoded `requests`.
fn signed_request(relayer: &sr25519::Pair, body: Vec<u8>) -> MessageWithWeight {
	let post = PostRequest {
		source: SOURCE,
		dest: DEST,
		nonce: 0,
		from: vec![1; 32],
		to: vec![2; 32],
		timeout_timestamp: 100,
		body,
	};
	let requests = vec![post];
	let signed = keccak_256(&requests.encode());
	let sig = relayer.sign(&signed);
	let signer = Signature::Sr25519 {
		public_key: relayer.public().to_raw_vec(),
		signature: sig.to_raw_vec(),
	}
	.encode();

	MessageWithWeight {
		message: Message::Request(RequestMessage {
			requests,
			proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId { state_id: SOURCE, consensus_state_id: *b"mock" },
					height: 1,
				},
				proof: vec![],
			},
			signer,
		}),
		weight: Weight::zero(),
	}
}

fn relayer_balance(account: &AccountId32) -> u128 {
	Assets::balance(ReputationAssetId::get(), account)
}

#[test]
fn set_mint_per_byte_updates_rate() {
	new_test_ext().execute_with(|| {
		assert_eq!(MintPerByte::<Test>::get(), 0);
		MessagingRelayerIncentives::set_mint_per_byte(RuntimeOrigin::root(), 7).unwrap();
		assert_eq!(MintPerByte::<Test>::get(), 7);

		// Zero re-disables the mint.
		MessagingRelayerIncentives::set_mint_per_byte(RuntimeOrigin::root(), 0).unwrap();
		assert_eq!(MintPerByte::<Test>::get(), 0);
	});
}

#[test]
fn on_executed_mints_reputation_proportional_to_bytes() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&[7u8; 32]);
		let relayer_account = AccountId32::new(relayer_pair.public().0);
		setup_relayer_and_asset(&relayer_account);

		let rate: u128 = 3;
		MessagingRelayerIncentives::set_mint_per_byte(RuntimeOrigin::root(), rate).unwrap();

		let body = vec![0u8; 100];
		let msg = signed_request(&relayer_pair, body.clone());
		MessagingRelayerIncentives::on_executed(vec![msg], vec![]).unwrap();

		assert_eq!(relayer_balance(&relayer_account), rate * 100);
	});
}

/// The bandwidth gate counts a 32-byte minimum even on empty bodies;
/// the mint follows the same floor so undersized payloads can't sneak
/// in for free.
#[test]
fn on_executed_applies_thirty_two_byte_floor() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&[8u8; 32]);
		let relayer_account = AccountId32::new(relayer_pair.public().0);
		setup_relayer_and_asset(&relayer_account);

		MessagingRelayerIncentives::set_mint_per_byte(RuntimeOrigin::root(), 1).unwrap();

		let msg = signed_request(&relayer_pair, vec![]);
		MessagingRelayerIncentives::on_executed(vec![msg], vec![]).unwrap();

		assert_eq!(relayer_balance(&relayer_account), 32);
	});
}

#[test]
fn on_executed_does_not_mint_when_rate_is_zero() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&[9u8; 32]);
		let relayer_account = AccountId32::new(relayer_pair.public().0);
		setup_relayer_and_asset(&relayer_account);

		// MintPerByte defaults to 0.
		let msg = signed_request(&relayer_pair, vec![0u8; 100]);
		MessagingRelayerIncentives::on_executed(vec![msg], vec![]).unwrap();

		assert_eq!(relayer_balance(&relayer_account), 0);
	});
}

#[test]
fn set_mint_per_byte_requires_admin_origin() {
	new_test_ext().execute_with(|| {
		let alice: AccountId32 = AccountId32::new([1u8; 32]);
		MessagingRelayerIncentives::set_mint_per_byte(RuntimeOrigin::signed(alice), 5)
			.expect_err("non-admin must be rejected");
		assert_eq!(MintPerByte::<Test>::get(), 0);
	});
}

/// Each `on_executed` call mints to the relayer; the soulbound semantics
/// live in the runtime call filter, not in the mint path, so successive
/// mints to the same account must accumulate normally.
#[test]
fn on_executed_mints_accumulate_across_deliveries() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&[11u8; 32]);
		let relayer_account = AccountId32::new(relayer_pair.public().0);
		setup_relayer_and_asset(&relayer_account);

		MessagingRelayerIncentives::set_mint_per_byte(RuntimeOrigin::root(), 1).unwrap();

		let body = vec![0u8; 100];
		MessagingRelayerIncentives::on_executed(
			vec![signed_request(&relayer_pair, body.clone())],
			vec![],
		)
		.unwrap();
		assert_eq!(relayer_balance(&relayer_account), 100);

		MessagingRelayerIncentives::on_executed(
			vec![signed_request(&relayer_pair, body)],
			vec![],
		)
		.unwrap();
		assert_eq!(relayer_balance(&relayer_account), 200);
	});
}

/// Reputation is burned by `pallet-collator-manager` on each session
/// rotation to reset accumulated rewards. Soulbound enforcement lives at
/// the dispatch layer (`ReputationCallFilter`) rather than at the asset
/// layer, so programmatic `fungible::Mutate::burn_from` must keep working
/// — otherwise the rotation reset would silently no-op and reputation
/// would accumulate forever, breaking the per-session ranking.
#[test]
fn programmatic_burn_still_works_on_reputation_holder() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&[12u8; 32]);
		let relayer_account = AccountId32::new(relayer_pair.public().0);
		setup_relayer_and_asset(&relayer_account);

		MessagingRelayerIncentives::set_mint_per_byte(RuntimeOrigin::root(), 1).unwrap();
		MessagingRelayerIncentives::on_executed(
			vec![signed_request(&relayer_pair, vec![0u8; 200])],
			vec![],
		)
		.unwrap();
		assert_eq!(relayer_balance(&relayer_account), 200);

		// Mirrors `pallet-collator-manager::new_session` — burn the full
		// balance to reset reputation across sessions.
		ReputationAsset::burn_from(
			&relayer_account,
			200,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Polite,
		)
		.expect("programmatic burn must succeed on a reputation holder");

		assert_eq!(relayer_balance(&relayer_account), 0);
	});
}

/// The whole point of the soulbound treatment: a relayer that earned
/// reputation cannot dispatch a transfer of it to another account. The
/// rejection must come from the runtime call filter (so an unsigned
/// holder-driven dispatch fails); we verify that by routing through
/// `RuntimeCall::dispatch`, which is what production extrinsics use.
#[test]
fn dispatched_transfer_of_reputation_is_filtered() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&[13u8; 32]);
		let relayer_account = AccountId32::new(relayer_pair.public().0);
		setup_relayer_and_asset(&relayer_account);

		MessagingRelayerIncentives::set_mint_per_byte(RuntimeOrigin::root(), 1).unwrap();
		MessagingRelayerIncentives::on_executed(
			vec![signed_request(&relayer_pair, vec![0u8; 100])],
			vec![],
		)
		.unwrap();
		assert_eq!(relayer_balance(&relayer_account), 100);

		let call = RuntimeCall::Assets(pallet_assets::Call::transfer {
			id: ReputationAssetId::get(),
			target: BOB.clone().into(),
			amount: 50,
		});
		// Bring `weigh_data` into scope so we exercise the same dispatch path
		// the runtime uses; assert the filter rejects rather than the inner call.
		let _ = call.get_dispatch_info();
		let err = call
			.dispatch(RuntimeOrigin::signed(relayer_account.clone()))
			.expect_err("reputation transfer must be filtered");
		assert_eq!(err.error, frame_system::Error::<Test>::CallFiltered.into());

		assert_eq!(relayer_balance(&relayer_account), 100);
		assert_eq!(relayer_balance(&BOB), 0);
	});
}

/// A non-reputation asset must NOT be filtered — the call filter only
/// targets the reputation asset id.
#[test]
fn dispatched_transfer_of_other_asset_not_filtered() {
	new_test_ext().execute_with(|| {
		use polkadot_sdk::frame_support::assert_ok;

		let other_asset = sp_core::H256::repeat_byte(0x42);
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			other_asset,
			crate::runtime::ALICE,
			true,
			1,
		));
		Assets::set_balance(other_asset, &crate::runtime::ALICE, 1_000);

		let call = RuntimeCall::Assets(pallet_assets::Call::transfer {
			id: other_asset,
			target: BOB.clone().into(),
			amount: 100,
		});
		call.dispatch(RuntimeOrigin::signed(crate::runtime::ALICE))
			.expect("non-reputation asset transfer must pass the filter");
		assert_eq!(Assets::balance(other_asset, &BOB), 100);
	});
}

#[test]
fn unsigned_message_does_not_mint() {
	new_test_ext().execute_with(|| {
		let relayer_pair = sr25519::Pair::from_seed(&[10u8; 32]);
		let relayer_account = AccountId32::new(relayer_pair.public().0);
		setup_relayer_and_asset(&relayer_account);

		MessagingRelayerIncentives::set_mint_per_byte(RuntimeOrigin::root(), 1).unwrap();

		// Replace the signature bytes with garbage — the pallet must
		// refuse to mint when it can't recover a relayer.
		let mut msg = signed_request(&relayer_pair, vec![0u8; 50]);
		if let Message::Request(ref mut r) = msg.message {
			r.signer = vec![0u8; 64];
		}
		MessagingRelayerIncentives::on_executed(vec![msg], vec![]).unwrap();

		assert_eq!(relayer_balance(&relayer_account), 0);
	});
}
