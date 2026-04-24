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
	messaging::{hash_post_response, hash_request, Proof},
	router::{PostRequest, Request},
};
use pallet_ismp::{
	child_trie::{RequestCommitments, RequestReceipts, ResponseCommitments, ResponseReceipts},
	dispatcher::FeeMetadata,
	ResponseReceipt,
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

		let responses = (0u64..10)
			.into_iter()
			.map(|nonce| {
				let post = PostRequest {
					source: StateMachine::Kusama(2001),
					dest: StateMachine::Kusama(2000),
					nonce,
					from: vec![],
					to: vec![],
					timeout_timestamp: 0,
					body: vec![],
				};
				let response = ismp::router::PostResponse {
					post: post.clone(),
					response: vec![0; 32],
					timeout_timestamp: nonce,
				};
				(hash_request::<Ismp>(&Request::Post(post)), hash_post_response::<Ismp>(&response))
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

		// Insert requests and responses
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

		for (request, response) in &responses {
			let response_commitment_key = ResponseCommitments::<Test>::storage_key(*response);
			let response_receipt_key = ResponseReceipts::<Test>::storage_key(*request);
			let fee_metadata = FeeMetadata::<Test> { payer: [0; 32].into(), fee: 1000u128.into() };
			let leaf_meta = RequestMetadata {
				offchain: LeafIndexAndPos { leaf_index: 0, pos: 0 },
				fee: fee_metadata,
				claimed: false,
			};
			ResponseCommitments::<Test>::insert(*response, leaf_meta.clone());
			source_trie.insert(&response_commitment_key, &leaf_meta.encode()).unwrap();
			let receipt = ResponseReceipt { response: *response, relayer: public_key.clone() };
			dest_trie.insert(&response_receipt_key, &receipt.encode()).unwrap();
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

		for (index, (request, response)) in responses.iter().enumerate() {
			if index % 2 == 0 {
				let response_commitment_key = ResponseCommitments::<Test>::storage_key(*response);
				let response_receipt_key = ResponseReceipts::<Test>::storage_key(*request);
				source_trie.get(&response_commitment_key).unwrap();
				dest_trie.get(&response_receipt_key).unwrap();
				keys.push(Key::Response {
					response_commitment: *response,
					request_commitment: *request,
				});
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
			U256::from(10000u128)
		);
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

		let responses = (0u64..10)
			.into_iter()
			.map(|nonce| {
				let post = PostRequest {
					source: StateMachine::Kusama(2001),
					dest: StateMachine::Kusama(2000),
					nonce,
					from: vec![],
					to: vec![],
					timeout_timestamp: 0,
					body: vec![],
				};
				let response = ismp::router::PostResponse {
					post: post.clone(),
					response: vec![0; 32],
					timeout_timestamp: nonce,
				};
				(hash_request::<Ismp>(&Request::Post(post)), hash_post_response::<Ismp>(&response))
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

		// Insert requests and responses
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

		for (request, response) in &responses {
			let response_commitment_key = ResponseCommitments::<Test>::storage_key(*response);
			let response_receipt_key = ResponseReceipts::<Test>::storage_key(*request);
			let fee_metadata = FeeMetadata::<Test> { payer: [0; 32].into(), fee: 1000u128.into() };
			let leaf_meta = RequestMetadata {
				offchain: LeafIndexAndPos { leaf_index: 0, pos: 0 },
				fee: fee_metadata,
				claimed: false,
			};
			ResponseCommitments::<Test>::insert(*response, leaf_meta.clone());
			source_trie.insert(&response_commitment_key, &leaf_meta.encode()).unwrap();
			let receipt = ResponseReceipt { response: *response, relayer: eth_address.clone() };
			dest_trie.insert(&response_receipt_key, &receipt.encode()).unwrap();
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

		for (index, (request, response)) in responses.iter().enumerate() {
			if index % 2 == 0 {
				let response_commitment_key = ResponseCommitments::<Test>::storage_key(*response);
				let response_receipt_key = ResponseReceipts::<Test>::storage_key(*request);
				source_trie.get(&response_commitment_key).unwrap();
				dest_trie.get(&response_receipt_key).unwrap();
				keys.push(Key::Response {
					response_commitment: *response,
					request_commitment: *request,
				});
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
			U256::from(10000u128)
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

// ── outbound consensus delivery reward tests ────────────────────────────────
//
// These exercise `claim_outbound_consensus_delivery_reward`: build a real
// in-memory trie containing the destination's
// `pallet-ismp::StateCommitments` entry for a Hyperbridge rotation, store
// the destination state root on `Ismp` so the pallet's state-proof
// verifier can find it, set the per-chain reward, populate the rotation
// oracle, fund the treasury, and submit the claim.

mod outbound_consensus_delivery {
	use super::*;
	use crate::runtime::{
		clear_test_rotations, set_test_rotation, Balances, OutboundRewardTreasury,
	};
	use frame_support::traits::{fungible::Inspect, tokens::fungible::Mutate as FungibleMutate};
	use pallet_ismp_relayer::{
		Error, OutboundConsensusDeliveryReward, OutboundConsensusRotationsClaimed,
	};
	use polkadot_sdk::{
		frame_support::traits::Get,
		sp_io::hashing::blake2_128,
		sp_runtime::{traits::AccountIdConversion, AccountId32},
	};

	const DEST: StateMachine = StateMachine::Kusama(2001);
	const HB_CONSENSUS_STATE_ID: [u8; 4] = *b"BEEF";
	const REWARD: u128 = 1_000_000_000_000;
	const DEST_HEIGHT: u64 = 42;
	const ROTATION_HEIGHT: u64 = 7;
	const NEW_SET_ID: u64 = 3;

	fn state_commitments_key(rotation_height: u64) -> Vec<u8> {
		use polkadot_sdk::sp_io::hashing::twox_128;
		let id = StateMachineId {
			state_id: <<Test as pallet_ismp::Config>::HostStateMachine as Get<StateMachine>>::get(),
			consensus_state_id: HB_CONSENSUS_STATE_ID,
		};
		let id_encoded = id.encode();
		let height_encoded = rotation_height.encode();
		let mut key = Vec::with_capacity(64 + id_encoded.len() + height_encoded.len());
		key.extend_from_slice(&twox_128(b"Ismp"));
		key.extend_from_slice(&twox_128(b"BoundedStateCommitments"));
		key.extend_from_slice(&blake2_128(&id_encoded));
		key.extend_from_slice(&id_encoded);
		key.extend_from_slice(&blake2_128(&height_encoded));
		key.extend_from_slice(&height_encoded);
		key
	}

	fn build_proof_for_rotation(rotation_height: u64, dest_height: u64) -> Proof {
		let key = state_commitments_key(rotation_height);
		let fake_value =
			StateCommitment { timestamp: 1, overlay_root: None, state_root: Default::default() }
				.encode();

		let mut state_root = H256::default();
		let mut db = MemoryDB::<KeccakHasher>::default();
		{
			let mut trie =
				TrieDBMutBuilder::<LayoutV0<KeccakHasher>>::new(&mut db, &mut state_root).build();
			trie.insert(&key, &fake_value).unwrap();
		}

		let mut recorder = Recorder::<LayoutV0<KeccakHasher>>::default();
		{
			let trie = TrieDBBuilder::<LayoutV0<KeccakHasher>>::new(&db, &state_root)
				.with_recorder(&mut recorder)
				.build();
			trie.get(&key).unwrap();
		}
		let storage_proof: Vec<Vec<u8>> = recorder.drain().into_iter().map(|f| f.data).collect();

		Ismp::default()
			.store_state_machine_commitment(
				StateMachineHeight {
					id: StateMachineId {
						state_id: DEST,
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: dest_height,
				},
				StateCommitment { timestamp: 100, overlay_root: None, state_root },
			)
			.unwrap();
		Ismp::default()
			.store_state_machine_update_time(
				StateMachineHeight {
					id: StateMachineId {
						state_id: DEST,
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: dest_height,
				},
				Duration::from_secs(100),
			)
			.unwrap();
		Ismp::default()
			.store_consensus_state(MOCK_CONSENSUS_STATE_ID, Default::default())
			.unwrap();
		Ismp::default()
			.store_consensus_state_id(MOCK_CONSENSUS_STATE_ID, MOCK_CONSENSUS_CLIENT_ID)
			.unwrap();
		Ismp::default()
			.store_unbonding_period(MOCK_CONSENSUS_STATE_ID, 10_000_000_000)
			.unwrap();
		Ismp::default()
			.store_challenge_period(
				StateMachineId { state_id: DEST, consensus_state_id: MOCK_CONSENSUS_STATE_ID },
				0,
			)
			.unwrap();

		Proof {
			height: StateMachineHeight {
				id: StateMachineId { state_id: DEST, consensus_state_id: MOCK_CONSENSUS_STATE_ID },
				height: dest_height,
			},
			proof: SubstrateStateProof::StateProof(StateMachineProof {
				hasher: HashAlgorithm::Keccak,
				storage_proof,
			})
			.encode(),
		}
	}

	fn setup_and_build_claim() -> (sp_core::sr25519::Pair, OutboundConsensusDeliveryClaim, Vec<u8>)
	{
		clear_test_rotations();
		set_test_rotation(NEW_SET_ID, ROTATION_HEIGHT);
		OutboundConsensusDeliveryReward::<Test>::insert(DEST, REWARD);

		let treasury: AccountId32 = <Test as pallet_ismp_relayer::Config>::TreasuryPalletId::get()
			.into_account_truncating();
		Balances::mint_into(&treasury, REWARD * 10).unwrap();

		let pair = sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
		let claimer = pair.public().0;

		let state_proof = build_proof_for_rotation(ROTATION_HEIGHT, DEST_HEIGHT);
		let claim = OutboundConsensusDeliveryClaim {
			state_proof,
			rotation_height: ROTATION_HEIGHT,
			new_set_id: NEW_SET_ID,
			hb_consensus_state_id: HB_CONSENSUS_STATE_ID,
			claimer,
		};
		let payload = (b"outbound-consensus-delivery-reward", DEST, NEW_SET_ID, claimer).encode();
		let signature = pair.sign(&keccak_256(&payload)).0.to_vec();

		(pair, claim, signature)
	}

	#[test]
	fn happy_path_pays_out_and_marks_claimed() {
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			let (pair, claim, signature) = setup_and_build_claim();
			let claimer: AccountId32 = pair.public().0.into();
			let starting_balance = Balances::balance(&claimer);

			pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
				RuntimeOrigin::none(),
				claim.clone(),
				signature,
			)
			.unwrap();

			assert_eq!(
				Balances::balance(&claimer),
				starting_balance + REWARD,
				"claimer should receive the reward from the treasury",
			);
			assert!(
				OutboundConsensusRotationsClaimed::<Test>::contains_key(DEST, NEW_SET_ID),
				"idempotency tag should be inserted",
			);
		})
	}

	#[test]
	fn second_claim_for_same_rotation_is_rejected() {
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			let (_pair, claim, signature) = setup_and_build_claim();

			pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
				RuntimeOrigin::none(),
				claim.clone(),
				signature.clone(),
			)
			.unwrap();

			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					claim,
					signature,
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundRotationAlreadyClaimed.into());
		})
	}

	#[test]
	fn unknown_rotation_is_rejected() {
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			let (_pair, mut claim, signature) = setup_and_build_claim();
			clear_test_rotations();
			claim.new_set_id = 999;

			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					claim,
					signature,
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundRotationNotKnown.into());
		})
	}

	#[test]
	fn height_mismatch_is_rejected() {
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			let (_pair, mut claim, signature) = setup_and_build_claim();
			claim.rotation_height = ROTATION_HEIGHT + 1;

			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					claim,
					signature,
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundRotationNotKnown.into());
		})
	}

	#[test]
	fn invalid_signature_is_rejected() {
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			let (_pair, claim, _real_sig) = setup_and_build_claim();
			let imposter =
				sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
			let payload =
				(b"outbound-consensus-delivery-reward", DEST, NEW_SET_ID, claim.claimer).encode();
			let bogus = imposter.sign(&keccak_256(&payload)).0.to_vec();

			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					claim,
					bogus,
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::InvalidSignature.into());
		})
	}

	#[test]
	fn zero_reward_is_rejected() {
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			let (_pair, claim, signature) = setup_and_build_claim();
			OutboundConsensusDeliveryReward::<Test>::remove(DEST);

			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					claim,
					signature,
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundNoRewardConfigured.into());
		})
	}
}

// ── EVM-destination tests ───────────────────────────────────────────────────
//
// The pallet's claim extrinsic has a `match destination` on
// `StateMachine::Evm(_)` that derives a contract-scoped storage key instead
// of the substrate `BoundedStateCommitments` key. These tests exercise the
// EVM branch. We don't construct a real Ethereum Patricia trie (that's a
// multi-hundred-line setup with account tries and RLP-encoded accounts);
// instead we assert the EVM-specific pre-verification errors fire, and that
// the branch is reachable end-to-end. The substrate tests above already
// cover the post-verification flow (reward transfer, idempotency, event),
// and the two branches converge to the same code after key derivation.

mod outbound_consensus_delivery_evm {
	use super::*;
	use crate::runtime::{
		clear_test_rotations, set_test_rotation, Balances, OutboundRewardTreasury,
	};
	use frame_support::traits::{fungible::Inspect, tokens::fungible::Mutate as FungibleMutate};
	use pallet_ismp_host_executive::EvmHosts;
	use pallet_ismp_relayer::{Error, OutboundConsensusDeliveryReward};
	use polkadot_sdk::{
		frame_support::traits::Get,
		sp_runtime::{traits::AccountIdConversion, AccountId32},
	};

	/// EVM destination chain id used across the tests. Uses a real EVM
	/// state machine tag so the pallet's `match destination` takes the
	/// `StateMachine::Evm(_)` arm.
	const DEST: StateMachine = StateMachine::Evm(11155111);
	const HB_CONSENSUS_STATE_ID: [u8; 4] = *b"BEEF";
	const REWARD: u128 = 1_000_000_000_000;
	const ROTATION_HEIGHT: u64 = 9;
	const NEW_SET_ID: u64 = 4;
	const EVM_HOST: [u8; 20] = [0xCAu8; 20];

	fn dummy_claim(pair: &sp_core::sr25519::Pair) -> (OutboundConsensusDeliveryClaim, Vec<u8>) {
		// EVM destinations need a state proof, but every error path we can
		// reach here (no EvmHost, already claimed, unknown rotation,
		// invalid signature, no reward) fires before proof verification.
		// A placeholder proof is fine.
		let claim = OutboundConsensusDeliveryClaim {
			state_proof: Proof {
				height: StateMachineHeight {
					id: StateMachineId {
						state_id: DEST,
						consensus_state_id: MOCK_CONSENSUS_STATE_ID,
					},
					height: 100,
				},
				proof: vec![],
			},
			rotation_height: ROTATION_HEIGHT,
			new_set_id: NEW_SET_ID,
			hb_consensus_state_id: HB_CONSENSUS_STATE_ID,
			claimer: pair.public().0,
		};
		let payload =
			(b"outbound-consensus-delivery-reward", DEST, NEW_SET_ID, pair.public().0).encode();
		let signature = pair.sign(&keccak_256(&payload)).0.to_vec();
		(claim, signature)
	}

	#[test]
	fn missing_evm_host_is_rejected() {
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			clear_test_rotations();
			set_test_rotation(NEW_SET_ID, ROTATION_HEIGHT);
			OutboundConsensusDeliveryReward::<Test>::insert(DEST, REWARD);

			let treasury: AccountId32 =
				<Test as pallet_ismp_relayer::Config>::TreasuryPalletId::get()
					.into_account_truncating();
			Balances::mint_into(&treasury, REWARD * 10).unwrap();

			let pair = sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
			let (claim, signature) = dummy_claim(&pair);

			// No EvmHosts entry registered for DEST. The key-derivation
			// helper should fail with OutboundEvmHostNotKnown before any
			// proof verification is attempted.
			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					claim,
					signature,
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundEvmHostNotKnown.into());
		})
	}

	#[test]
	fn evm_branch_reaches_proof_verification() {
		// Confirms the EVM key-derivation succeeds (EvmHost registered,
		// rotation oracle matches, signature verifies) and the pallet
		// advances to the proof step. With a placeholder empty proof, the
		// destination's state commitment isn't known to Ismp, so we get
		// `OutboundDestinationStateNotKnown` — not any earlier error. This
		// is the "EVM branch is reachable" smoke test.
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			clear_test_rotations();
			set_test_rotation(NEW_SET_ID, ROTATION_HEIGHT);
			OutboundConsensusDeliveryReward::<Test>::insert(DEST, REWARD);
			EvmHosts::<Test>::insert(DEST, sp_core::H160::from(EVM_HOST));

			let treasury: AccountId32 =
				<Test as pallet_ismp_relayer::Config>::TreasuryPalletId::get()
					.into_account_truncating();
			Balances::mint_into(&treasury, REWARD * 10).unwrap();

			let pair = sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
			let (claim, signature) = dummy_claim(&pair);

			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					claim,
					signature,
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundDestinationStateNotKnown.into());
		})
	}

	#[test]
	fn evm_unknown_rotation_is_rejected() {
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			clear_test_rotations();
			OutboundConsensusDeliveryReward::<Test>::insert(DEST, REWARD);
			EvmHosts::<Test>::insert(DEST, sp_core::H160::from(EVM_HOST));

			let pair = sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
			let (claim, signature) = dummy_claim(&pair);

			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					claim,
					signature,
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundRotationNotKnown.into());
		})
	}

	#[test]
	fn evm_invalid_signature_is_rejected() {
		new_test_ext().execute_with(|| {
			set_timestamp::<Test>(10_000_000_000);
			clear_test_rotations();
			set_test_rotation(NEW_SET_ID, ROTATION_HEIGHT);
			OutboundConsensusDeliveryReward::<Test>::insert(DEST, REWARD);
			EvmHosts::<Test>::insert(DEST, sp_core::H160::from(EVM_HOST));

			let pair = sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
			let (claim, _) = dummy_claim(&pair);
			let imposter =
				sp_core::sr25519::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
			let payload =
				(b"outbound-consensus-delivery-reward", DEST, NEW_SET_ID, claim.claimer).encode();
			let bogus = imposter.sign(&keccak_256(&payload)).0.to_vec();

			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					claim,
					bogus,
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::InvalidSignature.into());
		})
	}
}
