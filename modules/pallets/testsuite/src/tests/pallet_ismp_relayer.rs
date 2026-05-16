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
	self as pallet_ismp_relayer, message,
	withdrawal::{Key, Signature, WithdrawalInputData, WithdrawalProof},
	OutboundConsensusDeliveryClaim,
};
use sp_core::{keccak_256, Pair, H256, U256};
use sp_trie::LayoutV0;
use std::time::Duration;
use substrate_state_machine::{HashAlgorithm, StateMachineProof, SubstrateStateProof};
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
				keys.push(Key::Request(*request));
			}
		}

		let source_keys_proof =
			source_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();
		let dest_keys_proof = dest_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();

		let source_state_proof = SubstrateStateProof::OverlayProof(StateMachineProof {
			hasher: HashAlgorithm::Keccak,
			storage_proof: source_keys_proof,
		});

		let dest_state_proof = SubstrateStateProof::OverlayProof(StateMachineProof {
			hasher: HashAlgorithm::Keccak,
			storage_proof: dest_keys_proof,
		});

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

		let signature = pair.sign(&keccak_256(beneficiary_address.as_bytes())).to_vec();

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
			keys.push(Key::Request(*request));
		}

		let source_keys_proof =
			source_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();
		let dest_keys_proof = dest_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();

		let source_state_proof = SubstrateStateProof::OverlayProof(StateMachineProof {
			hasher: HashAlgorithm::Keccak,
			storage_proof: source_keys_proof,
		});
		let dest_state_proof = SubstrateStateProof::OverlayProof(StateMachineProof {
			hasher: HashAlgorithm::Keccak,
			storage_proof: dest_keys_proof,
		});

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
			commitments: vec![Key::Request(request), Key::Request(request)],
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
				keys.push(Key::Request(*request));
			}
		}

		let source_keys_proof =
			source_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();
		let dest_keys_proof = dest_recorder.drain().into_iter().map(|f| f.data).collect::<Vec<_>>();

		let source_state_proof = SubstrateStateProof::OverlayProof(StateMachineProof {
			hasher: HashAlgorithm::Keccak,
			storage_proof: source_keys_proof,
		});

		let dest_state_proof = SubstrateStateProof::OverlayProof(StateMachineProof {
			hasher: HashAlgorithm::Keccak,
			storage_proof: dest_keys_proof,
		});

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

		let signature = pair.sign_prehashed(&keccak_256(beneficiary_address.as_bytes())).to_vec();

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
				Signature::Evm { address: eth_address, signature },
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
// the destination's `HandlerV2._epochs[set_id]` slot.
mod outbound_consensus_delivery {
	use super::*;
	use crate::runtime::Balances;
	use crypto_utils::verification::Signature;
	use pallet_ismp_host_executive::HostParams;
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
	const HANDLER: [u8; 20] = [0xCAu8; 20];

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

	/// Register an EVM HostParams entry with `HANDLER` as the HandlerV2
	/// address. Other fields default — only `handler` is consulted by the
	/// pallet under test.
	fn register_evm_host() {
		let mut params = EvmHostParam::default();
		params.handler = sp_core::H160::from(HANDLER);
		HostParams::<Test>::insert(DEST, HostParam::EvmHostParam(params));
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
	fn missing_host_params_is_rejected() {
		// No HostParams entry for DEST → the lookup fails before any proof
		// verification runs.
		new_test_ext().execute_with(|| {
			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					placeholder_claim(),
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundHostParamsNotKnown.into());
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

			let decoded = pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(&raw)
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
				pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(&raw).is_none(),
			);
		}

		/// RLP of an empty byte string is `0x80`. Logically "no value
		/// stored" — the trie shouldn't even have an entry for an
		/// unset slot, but if it does we treat it the same as
		/// `proof_results.get == None` and return None.
		#[test]
		fn rejects_rlp_empty_string() {
			assert!(
				pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(&[0x80,]).is_none()
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
				pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(&raw).is_none(),
				"raw 32-byte word must not decode — that was the broken legacy shape",
			);
		}

		/// Garbage bytes that aren't a valid RLP byte string at all.
		#[test]
		fn rejects_garbage() {
			assert!(pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(&[
				0xff, 0xff, 0xff,
			])
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
			assert!(pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(&raw).is_none());
		}

		/// Anything longer than 20 bytes (after stripping the RLP
		/// prefix) cannot be an address. Reject rather than truncate.
		#[test]
		fn rejects_oversized_string() {
			let big = vec![0xab; 32];
			let mut raw = vec![0x80 + big.len() as u8];
			raw.extend_from_slice(&big);
			assert!(pallet_ismp_relayer::Pallet::<Test>::decode_epochs_slot_address(&raw).is_none());
		}
	}

	#[test]
	fn placeholder_proof_reaches_verification_stage() {
		// HostParams registered, claim is otherwise valid pre-verification.
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
