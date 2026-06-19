// Copyright (c) 2025 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg(test)]
use polkadot_sdk::*;

use codec::Encode;
use ethereum_triedb::{keccak::KeccakHasher, MemoryDB};
use frame_support::crypto::ecdsa::ECDSAExt;
use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	host::{IsmpHost, StateMachine},
	messaging::{hash_request, Proof},
	router::{PostRequest, Request},
};
use pallet_ismp::{
	child_trie::{RequestCommitments, RequestReceipts},
	dispatcher::FeeMetadata,
};
use pallet_ismp_host_executive::{EvmHostParam, HostParam};
use pallet_ismp_relayer::{
	self as pallet_ismp_relayer, beneficiary_message, message,
	withdrawal::{Signature, WithdrawalInputData, WithdrawalProof},
	OutboundConsensusDeliveryClaim, OutboundRequestDeliveryClaim,
};
use sp_core::{Pair, H256, U256};
use sp_trie::LayoutV0;
use std::time::Duration;
use substrate_state_machine::{HashAlgorithm, StateMachineProof};
use trie_db::{Recorder, Trie, TrieDBBuilder, TrieDBMutBuilder, TrieMut};

use crate::runtime::{
	new_test_ext, set_timestamp, Ismp, RuntimeOrigin, Test, MOCK_CONSENSUS_CLIENT_ID,
	MOCK_CONSENSUS_STATE_ID,
};
use pallet_ismp::{dispatcher::RequestMetadata, offchain::LeafIndexAndPos};

#[test]
fn test_accumulate_fees() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		set_timestamp::<Test>(10_000_000_000);
		let requests = (0u64..10)
			.into_iter()
			.map(|nonce| {
				let post = PostRequest {
					source: StateMachine::Kusama(2000),
					dest: StateMachine::Kusama(2001),
					nonce,
					from: vec![],
					to: vec![],
					timeout_timestamp: 0,
					body: vec![],
				};
				hash_request::<Ismp>(&Request::Post(post))
			})
			.collect::<Vec<_>>();

		let pair = sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
		let public_key = pair.public().0.to_vec();

		let mut source_root = H256::default();

		let mut source_db = MemoryDB::<KeccakHasher>::default();
		let mut source_trie =
			TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut source_db, &mut source_root)
				.build();
		let mut dest_root = H256::default();

		let mut dest_db = MemoryDB::<KeccakHasher>::default();
		let mut dest_trie =
			TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut dest_db, &mut dest_root).build();

		// Insert requests
		for request in &requests {
			let request_commitment_key = RequestCommitments::<Test>::storage_key(*request);
			let request_receipt_key = RequestReceipts::<Test>::storage_key(*request);
			let fee_metadata = FeeMetadata::<Test> { payer: [0; 32].into(), fee: 1000u128.into() };
			let leaf_meta = RequestMetadata {
				offchain: LeafIndexAndPos { leaf_index: 0, pos: 0 },
				fee: fee_metadata,
				claimed: false,
			};
			RequestCommitments::<Test>::insert(*request, leaf_meta.clone());
			source_trie.insert(&request_commitment_key, &leaf_meta.encode()).unwrap();
			dest_trie.insert(&request_receipt_key, &public_key.encode()).unwrap();
		}

		drop(source_trie);
		drop(dest_trie);

		let mut source_recorder = Recorder::<LayoutV0<KeccakHasher>>::default();
		let mut dest_recorder = Recorder::<LayoutV0<KeccakHasher>>::default();
		let source_trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&source_db, &source_root)
			.with_recorder(&mut source_recorder)
			.build();

		let dest_trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&dest_db, &dest_root)
			.with_recorder(&mut dest_recorder)
			.build();

		let mut keys = vec![];

		for (index, request) in requests.iter().enumerate() {
			if index % 2 == 0 {
				let request_commitment_key = RequestCommitments::<Test>::storage_key(*request);
				let request_receipt_key = RequestReceipts::<Test>::storage_key(*request);
				source_trie.get(&request_commitment_key).unwrap();
				dest_trie.get(&request_receipt_key).unwrap();
				keys.push(*request);
			}
		}

		let source_keys_proof =
			source_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();
		let dest_keys_proof = dest_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();

		let source_state_proof =
			StateMachineProof { hasher: HashAlgorithm::Keccak, storage_proof: source_keys_proof };

		let dest_state_proof =
			StateMachineProof { hasher: HashAlgorithm::Keccak, storage_proof: dest_keys_proof };

		let host = Ismp::default();
		host.store_state_machine_commitment(
			StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Kusama(2000),
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: 1,
			},
			StateCommitment {
				timestamp: 100,
				overlay_root: Some(source_root),
				state_root: Default::default(),
			},
		)
		.unwrap();

		host.store_state_machine_commitment(
			StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Kusama(2001),
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: 1,
			},
			StateCommitment {
				timestamp: 100,
				overlay_root: Some(dest_root),
				state_root: Default::default(),
			},
		)
		.unwrap();

		host.store_state_machine_update_time(
			StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Kusama(2000),
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: 1,
			},
			Duration::from_secs(100),
		)
		.unwrap();

		host.store_state_machine_update_time(
			StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Kusama(2001),
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: 1,
			},
			Duration::from_secs(100),
		)
		.unwrap();
		host.store_consensus_state(MOCK_CONSENSUS_STATE_ID, Default::default()).unwrap();

		host.store_consensus_state_id(MOCK_CONSENSUS_STATE_ID, MOCK_CONSENSUS_CLIENT_ID)
			.unwrap();

		host.store_unbonding_period(MOCK_CONSENSUS_STATE_ID, 10_000_000_000).unwrap();

		host.store_challenge_period(
			StateMachineId {
				state_id: StateMachine::Kusama(2001),
				consensus_state_id: MOCK_CONSENSUS_STATE_ID,
			},
			0,
		)
		.unwrap();

		host.store_challenge_period(
			StateMachineId {
				state_id: StateMachine::Kusama(2000),
				consensus_state_id: MOCK_CONSENSUS_STATE_ID,
			},
			0,
		)
		.unwrap();

		let beneficiary_address = H256::random();

		let signature = pair
			.sign(&beneficiary_message(
				0,
				StateMachine::Kusama(2000),
				beneficiary_address.as_bytes(),
			))
			.to_vec();

		let withdrawal_proof = WithdrawalProof {
			commitments: keys,
			source_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Kusama(2000),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 1,
				},
				proof: source_state_proof.encode(),
			},
			dest_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Kusama(2001),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 1,
				},
				proof: dest_state_proof.encode(),
			},
			beneficiary_details: Some((
				beneficiary_address.0.to_vec().clone(),
				Signature::Sr25519 { public_key, signature },
			)),
		};

		pallet_ismp_relayer::Pallet::<Test>::accumulate_fees(
			RuntimeOrigin::none(),
			withdrawal_proof,
		)
		.unwrap();

		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(
				StateMachine::Kusama(2000),
				beneficiary_address.0.to_vec()
			),
			U256::from(5000u128)
		);
	})
}

/// `accumulate` only stores one delivery address per batch, so a
/// batch whose receipts resolve to more than one address must be
/// rejected before any fee credit or claimed-flag mutation. This
/// matches the official relayer's behaviour and keeps the
/// per-batch fee accounting unambiguous.
#[test]
fn test_accumulate_fees_rejects_mixed_delivery_addresses() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		set_timestamp::<Test>(10_000_000_000);

		// Two request commitments with receipts from two distinct relayer
		// addresses, chosen so they sort to different positions in the
		// fee-accumulator's `BTreeMap`.
		let requests = (0u64..2)
			.into_iter()
			.map(|nonce| {
				let post = PostRequest {
					source: StateMachine::Kusama(2000),
					dest: StateMachine::Kusama(2001),
					nonce,
					from: vec![],
					to: vec![],
					timeout_timestamp: 0,
					body: vec![],
				};
				hash_request::<Ismp>(&Request::Post(post))
			})
			.collect::<Vec<_>>();

		let relayer_a: Vec<u8> = vec![0x11; 32];
		let relayer_b: Vec<u8> = vec![0xff; 32];

		let mut source_root = H256::default();
		let mut source_db = MemoryDB::<KeccakHasher>::default();
		let mut source_trie =
			TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut source_db, &mut source_root)
				.build();
		let mut dest_root = H256::default();
		let mut dest_db = MemoryDB::<KeccakHasher>::default();
		let mut dest_trie =
			TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut dest_db, &mut dest_root).build();

		// Two commitments, one delivery receipt per relayer.
		for (index, request) in requests.iter().enumerate() {
			let request_commitment_key = RequestCommitments::<Test>::storage_key(*request);
			let request_receipt_key = RequestReceipts::<Test>::storage_key(*request);
			let fee_metadata = FeeMetadata::<Test> { payer: [0; 32].into(), fee: 1000u128.into() };
			let leaf_meta = RequestMetadata {
				offchain: LeafIndexAndPos { leaf_index: 0, pos: 0 },
				fee: fee_metadata,
				claimed: false,
			};
			RequestCommitments::<Test>::insert(*request, leaf_meta.clone());
			source_trie.insert(&request_commitment_key, &leaf_meta.encode()).unwrap();

			let relayer = if index == 0 { &relayer_a } else { &relayer_b };
			dest_trie.insert(&request_receipt_key, &relayer.encode()).unwrap();
		}
		drop(source_trie);
		drop(dest_trie);

		let mut source_recorder = Recorder::<LayoutV0<KeccakHasher>>::default();
		let mut dest_recorder = Recorder::<LayoutV0<KeccakHasher>>::default();
		let source_trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&source_db, &source_root)
			.with_recorder(&mut source_recorder)
			.build();
		let dest_trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&dest_db, &dest_root)
			.with_recorder(&mut dest_recorder)
			.build();

		let mut keys = vec![];
		for request in &requests {
			let request_commitment_key = RequestCommitments::<Test>::storage_key(*request);
			let request_receipt_key = RequestReceipts::<Test>::storage_key(*request);
			source_trie.get(&request_commitment_key).unwrap();
			dest_trie.get(&request_receipt_key).unwrap();
			keys.push(*request);
		}

		let source_keys_proof =
			source_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();
		let dest_keys_proof = dest_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();

		let source_state_proof =
			StateMachineProof { hasher: HashAlgorithm::Keccak, storage_proof: source_keys_proof };
		let dest_state_proof =
			StateMachineProof { hasher: HashAlgorithm::Keccak, storage_proof: dest_keys_proof };

		let host = Ismp::default();
		host.store_state_machine_commitment(
			StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Kusama(2000),
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: 1,
			},
			StateCommitment {
				timestamp: 100,
				overlay_root: Some(source_root),
				state_root: Default::default(),
			},
		)
		.unwrap();
		host.store_state_machine_commitment(
			StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Kusama(2001),
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: 1,
			},
			StateCommitment {
				timestamp: 100,
				overlay_root: Some(dest_root),
				state_root: Default::default(),
			},
		)
		.unwrap();
		host.store_state_machine_update_time(
			StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Kusama(2000),
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: 1,
			},
			Duration::from_secs(100),
		)
		.unwrap();
		host.store_state_machine_update_time(
			StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Kusama(2001),
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: 1,
			},
			Duration::from_secs(100),
		)
		.unwrap();
		host.store_consensus_state(MOCK_CONSENSUS_STATE_ID, Default::default()).unwrap();
		host.store_consensus_state_id(MOCK_CONSENSUS_STATE_ID, MOCK_CONSENSUS_CLIENT_ID)
			.unwrap();
		host.store_unbonding_period(MOCK_CONSENSUS_STATE_ID, 10_000_000_000).unwrap();
		host.store_challenge_period(
			StateMachineId {
				state_id: StateMachine::Kusama(2001),
				consensus_state_id: MOCK_CONSENSUS_STATE_ID,
			},
			0,
		)
		.unwrap();
		host.store_challenge_period(
			StateMachineId {
				state_id: StateMachine::Kusama(2000),
				consensus_state_id: MOCK_CONSENSUS_STATE_ID,
			},
			0,
		)
		.unwrap();

		let withdrawal_proof = WithdrawalProof {
			commitments: keys,
			source_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Kusama(2000),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 1,
				},
				proof: source_state_proof.encode(),
			},
			dest_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Kusama(2001),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 1,
				},
				proof: dest_state_proof.encode(),
			},
			beneficiary_details: None,
		};

		let err = pallet_ismp_relayer::Pallet::<Test>::accumulate_fees(
			RuntimeOrigin::none(),
			withdrawal_proof,
		)
		.expect_err("mixed-delivery-address batch must be rejected");

		assert_eq!(err, pallet_ismp_relayer::Error::<Test>::MixedDeliveryAddressesInBatch.into(),);

		// Neither delivery address should have been credited.
		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(StateMachine::Kusama(2000), &relayer_a),
			U256::zero(),
		);
		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(StateMachine::Kusama(2000), &relayer_b),
			U256::zero(),
		);

		// And neither commitment should have been marked claimed.
		assert!(!RequestCommitments::<Test>::get(requests[0]).unwrap().claimed);
		assert!(!RequestCommitments::<Test>::get(requests[1]).unwrap().claimed);
	})
}

/// `accumulate_fees` is unsigned, so anyone can submit a `WithdrawalProof`.
/// A batch padded with identical commitments must be rejected outright
/// before any proof verification or fee credit, so an attacker cannot
/// double-claim a single delivery.
#[test]
fn test_accumulate_fees_rejects_duplicate_commitments() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let request = H256::repeat_byte(0xab);
		let withdrawal_proof = WithdrawalProof {
			commitments: vec![request, request],
			source_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Kusama(2000),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 1,
				},
				proof: vec![],
			},
			dest_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Kusama(2001),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 1,
				},
				proof: vec![],
			},
			beneficiary_details: None,
		};

		let err = pallet_ismp_relayer::Pallet::<Test>::accumulate_fees(
			RuntimeOrigin::none(),
			withdrawal_proof,
		)
		.expect_err("duplicate-commitment batch must be rejected");

		assert_eq!(err, pallet_ismp_relayer::Error::<Test>::DuplicateCommitment.into());
	})
}

#[test]
fn test_accumulate_fees_evm_signatures() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		set_timestamp::<Test>(10_000_000_000);
		let requests = (0u64..10)
			.into_iter()
			.map(|nonce| {
				let post = PostRequest {
					source: StateMachine::Kusama(2000),
					dest: StateMachine::Kusama(2001),
					nonce,
					from: vec![],
					to: vec![],
					timeout_timestamp: 0,
					body: vec![],
				};
				hash_request::<Ismp>(&Request::Post(post))
			})
			.collect::<Vec<_>>();

		let pair = sp_core::ecdsa::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
		let eth_address = pair.public().to_eth_address().unwrap().to_vec();

		let mut source_root = H256::default();

		let mut source_db = MemoryDB::<KeccakHasher>::default();
		let mut source_trie =
			TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut source_db, &mut source_root)
				.build();
		let mut dest_root = H256::default();

		let mut dest_db = MemoryDB::<KeccakHasher>::default();
		let mut dest_trie =
			TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut dest_db, &mut dest_root).build();

		// Insert requests
		for request in &requests {
			let request_commitment_key = RequestCommitments::<Test>::storage_key(*request);
			let request_receipt_key = RequestReceipts::<Test>::storage_key(*request);
			let fee_metadata = FeeMetadata::<Test> { payer: [0; 32].into(), fee: 1000u128.into() };
			let leaf_meta = RequestMetadata {
				offchain: LeafIndexAndPos { leaf_index: 0, pos: 0 },
				fee: fee_metadata,
				claimed: false,
			};
			RequestCommitments::<Test>::insert(*request, leaf_meta.clone());
			source_trie.insert(&request_commitment_key, &leaf_meta.encode()).unwrap();
			dest_trie.insert(&request_receipt_key, &eth_address.encode()).unwrap();
		}

		drop(source_trie);
		drop(dest_trie);

		let mut source_recorder = Recorder::<LayoutV0<KeccakHasher>>::default();
		let mut dest_recorder = Recorder::<LayoutV0<KeccakHasher>>::default();
		let source_trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&source_db, &source_root)
			.with_recorder(&mut source_recorder)
			.build();

		let dest_trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&dest_db, &dest_root)
			.with_recorder(&mut dest_recorder)
			.build();

		let mut keys = vec![];

		for (index, request) in requests.iter().enumerate() {
			if index % 2 == 0 {
				let request_commitment_key = RequestCommitments::<Test>::storage_key(*request);
				let request_receipt_key = RequestReceipts::<Test>::storage_key(*request);
				source_trie.get(&request_commitment_key).unwrap();
				dest_trie.get(&request_receipt_key).unwrap();
				keys.push(*request);
			}
		}

		let source_keys_proof =
			source_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();
		let dest_keys_proof = dest_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();

		let source_state_proof =
			StateMachineProof { hasher: HashAlgorithm::Keccak, storage_proof: source_keys_proof };

		let dest_state_proof =
			StateMachineProof { hasher: HashAlgorithm::Keccak, storage_proof: dest_keys_proof };

		let host = Ismp::default();
		host.store_state_machine_commitment(
			StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Kusama(2000),
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: 1,
			},
			StateCommitment {
				timestamp: 100,
				overlay_root: Some(source_root),
				state_root: Default::default(),
			},
		)
		.unwrap();

		host.store_state_machine_commitment(
			StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Kusama(2001),
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: 1,
			},
			StateCommitment {
				timestamp: 100,
				overlay_root: Some(dest_root),
				state_root: Default::default(),
			},
		)
		.unwrap();

		host.store_state_machine_update_time(
			StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Kusama(2000),
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: 1,
			},
			Duration::from_secs(100),
		)
		.unwrap();

		host.store_state_machine_update_time(
			StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Kusama(2001),
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: 1,
			},
			Duration::from_secs(100),
		)
		.unwrap();
		host.store_consensus_state(MOCK_CONSENSUS_STATE_ID, Default::default()).unwrap();

		host.store_consensus_state_id(MOCK_CONSENSUS_STATE_ID, MOCK_CONSENSUS_CLIENT_ID)
			.unwrap();

		host.store_unbonding_period(MOCK_CONSENSUS_STATE_ID, 10_000_000_000).unwrap();

		host.store_challenge_period(
			StateMachineId {
				state_id: StateMachine::Kusama(2001),
				consensus_state_id: MOCK_CONSENSUS_STATE_ID,
			},
			0,
		)
		.unwrap();

		host.store_challenge_period(
			StateMachineId {
				state_id: StateMachine::Kusama(2000),
				consensus_state_id: MOCK_CONSENSUS_STATE_ID,
			},
			0,
		)
		.unwrap();

		let beneficiary_address = H256::random();

		let signature = pair
			.sign_prehashed(&beneficiary_message(
				0,
				StateMachine::Kusama(2000),
				beneficiary_address.as_bytes(),
			))
			.to_vec();

		let withdrawal_proof = WithdrawalProof {
			commitments: keys,
			source_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Kusama(2000),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 1,
				},
				proof: source_state_proof.encode(),
			},
			dest_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Kusama(2001),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 1,
				},
				proof: dest_state_proof.encode(),
			},
			beneficiary_details: Some((
				beneficiary_address.0.to_vec().clone(),
				Signature::Evm { address: eth_address.clone(), signature },
			)),
		};

		pallet_ismp_relayer::Pallet::<Test>::accumulate_fees(
			RuntimeOrigin::none(),
			withdrawal_proof,
		)
		.unwrap();

		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(
				StateMachine::Kusama(2000),
				beneficiary_address.0.to_vec()
			),
			U256::from(5000u128)
		);
		assert_eq!(
			pallet_ismp_relayer::Nonce::<Test>::get(eth_address, StateMachine::Kusama(2000)),
			1,
			"a successful redirect must consume the signer's nonce"
		);
	})
}

/// A redirect signature signed for one source chain must not satisfy verification
/// when reused on another. The state machine is folded into the signed payload
/// so the recovered address only matches the delivery address on the chain the
/// signature was issued for.
#[test]
fn test_accumulate_fees_rejects_replay_from_other_chain() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		set_timestamp::<Test>(10_000_000_000);
		let requests = (0u64..2)
			.into_iter()
			.map(|nonce| {
				let post = PostRequest {
					source: StateMachine::Kusama(2000),
					dest: StateMachine::Kusama(2001),
					nonce,
					from: vec![],
					to: vec![],
					timeout_timestamp: 0,
					body: vec![],
				};
				hash_request::<Ismp>(&Request::Post(post))
			})
			.collect::<Vec<_>>();

		let pair = sp_core::ecdsa::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
		let eth_address = pair.public().to_eth_address().unwrap().to_vec();

		let mut source_root = H256::default();
		let mut source_db = MemoryDB::<KeccakHasher>::default();
		let mut source_trie =
			TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut source_db, &mut source_root)
				.build();
		let mut dest_root = H256::default();
		let mut dest_db = MemoryDB::<KeccakHasher>::default();
		let mut dest_trie =
			TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut dest_db, &mut dest_root).build();

		for request in &requests {
			let request_commitment_key = RequestCommitments::<Test>::storage_key(*request);
			let request_receipt_key = RequestReceipts::<Test>::storage_key(*request);
			let fee_metadata = FeeMetadata::<Test> { payer: [0; 32].into(), fee: 1000u128.into() };
			let leaf_meta = RequestMetadata {
				offchain: LeafIndexAndPos { leaf_index: 0, pos: 0 },
				fee: fee_metadata,
				claimed: false,
			};
			RequestCommitments::<Test>::insert(*request, leaf_meta.clone());
			source_trie.insert(&request_commitment_key, &leaf_meta.encode()).unwrap();
			dest_trie.insert(&request_receipt_key, &eth_address.encode()).unwrap();
		}
		drop(source_trie);
		drop(dest_trie);

		let mut source_recorder = Recorder::<LayoutV0<KeccakHasher>>::default();
		let mut dest_recorder = Recorder::<LayoutV0<KeccakHasher>>::default();
		let source_trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&source_db, &source_root)
			.with_recorder(&mut source_recorder)
			.build();
		let dest_trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&dest_db, &dest_root)
			.with_recorder(&mut dest_recorder)
			.build();

		let mut keys = vec![];
		for request in requests.iter() {
			let request_commitment_key = RequestCommitments::<Test>::storage_key(*request);
			let request_receipt_key = RequestReceipts::<Test>::storage_key(*request);
			source_trie.get(&request_commitment_key).unwrap();
			dest_trie.get(&request_receipt_key).unwrap();
			keys.push(*request);
		}

		let source_state_proof = StateMachineProof {
			hasher: HashAlgorithm::Keccak,
			storage_proof: source_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>(),
		};
		let dest_state_proof = StateMachineProof {
			hasher: HashAlgorithm::Keccak,
			storage_proof: dest_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>(),
		};

		let host = Ismp::default();
		for chain in [StateMachine::Kusama(2000), StateMachine::Kusama(2001)] {
			host.store_state_machine_commitment(
				StateMachineHeight {
					id: StateMachineId {
						state_id: chain,
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 1,
				},
				StateCommitment {
					timestamp: 100,
					overlay_root: Some(if chain == StateMachine::Kusama(2000) {
						source_root
					} else {
						dest_root
					}),
					state_root: Default::default(),
				},
			)
			.unwrap();
			host.store_state_machine_update_time(
				StateMachineHeight {
					id: StateMachineId {
						state_id: chain,
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 1,
				},
				Duration::from_secs(100),
			)
			.unwrap();
			host.store_challenge_period(
				StateMachineId { state_id: chain, consensus_state_id: MOCK_CONSENSUS_STATE_ID },
				0,
			)
			.unwrap();
		}
		host.store_consensus_state(MOCK_CONSENSUS_STATE_ID, Default::default()).unwrap();
		host.store_consensus_state_id(MOCK_CONSENSUS_STATE_ID, MOCK_CONSENSUS_CLIENT_ID)
			.unwrap();
		host.store_unbonding_period(MOCK_CONSENSUS_STATE_ID, 10_000_000_000).unwrap();

		let beneficiary_address = H256::random();

		// A signature the relayer would have produced for a different source chain.
		// The byte payload is reused as if captured from that chain and replayed here.
		let foreign_chain = StateMachine::Kusama(7777);
		let signature = pair
			.sign_prehashed(&beneficiary_message(0, foreign_chain, beneficiary_address.as_bytes()))
			.to_vec();

		let withdrawal_proof = WithdrawalProof {
			commitments: keys,
			source_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Kusama(2000),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 1,
				},
				proof: source_state_proof.encode(),
			},
			dest_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: StateMachine::Kusama(2001),
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 1,
				},
				proof: dest_state_proof.encode(),
			},
			beneficiary_details: Some((
				beneficiary_address.0.to_vec(),
				Signature::Evm { address: eth_address, signature },
			)),
		};

		let result = pallet_ismp_relayer::Pallet::<Test>::accumulate_fees(
			RuntimeOrigin::none(),
			withdrawal_proof,
		);

		assert!(result.is_err(), "replayed signature from a different chain must be rejected");
		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(
				StateMachine::Kusama(2000),
				beneficiary_address.0.to_vec()
			),
			U256::zero()
		);
	})
}

#[test]
fn test_withdrawal_fees() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let pair = sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
		let public_key = pair.public().0.to_vec();
		pallet_ismp_relayer::Fees::<Test>::insert(
			StateMachine::Kusama(2000),
			public_key.clone(),
			U256::from(250_000_000_000_000_000_000u128),
		);
		let message = message(0, StateMachine::Kusama(2000), None);
		let signature = pair.sign(&message).0.to_vec();

		let withdrawal_input = WithdrawalInputData {
			signature: Signature::Sr25519 { public_key: public_key.clone(), signature },
			beneficiary: None,
			dest_chain: StateMachine::Kusama(2000),
		};

		pallet_ismp_relayer::Pallet::<Test>::withdraw_fees(
			RuntimeOrigin::none(),
			withdrawal_input.clone(),
		)
		.unwrap();
		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(StateMachine::Kusama(2000), public_key.clone()),
			U256::zero()
		);

		assert_eq!(
			pallet_ismp_relayer::Nonce::<Test>::get(public_key, StateMachine::Kusama(2000)),
			1
		);

		assert!(pallet_ismp_relayer::Pallet::<Test>::withdraw_fees(
			RuntimeOrigin::none(),
			withdrawal_input.clone()
		)
		.is_err());
	})
}

#[test]
fn test_withdrawal_fees_evm() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let pair = sp_core::ecdsa::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
		let address = pair.public().to_eth_address().unwrap();
		let evm_host_params = EvmHostParam::default();
		pallet_ismp_host_executive::HostParams::<Test>::insert(
			StateMachine::Evm(84532),
			HostParam::EvmHostParam(evm_host_params),
		);
		pallet_ismp_relayer::Fees::<Test>::insert(
			StateMachine::Evm(84532),
			address.to_vec(),
			U256::from(250_000_000_000_000_000_000u128),
		);
		let message = message(0, StateMachine::Evm(84532), None);
		let signature = pair.sign_prehashed(&message).0.to_vec();

		let withdrawal_input = WithdrawalInputData {
			signature: Signature::Evm { address: address.to_vec(), signature },
			dest_chain: StateMachine::Evm(84532),
			beneficiary: None,
		};

		pallet_ismp_relayer::Pallet::<Test>::withdraw_fees(
			RuntimeOrigin::none(),
			withdrawal_input.clone(),
		)
		.unwrap();
		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(StateMachine::Evm(84532), address.to_vec()),
			U256::zero()
		);

		assert_eq!(
			pallet_ismp_relayer::Nonce::<Test>::get(address.to_vec(), StateMachine::Evm(84532)),
			1
		);

		assert!(pallet_ismp_relayer::Pallet::<Test>::withdraw_fees(
			RuntimeOrigin::none(),
			withdrawal_input.clone()
		)
		.is_err());
	})
}

// Outbound consensus delivery reward tests.
//
// The pallet attributes a mandatory rotation to the EVM relayer recorded in
// the destination's `EvmHost._epochs[set_id]` slot.
mod outbound_consensus_delivery {
	use super::*;
	use crate::runtime::Balances;
	use crypto_utils::verification::Signature;
	use pallet_ismp_host_executive::EvmHosts;
	use pallet_ismp_relayer::{
		Error, OutboundConsensusDeliveryReward, OutboundConsensusRotationsClaimed,
	};
	use polkadot_sdk::{
		frame_support::traits::{
			fungible::Inspect, tokens::fungible::Mutate as FungibleMutate, Get,
		},
		sp_runtime::{traits::AccountIdConversion, AccountId32},
	};

	const DEST: StateMachine = StateMachine::Evm(11155111);
	const REWARD: u128 = 1_000_000_000_000;
	const SET_ID: u64 = 4;
	const HEIGHT: u64 = 100;
	const HOST: [u8; 20] = [0xCAu8; 20];

	/// Build a claim with a placeholder empty state proof. Most tests fire
	/// pre-verification errors, so the proof bytes don't need to be real.
	fn placeholder_claim() -> OutboundConsensusDeliveryClaim {
		OutboundConsensusDeliveryClaim {
			state_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: DEST,
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: HEIGHT,
				},
				proof: vec![],
			},
			set_id: SET_ID,
			payee: [0xAA; 32],
			signature: Signature::Evm { address: vec![0; 20], signature: vec![0; 65] },
		}
	}

	/// Register the destination's `EvmHost` address. The pallet under test
	/// reads this from `pallet-ismp-host-executive::EvmHosts`.
	fn register_evm_host() {
		EvmHosts::<Test>::insert(DEST, sp_core::H160::from(HOST));
	}

	#[test]
	fn already_claimed_is_rejected() {
		new_test_ext().execute_with(|| {
			OutboundConsensusRotationsClaimed::<Test>::insert(DEST, SET_ID, ());

			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					placeholder_claim(),
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundRotationAlreadyClaimed.into());
		})
	}

	#[test]
	fn missing_host_is_rejected() {
		// No `EvmHosts` entry for DEST → the lookup fails before any proof
		// verification runs.
		new_test_ext().execute_with(|| {
			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					placeholder_claim(),
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundHostNotKnown.into());
		})
	}

	mod decode_epochs_slot_address {
		//! Regression cases for `Pallet::decode_epochs_slot_address`,
		//! the RLP-aware decoder behind the `_epochs[set_id]` slot
		//! attribution. Earlier code did `<[u8; 32]>::try_from(raw)`,
		//! which rejected every populated slot the EVM state-trie
		//! returns — see
		//! `modules/pallets/testsuite/tests/verify_pending_claims.rs`
		//! for the live replay that uncovered the issue.
		use super::*;
		use alloy_primitives::Address as AlloyAddress;

		const RELAYER_ADDR: [u8; 20] = [
			0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
			0x0f, 0x10, 0x11, 0x12, 0x13, 0x14,
		];

		/// The 21-byte shape `EvmStateMachine::verify_state_proof`
		/// returned for every populated `_epochs[set_id]` slot in the
		/// testnet replay. RLP of a 20-byte string is `0x80 + 20`
		/// (`0x94`) followed by the bytes themselves.
		#[test]
		fn decodes_real_storage_slot_value() {
			let mut raw = vec![0x94];
			raw.extend_from_slice(&RELAYER_ADDR);
			assert_eq!(raw.len(), 21);

			let decoded = pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(ismp::host::StateMachine::Evm(1), &raw)
				.expect("21-byte RLP-encoded address should decode");
			assert_eq!(decoded, AlloyAddress::from_slice(&RELAYER_ADDR));
		}

		/// `alloy_rlp::Address::decode` requires exactly 20 payload
		/// bytes after the RLP header. The EVM strips leading zero
		/// bytes from values before RLP-encoding, so an address whose
		/// top byte is zero shows up as `<21` bytes total and decodes
		/// to an error. Relayer addresses are random 20-byte values
		/// generated at signer keygen, so the chance of a leading
		/// zero byte is ~1/256 and we accept this trade-off in
		/// exchange for a single-line decoder. This test pins the
		/// behaviour: if/when we change to a leading-zero-tolerant
		/// decoder, flip this to expect Some.
		#[test]
		fn rejects_address_with_leading_zero_stripped() {
			let mut addr_no_leading = [0u8; 20];
			addr_no_leading[1..].copy_from_slice(&RELAYER_ADDR[1..]); // top byte stays 0
			let stripped = &addr_no_leading[1..]; // 19 bytes (leading zero gone)
			let mut raw = vec![0x80 + stripped.len() as u8];
			raw.extend_from_slice(stripped);

			assert!(
				pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(ismp::host::StateMachine::Evm(1), &raw).is_none(),
			);
		}

		/// RLP of an empty byte string is `0x80`. Logically "no value
		/// stored" — the trie shouldn't even have an entry for an
		/// unset slot, but if it does we treat it the same as
		/// `proof_results.get == None` and return None.
		#[test]
		fn rejects_rlp_empty_string() {
			assert!(
				pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(ismp::host::StateMachine::Evm(1), &[0x80,]).is_none()
			);
		}

		/// The pre-fix code expected this raw 32-byte form. Real EVM
		/// tries never produce it (RLP-encoded strings of >55 bytes
		/// use a different prefix; raw bytes have no length tag at
		/// all), so the decoder must reject it.
		#[test]
		fn rejects_raw_32_byte_word() {
			let mut raw = [0u8; 32];
			raw[12..].copy_from_slice(&RELAYER_ADDR);
			// 32 bytes with leading 0x00 looks to alloy_rlp like a
			// 1-byte string `0x00` (which is invalid per RLP) or as
			// trailing garbage; either way we expect None.
			assert!(
				pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(ismp::host::StateMachine::Evm(1), &raw).is_none(),
				"raw 32-byte word must not decode — that was the broken legacy shape",
			);
		}

		/// Garbage bytes that aren't a valid RLP byte string at all.
		#[test]
		fn rejects_garbage() {
			assert!(pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(
				ismp::host::StateMachine::Evm(1),
				&[0xff, 0xff, 0xff],
			)
			.is_none());
		}

		/// Defence-in-depth: an explicitly RLP-encoded zero address is
		/// rejected the same way an unset slot is, so a malicious
		/// actor can't claim a reward by writing zeros. (In practice
		/// the EVM would never emit this — leading zeros are stripped
		/// — but the contract on chain could be compromised.)
		#[test]
		fn rejects_explicit_zero_address() {
			let mut raw = vec![0x94];
			raw.extend_from_slice(&[0u8; 20]);
			assert!(pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(ismp::host::StateMachine::Evm(1), &raw).is_none());
		}

		/// Anything longer than 20 bytes (after stripping the RLP
		/// prefix) cannot be an address. Reject rather than truncate.
		#[test]
		fn rejects_oversized_string() {
			let big = vec![0xab; 32];
			let mut raw = vec![0x80 + big.len() as u8];
			raw.extend_from_slice(&big);
			assert!(pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(ismp::host::StateMachine::Evm(1), &raw).is_none());
		}
	}

	#[test]
	fn placeholder_proof_reaches_verification_stage() {
		// EvmHost address registered, claim is otherwise valid pre-verification.
		// Empty proof → the verifier rejects; HB has no commitment for DEST
		// at HEIGHT either, so we get OutboundDestinationStateNotKnown.
		// This is the smoke test that confirms the pipeline reaches the
		// proof step (not blocked by an earlier ensure!).
		new_test_ext().execute_with(|| {
			register_evm_host();
			OutboundConsensusDeliveryReward::<Test>::insert(DEST, REWARD);

			let treasury: AccountId32 =
				<Test as pallet_ismp_relayer::Config>::TreasuryPalletId::get()
					.into_account_truncating();
			Balances::mint_into(&treasury, REWARD * 10).unwrap();

			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					placeholder_claim(),
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundDestinationStateNotKnown.into());
		})
	}
}

mod outbound_request_delivery {
	use super::*;
	use crate::runtime::Balances;
	use ismp::router::PostRequest;
	use pallet_ismp_relayer::{
		outbound_request_delivery_message, Error, OutboundRequestDeliveryReward,
		OutboundRequestsClaimed,
	};
	use polkadot_sdk::{
		frame_support::{
			traits::{fungible::Inspect, tokens::fungible::Mutate as FungibleMutate, Get},
			BoundedVec,
		},
		sp_runtime::{traits::AccountIdConversion, AccountId32},
	};

	const NEXUS: StateMachine = StateMachine::Kusama(100);
	const EVM_DEST: StateMachine = StateMachine::Evm(11155111);
	// Echo-backed EVM destination (see `EchoStateMachine`) for the request-claim pipeline test.
	const EVM_ECHO_DEST: StateMachine = StateMachine::Evm(11155112);
	const SUBSTRATE_DEST: StateMachine = StateMachine::Kusama(2001);
	const REWARD: u128 = 1_000_000_000_000;
	const HEIGHT: u64 = 100;
	const MODULE_ID: &[u8] = b"HOSTEXEC";

	fn post_request(dest: StateMachine, from: &[u8]) -> PostRequest {
		PostRequest {
			source: NEXUS,
			dest,
			nonce: 0,
			from: from.to_vec(),
			to: vec![0xBB; 20],
			timeout_timestamp: 0,
			body: vec![],
		}
	}

	fn commitment_of(req: &PostRequest) -> H256 {
		hash_request::<Ismp>(&ismp::router::Request::Post(req.clone()))
	}

	fn module_bound(id: &[u8]) -> BoundedVec<u8, pallet_ismp_relayer::ModuleIdBound> {
		BoundedVec::try_from(id.to_vec()).expect("module id within bound")
	}

	fn placeholder_claim_for(req: PostRequest) -> OutboundRequestDeliveryClaim {
		let dest = req.dest;
		OutboundRequestDeliveryClaim {
			request: req,
			state_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: dest,
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: HEIGHT,
				},
				proof: vec![],
			},
			payee: [0xAA; 32],
			signature: Signature::Evm { address: vec![0; 20], signature: vec![0; 65] },
		}
	}

	fn register_request(req: &PostRequest) -> H256 {
		let commitment = commitment_of(req);
		let fee = FeeMetadata::<Test> { payer: [0; 32].into(), fee: 0u128.into() };
		let metadata = RequestMetadata {
			offchain: LeafIndexAndPos { leaf_index: 0, pos: 0 },
			fee,
			claimed: false,
		};
		RequestCommitments::<Test>::insert(commitment, metadata);
		commitment
	}

	fn set_reward(_dest: StateMachine, module_id: &[u8], amount: u128) {
		OutboundRequestDeliveryReward::<Test>::insert(module_bound(module_id), amount);
	}

	fn fund_treasury(amount: u128) {
		let treasury: AccountId32 = <Test as pallet_ismp_relayer::Config>::TreasuryPalletId::get()
			.into_account_truncating();
		Balances::mint_into(&treasury, amount).unwrap();
	}

	#[test]
	fn source_not_hyperbridge_is_rejected() {
		new_test_ext().execute_with(|| {
			let mut req = post_request(EVM_DEST, MODULE_ID);
			req.source = StateMachine::Kusama(999);
			let err = pallet_ismp_relayer::Pallet::<Test>::claim_outbound_request_delivery_reward(
				RuntimeOrigin::none(),
				placeholder_claim_for(req),
			)
			.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundRequestSourceNotHyperbridge.into());
		})
	}

	#[test]
	fn unknown_commitment_is_rejected() {
		new_test_ext().execute_with(|| {
			let err = pallet_ismp_relayer::Pallet::<Test>::claim_outbound_request_delivery_reward(
				RuntimeOrigin::none(),
				placeholder_claim_for(post_request(EVM_DEST, MODULE_ID)),
			)
			.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundRequestNotKnown.into());
		})
	}

	#[test]
	fn already_claimed_is_rejected() {
		new_test_ext().execute_with(|| {
			let req = post_request(EVM_DEST, MODULE_ID);
			let commitment = register_request(&req);
			OutboundRequestsClaimed::<Test>::insert(commitment, ());

			let err = pallet_ismp_relayer::Pallet::<Test>::claim_outbound_request_delivery_reward(
				RuntimeOrigin::none(),
				placeholder_claim_for(req),
			)
			.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundRequestAlreadyClaimed.into());
		})
	}

	#[test]
	fn module_id_too_long_is_rejected() {
		new_test_ext().execute_with(|| {
			let long = vec![0u8; 65];
			let req = post_request(EVM_DEST, &long);
			register_request(&req);

			let err = pallet_ismp_relayer::Pallet::<Test>::claim_outbound_request_delivery_reward(
				RuntimeOrigin::none(),
				placeholder_claim_for(req),
			)
			.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundRequestModuleIdTooLong.into());
		})
	}

	#[test]
	fn allowlist_off_rejects_before_proof_verification() {
		// Reward not set for (dest, module_id). The check rejects before
		// any state-proof work happens, which is the whole point of the
		// allowlist ordering.
		new_test_ext().execute_with(|| {
			let req = post_request(EVM_DEST, MODULE_ID);
			register_request(&req);
			let err = pallet_ismp_relayer::Pallet::<Test>::claim_outbound_request_delivery_reward(
				RuntimeOrigin::none(),
				placeholder_claim_for(req),
			)
			.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundRequestNoRewardConfigured.into());
		})
	}

	#[test]
	fn placeholder_proof_reaches_verification_stage() {
		// All cheap checks pass (source, commitment, idempotency, reward,
		// destination type). State proof is empty so verification fails.
		new_test_ext().execute_with(|| {
			let req = post_request(EVM_DEST, MODULE_ID);
			register_request(&req);
			set_reward(EVM_DEST, MODULE_ID, REWARD);
			fund_treasury(REWARD * 10);

			let err = pallet_ismp_relayer::Pallet::<Test>::claim_outbound_request_delivery_reward(
				RuntimeOrigin::none(),
				placeholder_claim_for(req),
			)
			.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundDestinationStateNotKnown.into());
		})
	}

	#[test]
	fn proof_destination_must_match_request_dest() {
		// Defense-in-depth: even with a registered, unclaimed, allowlisted request, a
		// claim whose state proof is for a different destination than the request's
		// declared `dest` must be rejected before any proof verification runs. The
		// commitment is unique per `(request, dest)` so the wrong-destination proof
		// would normally just show an empty receipt slot, but the explicit equality
		// check catches the mismatch up front with a clearer error.
		new_test_ext().execute_with(|| {
			let req = post_request(EVM_DEST, MODULE_ID);
			register_request(&req);
			set_reward(EVM_DEST, MODULE_ID, REWARD);
			fund_treasury(REWARD * 10);

			// `req.dest = EVM_DEST` but the proof is for `SUBSTRATE_DEST`.
			let claim = OutboundRequestDeliveryClaim {
				request: req,
				state_proof: Proof {
					height: StateMachineHeight {
						id: StateMachineId {
							state_id: SUBSTRATE_DEST,
							consensus_state_id: MOCK_CONSENSUS_STATE_ID,
						},
						height: HEIGHT,
					},
					proof: vec![],
				},
				payee: [0xAA; 32],
				signature: Signature::Evm { address: vec![0; 20], signature: vec![0; 65] },
			};

			let err = pallet_ismp_relayer::Pallet::<Test>::claim_outbound_request_delivery_reward(
				RuntimeOrigin::none(),
				claim,
			)
			.unwrap_err();
			assert_eq!(err, Error::<Test>::MismatchedStateMachine.into());
		})
	}

	/// Builds a substrate destination trie with `RequestReceipts[commitment]`
	/// set to `relayer_value`, registers the resulting state commitment on the
	/// mock ISMP host, and returns the encoded state proof to embed in a claim.
	fn substrate_dest_proof(commitment: H256, relayer_value: &[u8]) -> Vec<u8> {
		let mut root = H256::default();
		let mut db = MemoryDB::<KeccakHasher>::default();
		{
			let mut trie =
				TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut db, &mut root).build();
			let receipt_key = RequestReceipts::<Test>::storage_key(commitment);
			trie.insert(&receipt_key, &relayer_value.encode()).unwrap();
		}

		let mut recorder = Recorder::<LayoutV0<KeccakHasher>>::default();
		{
			let trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&db, &root)
				.with_recorder(&mut recorder)
				.build();
			let receipt_key = RequestReceipts::<Test>::storage_key(commitment);
			trie.get(&receipt_key).unwrap();
		}
		let storage_proof = recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();

		let host = Ismp::default();
		host.store_state_machine_commitment(
			StateMachineHeight {
				id: StateMachineId {
					state_id: SUBSTRATE_DEST,
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: HEIGHT,
			},
			StateCommitment {
				timestamp: 100,
				overlay_root: Some(root),
				state_root: Default::default(),
			},
		)
		.unwrap();
		host.store_state_machine_update_time(
			StateMachineHeight {
				id: StateMachineId {
					state_id: SUBSTRATE_DEST,
					consensus_state_id: MOCK_CONSENSUS_STATE_ID,
				},
				height: HEIGHT,
			},
			Duration::from_secs(100),
		)
		.unwrap();
		host.store_consensus_state(MOCK_CONSENSUS_STATE_ID, Default::default()).unwrap();
		host.store_consensus_state_id(MOCK_CONSENSUS_STATE_ID, MOCK_CONSENSUS_CLIENT_ID)
			.unwrap();
		host.store_unbonding_period(MOCK_CONSENSUS_STATE_ID, 10_000_000_000).unwrap();
		host.store_challenge_period(
			StateMachineId {
				state_id: SUBSTRATE_DEST,
				consensus_state_id: MOCK_CONSENSUS_STATE_ID,
			},
			0,
		)
		.unwrap();

		StateMachineProof { hasher: HashAlgorithm::Keccak, storage_proof }.encode()
	}

	/// Set up the host's view of the echo-backed EVM destination and return the
	/// proof bytes, which `EchoStateMachine` hands back verbatim as the proven
	/// receipt value. The bytes are the relayer address rlp encoded, the shape an
	/// EVM `RequestReceipts` slot decodes to.
	fn evm_dest_proof(eth_addr: &[u8]) -> Vec<u8> {
		use alloy_primitives::Address;

		let host = Ismp::default();
		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: EVM_ECHO_DEST,
				consensus_state_id: MOCK_CONSENSUS_STATE_ID,
			},
			height: HEIGHT,
		};
		host.store_state_machine_commitment(
			height,
			StateCommitment { timestamp: 100, overlay_root: None, state_root: Default::default() },
		)
		.unwrap();
		host.store_state_machine_update_time(height, Duration::from_secs(100)).unwrap();
		host.store_consensus_state(MOCK_CONSENSUS_STATE_ID, Default::default()).unwrap();
		host.store_consensus_state_id(MOCK_CONSENSUS_STATE_ID, MOCK_CONSENSUS_CLIENT_ID)
			.unwrap();
		host.store_unbonding_period(MOCK_CONSENSUS_STATE_ID, 10_000_000_000).unwrap();
		host.store_challenge_period(
			StateMachineId { state_id: EVM_ECHO_DEST, consensus_state_id: MOCK_CONSENSUS_STATE_ID },
			0,
		)
		.unwrap();

		alloy_rlp::encode(Address::from_slice(eth_addr))
	}

	#[test]
	fn evm_destination_pays_relayer() {
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			let pair = sp_core::ecdsa::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
			let eth_address = pair.public().to_eth_address().unwrap().to_vec();
			let payee = [0xAA; 32];
			let payee_account: AccountId32 = payee.into();

			let req = post_request(EVM_ECHO_DEST, MODULE_ID);
			let commitment = register_request(&req);
			set_reward(EVM_ECHO_DEST, MODULE_ID, REWARD);
			fund_treasury(REWARD * 10);

			let proof = evm_dest_proof(&eth_address);
			let msg = outbound_request_delivery_message(commitment, EVM_ECHO_DEST, payee);
			let signature = pair.sign_prehashed(&msg).to_vec();

			let claim = OutboundRequestDeliveryClaim {
				request: req,
				state_proof: Proof {
					height: StateMachineHeight {
						id: StateMachineId {
							state_id: EVM_ECHO_DEST,
							consensus_state_id: MOCK_CONSENSUS_STATE_ID,
						},
						height: HEIGHT,
					},
					proof,
				},
				payee,
				signature: Signature::Evm { address: eth_address, signature },
			};

			let payee_before = Balances::balance(&payee_account);
			pallet_ismp_relayer::Pallet::<Test>::claim_outbound_request_delivery_reward(
				RuntimeOrigin::none(),
				claim,
			)
			.unwrap();

			assert_eq!(Balances::balance(&payee_account) - payee_before, REWARD);
			assert!(OutboundRequestsClaimed::<Test>::contains_key(commitment));
		})
	}

	#[test]
	fn substrate_destination_pays_relayer() {
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			let pair = sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
			let relayer_bytes = pair.public().0.to_vec();
			let payee = [0xAA; 32];
			let payee_account: AccountId32 = payee.into();

			let req = post_request(SUBSTRATE_DEST, MODULE_ID);
			let commitment = register_request(&req);
			set_reward(SUBSTRATE_DEST, MODULE_ID, REWARD);
			fund_treasury(REWARD * 10);

			let msg = outbound_request_delivery_message(commitment, SUBSTRATE_DEST, payee);
			let signature = pair.sign(&msg).0.to_vec();

			let claim = OutboundRequestDeliveryClaim {
				request: req,
				state_proof: Proof {
					height: StateMachineHeight {
						id: StateMachineId {
							state_id: SUBSTRATE_DEST,
							consensus_state_id: MOCK_CONSENSUS_STATE_ID,
						},
						height: HEIGHT,
					},
					proof: substrate_dest_proof(commitment, &relayer_bytes),
				},
				payee,
				signature: Signature::Sr25519 { public_key: relayer_bytes, signature },
			};

			let payee_before = Balances::balance(&payee_account);
			pallet_ismp_relayer::Pallet::<Test>::claim_outbound_request_delivery_reward(
				RuntimeOrigin::none(),
				claim,
			)
			.unwrap();

			assert_eq!(Balances::balance(&payee_account) - payee_before, REWARD);
			assert!(OutboundRequestsClaimed::<Test>::contains_key(commitment));
		})
	}

	#[test]
	fn signer_mismatch_is_rejected() {
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			let recorded =
				sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
			let other = sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
			let payee = [0xAA; 32];

			let req = post_request(SUBSTRATE_DEST, MODULE_ID);
			let commitment = register_request(&req);
			set_reward(SUBSTRATE_DEST, MODULE_ID, REWARD);
			fund_treasury(REWARD * 10);

			let msg = outbound_request_delivery_message(commitment, SUBSTRATE_DEST, payee);
			let other_sig = other.sign(&msg).0.to_vec();

			let claim = OutboundRequestDeliveryClaim {
				request: req,
				state_proof: Proof {
					height: StateMachineHeight {
						id: StateMachineId {
							state_id: SUBSTRATE_DEST,
							consensus_state_id: MOCK_CONSENSUS_STATE_ID,
						},
						height: HEIGHT,
					},
					proof: substrate_dest_proof(commitment, &recorded.public().0),
				},
				payee,
				signature: Signature::Sr25519 {
					public_key: other.public().0.to_vec(),
					signature: other_sig,
				},
			};

			let err = pallet_ismp_relayer::Pallet::<Test>::claim_outbound_request_delivery_reward(
				RuntimeOrigin::none(),
				claim,
			)
			.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundRequestSignerMismatch.into());
		})
	}

	mod decode_receipt_relayer {
		use super::*;

		#[test]
		fn decodes_evm_address() {
			new_test_ext().execute_with(|| {
				let address = alloy_primitives::Address::from([0x11u8; 20]);
				let raw = alloy_rlp::encode(&address);

				let decoded =
					pallet_ismp_relayer::Pallet::<Test>::decode_receipt_relayer(EVM_DEST, &raw)
						.unwrap();
				assert_eq!(decoded, address.0.to_vec());
			})
		}

		#[test]
		fn decodes_substrate_raw_bytes() {
			new_test_ext().execute_with(|| {
				let pubkey = [0x77u8; 32];
				let raw = pubkey.to_vec().encode();

				let decoded = pallet_ismp_relayer::Pallet::<Test>::decode_receipt_relayer(
					SUBSTRATE_DEST,
					&raw,
				)
				.unwrap();
				assert_eq!(decoded, pubkey.to_vec());
			})
		}

		#[test]
		fn decodes_substrate_signature_wrapper() {
			new_test_ext().execute_with(|| {
				let pair =
					sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
				let inner = Signature::Sr25519 {
					public_key: pair.public().0.to_vec(),
					signature: vec![0u8; 64],
				};
				let raw = inner.encode().encode();

				let decoded = pallet_ismp_relayer::Pallet::<Test>::decode_receipt_relayer(
					SUBSTRATE_DEST,
					&raw,
				)
				.unwrap();
				assert_eq!(decoded, pair.public().0.to_vec());
			})
		}

		#[test]
		fn unsupported_returns_error() {
			new_test_ext().execute_with(|| {
				let raw = vec![0u8; 21];
				assert!(pallet_ismp_relayer::Pallet::<Test>::decode_receipt_relayer(
					StateMachine::Tendermint(*b"cosm"),
					&raw,
				)
				.is_err());
			})
		}
	}
}
