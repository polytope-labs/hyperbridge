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
	OutboundConsensusDeliveryClaim, OutboundRequestDeliveryClaim,
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

	#[test]
	fn substrate_destination_is_rejected() {
		// HostParams::SubstrateHostParam variant should reject before key
		// derivation. The reward is HandlerV2-specific so non-EVM
		// destinations have no path through.
		new_test_ext().execute_with(|| {
			use pallet_hyperbridge::{SubstrateHostParams, VersionedHostParams};
			let substrate_dest = StateMachine::Kusama(2001);
			HostParams::<Test>::insert(
				substrate_dest,
				HostParam::SubstrateHostParam(VersionedHostParams::V1(SubstrateHostParams {
					default_per_byte_fee: 0u128,
					per_byte_fees: Default::default(),
					asset_registration_fee: 0u128,
				})),
			);

			let mut claim = placeholder_claim();
			claim.state_proof.height.id.state_id = substrate_dest;

			let err =
				pallet_ismp_relayer::Pallet::<Test>::claim_outbound_consensus_delivery_reward(
					RuntimeOrigin::none(),
					claim,
				)
				.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundDestinationNotEvm.into());
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
	fn unsupported_destination_is_rejected() {
		new_test_ext().execute_with(|| {
			let dest = StateMachine::Tendermint(*b"cosm");
			let req = post_request(dest, MODULE_ID);
			register_request(&req);
			set_reward(dest, MODULE_ID, REWARD);

			let err = pallet_ismp_relayer::Pallet::<Test>::claim_outbound_request_delivery_reward(
				RuntimeOrigin::none(),
				placeholder_claim_for(req),
			)
			.unwrap_err();
			assert_eq!(err, Error::<Test>::OutboundRequestUnsupportedDestination.into());
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

		SubstrateStateProof::OverlayProof(StateMachineProof {
			hasher: HashAlgorithm::Keccak,
			storage_proof,
		})
		.encode()
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

	mod request_receipt_key {
		use super::*;

		const COMMITMENT: H256 = H256::repeat_byte(0x42);

		#[test]
		fn evm_returns_slot_hash() {
			new_test_ext().execute_with(|| {
				assert!(pallet_ismp_relayer::Pallet::<Test>::request_receipt_key(
					EVM_DEST, COMMITMENT
				)
				.is_some());
			})
		}

		#[test]
		fn substrate_returns_child_trie_key() {
			new_test_ext().execute_with(|| {
				let key = pallet_ismp_relayer::Pallet::<Test>::request_receipt_key(
					SUBSTRATE_DEST,
					COMMITMENT,
				)
				.unwrap();
				assert_eq!(key, RequestReceipts::<Test>::storage_key(COMMITMENT));
			})
		}

		#[test]
		fn unsupported_returns_none() {
			new_test_ext().execute_with(|| {
				assert!(pallet_ismp_relayer::Pallet::<Test>::request_receipt_key(
					StateMachine::Tendermint(*b"cosm"),
					COMMITMENT,
				)
				.is_none());
			})
		}
	}

	mod decode_request_receipt_relayer {
		use super::*;

		#[test]
		fn decodes_evm_address() {
			new_test_ext().execute_with(|| {
				let address = alloy_primitives::Address::from([0x11u8; 20]);
				let raw = alloy_rlp::encode(&address);

				let decoded = pallet_ismp_relayer::Pallet::<Test>::decode_request_receipt_relayer(
					EVM_DEST, &raw,
				)
				.unwrap()
				.unwrap();
				assert_eq!(decoded, address.0.to_vec());
			})
		}

		#[test]
		fn decodes_substrate_raw_bytes() {
			new_test_ext().execute_with(|| {
				let pubkey = [0x77u8; 32];
				let raw = pubkey.to_vec().encode();

				let decoded = pallet_ismp_relayer::Pallet::<Test>::decode_request_receipt_relayer(
					SUBSTRATE_DEST,
					&raw,
				)
				.unwrap()
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

				let decoded = pallet_ismp_relayer::Pallet::<Test>::decode_request_receipt_relayer(
					SUBSTRATE_DEST,
					&raw,
				)
				.unwrap()
				.unwrap();
				assert_eq!(decoded, pair.public().0.to_vec());
			})
		}

		#[test]
		fn unsupported_returns_none() {
			new_test_ext().execute_with(|| {
				let raw = vec![0u8; 21];
				assert!(pallet_ismp_relayer::Pallet::<Test>::decode_request_receipt_relayer(
					StateMachine::Tendermint(*b"cosm"),
					&raw,
				)
				.unwrap()
				.is_none());
			})
		}
	}
}
