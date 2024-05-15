// Copyright (c) 2024 Polytope Labs.
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

use alloy_primitives::hex;
use codec::{Decode, Encode};
use ethereum_triedb::{keccak::KeccakHasher, MemoryDB, StorageProof};
use evm_common::types::EvmStateProof;
use frame_support::crypto::ecdsa::ECDSAExt;
use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	host::{IsmpHost, StateMachine},
	messaging::{hash_post_response, hash_request, Proof},
	router::{Post, Request},
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
};
use sp_core::{Pair, H160, H256, U256};
use sp_trie::LayoutV0;
use std::{fs::File, io::Read, time::Duration};
use substrate_state_machine::{HashAlgorithm, StateMachineProof, SubstrateStateProof};
use trie_db::{Recorder, Trie, TrieDBBuilder, TrieDBMutBuilder, TrieMut};

use crate::runtime::{
	new_test_ext, set_timestamp, Ismp, RuntimeCall, RuntimeOrigin, Test, MOCK_CONSENSUS_CLIENT_ID,
	MOCK_CONSENSUS_STATE_ID,
};
use ismp::host::Ethereum;
use ismp_bsc::BSC_CONSENSUS_ID;
use ismp_sync_committee::BEACON_CONSENSUS_ID;
use pallet_ismp::{dispatcher::RequestMetadata, mmr::LeafIndexAndPos};

#[test]
fn test_withdrawal_proof() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		set_timestamp::<Test>(10_000_000_000);
		let requests = (0u64..10)
			.into_iter()
			.map(|nonce| {
				let post = Post {
					source: StateMachine::Kusama(2000),
					dest: StateMachine::Kusama(2001),
					nonce,
					from: vec![],
					to: vec![],
					timeout_timestamp: 0,
					data: vec![],
				};
				hash_request::<Ismp>(&Request::Post(post))
			})
			.collect::<Vec<_>>();

		let responses = (0u64..10)
			.into_iter()
			.map(|nonce| {
				let post = Post {
					source: StateMachine::Kusama(2001),
					dest: StateMachine::Kusama(2000),
					nonce,
					from: vec![],
					to: vec![],
					timeout_timestamp: 0,
					data: vec![],
				};
				let response = ismp::router::PostResponse {
					post: post.clone(),
					response: vec![0; 32],
					timeout_timestamp: nonce,
				};
				(hash_request::<Ismp>(&Request::Post(post)), hash_post_response::<Ismp>(&response))
			})
			.collect::<Vec<_>>();

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
				mmr: LeafIndexAndPos { leaf_index: 0, pos: 0 },
				fee: fee_metadata,
				claimed: false,
			};
			RequestCommitments::<Test>::insert(*request, leaf_meta.clone());
			source_trie.insert(&request_commitment_key, &leaf_meta.encode()).unwrap();
			dest_trie.insert(&request_receipt_key, &vec![1u8; 32].encode()).unwrap();
		}

		for (request, response) in &responses {
			let response_commitment_key = ResponseCommitments::<Test>::storage_key(*response);
			let response_receipt_key = ResponseReceipts::<Test>::storage_key(*request);
			let fee_metadata = FeeMetadata::<Test> { payer: [0; 32].into(), fee: 1000u128.into() };
			let leaf_meta = RequestMetadata {
				mmr: LeafIndexAndPos { leaf_index: 0, pos: 0 },
				fee: fee_metadata,
				claimed: false,
			};
			ResponseCommitments::<Test>::insert(*response, leaf_meta.clone());
			source_trie.insert(&response_commitment_key, &leaf_meta.encode()).unwrap();
			let receipt = ResponseReceipt { response: *response, relayer: vec![2; 32] };
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

		host.store_challenge_period(MOCK_CONSENSUS_STATE_ID, 0).unwrap();

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
		};

		pallet_ismp_relayer::Pallet::<Test>::accumulate_fees(
			RuntimeOrigin::none(),
			withdrawal_proof,
		)
		.unwrap();

		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(StateMachine::Kusama(2000), vec![1; 32]),
			U256::from(5000u128)
		);
		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(StateMachine::Kusama(2000), vec![2; 32]),
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
			U256::from(5000u128),
		);
		let message = message(0, StateMachine::Kusama(2000), 2000u128.into());
		let signature = pair.sign(&message).0.to_vec();

		let withdrawal_input = WithdrawalInputData {
			signature: Signature::Sr25519 { public_key: public_key.clone(), signature },
			dest_chain: StateMachine::Kusama(2000),
			amount: U256::from(2000u128),
		};

		pallet_ismp_relayer::Pallet::<Test>::withdraw_fees(
			RuntimeOrigin::none(),
			withdrawal_input.clone(),
		)
		.unwrap();
		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(StateMachine::Kusama(2000), public_key.clone()),
			3_000u128.into()
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
			StateMachine::Ethereum(Ethereum::Base),
			HostParam::EvmHostParam(evm_host_params),
		);
		pallet_ismp_relayer::Fees::<Test>::insert(
			StateMachine::Ethereum(Ethereum::Base),
			address.to_vec(),
			U256::from(5000u128),
		);
		let message = message(0, StateMachine::Ethereum(Ethereum::Base), 2000u128.into());
		let signature = pair.sign_prehashed(&message).0.to_vec();

		let withdrawal_input = WithdrawalInputData {
			signature: Signature::Ethereum { address: address.to_vec(), signature },
			dest_chain: StateMachine::Ethereum(Ethereum::Base),
			amount: U256::from(2000u128),
		};

		pallet_ismp_relayer::Pallet::<Test>::withdraw_fees(
			RuntimeOrigin::none(),
			withdrawal_input.clone(),
		)
		.unwrap();
		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(
				StateMachine::Ethereum(Ethereum::Base),
				address.to_vec()
			),
			3_000u128.into()
		);

		assert_eq!(
			pallet_ismp_relayer::Nonce::<Test>::get(
				address.to_vec(),
				StateMachine::Ethereum(Ethereum::Base)
			),
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
fn test_evm_accumulate_fees() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		set_timestamp::<Test>(10_000_000_000);
		let bsc_root = H256::from_slice(
			&hex::decode("5e7239f000b1416b8230416ada9d39c342979aa5746172a86df00dda9fd221c9")
				.unwrap(),
		);
		let op_root = H256::from_slice(
			&hex::decode("6dfbb6ec490b26ca38796ecf291ff20a6d50cc8261b0d85f27e0962a6730661e")
				.unwrap(),
		);
		let claim_proof =
			hex::decode(read_file_string("src/tests/proofs/accumulate_fee_proof.txt")).unwrap();

		let op_host = H160::from(hex!("6bb05F1997396eC1A4A3040f48215bbC101ab7b6"));
		let bsc_host = H160::from(hex!("C0291b0eD2E44100d1D77d9cEeeE0535B26AA45C"));

		dbg!(claim_proof.len());

		let mut claim_proof = WithdrawalProof::decode(&mut &*claim_proof).unwrap();
		for key in claim_proof.commitments.clone() {
			match key {
				Key::Request(req) => {
					let leaf_meta = RequestMetadata {
						mmr: LeafIndexAndPos { leaf_index: 0, pos: 0 },
						fee: FeeMetadata::<Test> { payer: [0; 32].into(), fee: 1000u128.into() },
						claimed: false,
					};
					RequestCommitments::<Test>::insert(req, leaf_meta)
				},
				Key::Response { response_commitment, .. } => {
					let leaf_meta = RequestMetadata {
						mmr: LeafIndexAndPos { leaf_index: 0, pos: 0 },
						fee: FeeMetadata::<Test> { payer: [0; 32].into(), fee: 1000u128.into() },
						claimed: false,
					};
					ResponseCommitments::<Test>::insert(response_commitment, leaf_meta);
				},
			}
		}
		let mut source_evm_proof =
			EvmStateProof::decode(&mut &*claim_proof.source_proof.proof).unwrap();
		let mut dest_evm_proof =
			EvmStateProof::decode(&mut &*claim_proof.dest_proof.proof).unwrap();
		{
			let mut storage_proofs = vec![];
			for (_, proof) in source_evm_proof.storage_proof.clone() {
				storage_proofs.push(StorageProof::new(proof))
			}

			let storage_proof = StorageProof::merge(storage_proofs);
			source_evm_proof.storage_proof =
				vec![(bsc_host.0.to_vec(), storage_proof.into_nodes().into_iter().collect())]
					.into_iter()
					.collect();
			claim_proof.source_proof.proof = source_evm_proof.encode();
		}

		{
			let mut storage_proofs = vec![];
			for (_, proof) in dest_evm_proof.storage_proof.clone() {
				storage_proofs.push(StorageProof::new(proof))
			}

			let storage_proof = StorageProof::merge(storage_proofs);
			dest_evm_proof.storage_proof =
				vec![(op_host.0.to_vec(), storage_proof.into_nodes().into_iter().collect())]
					.into_iter()
					.collect();
			claim_proof.dest_proof.proof = dest_evm_proof.encode();
		}

		dbg!(claim_proof.encode().len());

		let host = Ismp::default();
		host.store_state_machine_commitment(
			claim_proof.source_proof.height,
			StateCommitment { timestamp: 100, overlay_root: None, state_root: bsc_root },
		)
		.unwrap();

		host.store_state_machine_commitment(
			claim_proof.dest_proof.height,
			StateCommitment { timestamp: 100, overlay_root: None, state_root: op_root },
		)
		.unwrap();

		host.store_state_machine_update_time(
			claim_proof.source_proof.height,
			Duration::from_secs(100),
		)
		.unwrap();

		host.store_state_machine_update_time(
			claim_proof.dest_proof.height,
			Duration::from_secs(100),
		)
		.unwrap();
		let bsc_consensus_state = ismp_bsc::ConsensusState {
			current_validators: vec![],
			next_validators: None,
			finalized_height: 0,
			finalized_hash: Default::default(),
			current_epoch: 0,
			ismp_contract_address: bsc_host,
		};
		let sync_committee_consensus_state = ismp_sync_committee::types::ConsensusState {
			frozen_height: None,
			light_client_state: Default::default(),
			ismp_contract_addresses: vec![(StateMachine::Ethereum(Ethereum::Optimism), op_host)]
				.into_iter()
				.collect(),
			l2_consensus: Default::default(),
		};
		host.store_consensus_state(
			claim_proof.source_proof.height.id.consensus_state_id,
			bsc_consensus_state.encode(),
		)
		.unwrap();
		host.store_consensus_state(
			claim_proof.dest_proof.height.id.consensus_state_id,
			sync_committee_consensus_state.encode(),
		)
		.unwrap();

		host.store_consensus_state_id(
			claim_proof.source_proof.height.id.consensus_state_id,
			BSC_CONSENSUS_ID,
		)
		.unwrap();

		host.store_consensus_state_id(
			claim_proof.dest_proof.height.id.consensus_state_id,
			BEACON_CONSENSUS_ID,
		)
		.unwrap();

		host.store_unbonding_period(
			claim_proof.source_proof.height.id.consensus_state_id,
			10_000_000_000,
		)
		.unwrap();

		host.store_challenge_period(claim_proof.source_proof.height.id.consensus_state_id, 0)
			.unwrap();

		host.store_unbonding_period(
			claim_proof.dest_proof.height.id.consensus_state_id,
			10_000_000_000,
		)
		.unwrap();

		host.store_challenge_period(claim_proof.dest_proof.height.id.consensus_state_id, 0)
			.unwrap();

		pallet_ismp_relayer::Pallet::<Test>::accumulate_fees(
			RuntimeOrigin::none(),
			claim_proof.clone(),
		)
		.unwrap();

		assert_eq!(
			pallet_ismp_relayer::Fees::<Test>::get(
				StateMachine::Bsc,
				vec![
					125, 114, 152, 63, 237, 193, 243, 50, 229, 80, 6, 254, 162, 162, 175, 193, 72,
					246, 97, 66
				]
			),
			U256::from(50_000_000_000_000_000_000u128)
		);

		assert!(pallet_ismp_relayer::Pallet::<Test>::accumulate_fees(
			RuntimeOrigin::none(),
			claim_proof.clone()
		)
		.is_err());
	})
}

#[test]
fn test_evm_accumulate_fees_with_zero_fee_values() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let claim_proof = setup_host_for_accumulate_fees();

		pallet_ismp_relayer::Pallet::<Test>::accumulate_fees(
			RuntimeOrigin::none(),
			claim_proof.clone(),
		)
		.unwrap();
		assert_eq!(claim_proof.commitments.len(), 6);
		let claimed = claim_proof.commitments.into_iter().fold(0u32, |acc, key| match key {
			Key::Request(req) => RequestCommitments::<Test>::get(req)
				.unwrap()
				.claimed
				.then(|| acc + 1)
				.unwrap_or(acc),
			Key::Response { response_commitment, .. } =>
				ResponseCommitments::<Test>::get(response_commitment)
					.unwrap()
					.claimed
					.then(|| acc + 1)
					.unwrap_or(acc),
		});
		assert_eq!(claimed, 5);
	})
}

fn setup_host_for_accumulate_fees() -> WithdrawalProof {
	set_timestamp::<Test>(10_000_000_000);
	let bsc_root = H256::from_slice(
		&hex::decode("1f395eaae1db73f6213984c8a47b0e025a5fc47390aab06cc93144cac993defd").unwrap(),
	);
	let op_root = H256::from_slice(
		&hex::decode("f123c7969c1021781d4a3a2f9055786f309a051f14bc840789dc2a6c2713e501").unwrap(),
	);

	let claim_proof =
		hex::decode(read_file_string("src/tests/proofs/withdrawal_claim_proof.txt")).unwrap();

	let op_host = H160::from(hex!("1D14e30e440B8DBA9765108eC291B7b66F98Fd09"));
	let bsc_host = H160::from(hex!("4e5bbdd9fE89F54157DDb64b21eD4D1CA1CDf9a6"));

	let claim_proof = WithdrawalProof::decode(&mut &*claim_proof).unwrap();

	for key in claim_proof.commitments.clone() {
		match key {
			Key::Request(req) => {
				let leaf_meta = RequestMetadata {
					mmr: LeafIndexAndPos { leaf_index: 0, pos: 0 },
					fee: FeeMetadata::<Test> { payer: [0; 32].into(), fee: 1000u128.into() },
					claimed: false,
				};
				RequestCommitments::<Test>::insert(req, leaf_meta)
			},
			Key::Response { response_commitment, .. } => {
				let leaf_meta = RequestMetadata {
					mmr: LeafIndexAndPos { leaf_index: 0, pos: 0 },
					fee: FeeMetadata::<Test> { payer: [0; 32].into(), fee: 1000u128.into() },
					claimed: false,
				};
				ResponseCommitments::<Test>::insert(response_commitment, leaf_meta);
			},
		}
	}

	let host = Ismp::default();
	host.store_state_machine_commitment(
		claim_proof.source_proof.height,
		StateCommitment { timestamp: 100, overlay_root: None, state_root: bsc_root },
	)
	.unwrap();

	host.store_state_machine_commitment(
		claim_proof.dest_proof.height,
		StateCommitment { timestamp: 100, overlay_root: None, state_root: op_root },
	)
	.unwrap();

	host.store_state_machine_update_time(claim_proof.source_proof.height, Duration::from_secs(100))
		.unwrap();

	host.store_state_machine_update_time(claim_proof.dest_proof.height, Duration::from_secs(100))
		.unwrap();
	let bsc_consensus_state = ismp_bsc::ConsensusState {
		current_validators: vec![],
		next_validators: None,
		finalized_height: 0,
		finalized_hash: Default::default(),
		current_epoch: 0,
		ismp_contract_address: bsc_host,
	};
	let sync_committee_consensus_state = ismp_sync_committee::types::ConsensusState {
		frozen_height: None,
		light_client_state: Default::default(),
		ismp_contract_addresses: vec![(StateMachine::Ethereum(Ethereum::Base), op_host)]
			.into_iter()
			.collect(),
		l2_consensus: Default::default(),
	};
	host.store_consensus_state(
		claim_proof.source_proof.height.id.consensus_state_id,
		bsc_consensus_state.encode(),
	)
	.unwrap();
	host.store_consensus_state(
		claim_proof.dest_proof.height.id.consensus_state_id,
		sync_committee_consensus_state.encode(),
	)
	.unwrap();

	host.store_consensus_state_id(
		claim_proof.source_proof.height.id.consensus_state_id,
		BSC_CONSENSUS_ID,
	)
	.unwrap();

	host.store_consensus_state_id(
		claim_proof.dest_proof.height.id.consensus_state_id,
		BEACON_CONSENSUS_ID,
	)
	.unwrap();

	host.store_unbonding_period(
		claim_proof.source_proof.height.id.consensus_state_id,
		10_000_000_000,
	)
	.unwrap();

	host.store_challenge_period(claim_proof.source_proof.height.id.consensus_state_id, 0)
		.unwrap();

	host.store_unbonding_period(
		claim_proof.dest_proof.height.id.consensus_state_id,
		10_000_000_000,
	)
	.unwrap();

	host.store_challenge_period(claim_proof.dest_proof.height.id.consensus_state_id, 0)
		.unwrap();

	claim_proof
}

pub fn read_file_string(path: &str) -> String {
	let mut file = File::open(path).unwrap();

	let mut contents = String::new();
	file.read_to_string(&mut contents).unwrap();

	contents
}

pub fn encode_accumulate_fees_call() -> Vec<u8> {
	let claim_proof = setup_host_for_accumulate_fees();

	let call = RuntimeCall::Relayer(pallet_ismp_relayer::Call::accumulate_fees {
		withdrawal_proof: claim_proof,
	});

	call.encode()
}
