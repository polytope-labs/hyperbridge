// Copyright (C) 2023 Polytope Labs.
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
use ethereum_trie::{keccak::KeccakHasher, MemoryDB, StorageProof};
use frame_support::crypto::ecdsa::ECDSAExt;
use ismp::{
    consensus::{StateCommitment, StateMachineHeight, StateMachineId},
    host::{IsmpHost, StateMachine},
    messaging::Proof,
    router::{Post, Request},
    util::{hash_post_response, hash_request},
};
use ismp_sync_committee::types::EvmStateProof;
use pallet_ismp::{
    child_trie::{RequestCommitments, RequestReceipts, ResponseCommitments, ResponseReceipts},
    dispatcher::FeeMetadata,
    host::Host,
    primitives::{HashAlgorithm, SubstrateStateProof},
    ResponseReceipt,
};
use pallet_ismp_relayer::{
    self as pallet_ismp_relayer, message,
    withdrawal::{Key, Signature, WithdrawalInputData, WithdrawalProof},
    Claimed,
};
use sp_core::{Pair, H160, H256, U256};
use sp_trie::LayoutV0;
use std::time::Duration;
use trie_db::{Recorder, Trie, TrieDBBuilder, TrieDBMutBuilder, TrieMut};

use crate::runtime::{
    new_test_ext, set_timestamp, RuntimeOrigin, Test, MOCK_CONSENSUS_CLIENT_ID,
    MOCK_CONSENSUS_STATE_ID,
};
use ismp::host::Ethereum;
use ismp_bsc::BSC_CONSENSUS_ID;
use ismp_sync_committee::BEACON_CONSENSUS_ID;
use pallet_ismp::{dispatcher::LeafMetadata, primitives::LeafIndexAndPos};

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
                    gas_limit: 0,
                };
                hash_request::<Host<Test>>(&Request::Post(post))
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
                    gas_limit: 0,
                };
                let response = ismp::router::PostResponse {
                    post: post.clone(),
                    response: vec![0; 32],
                    timeout_timestamp: nonce,
                    gas_limit: nonce,
                };
                (
                    hash_request::<Host<Test>>(&Request::Post(post)),
                    hash_post_response::<Host<Test>>(&response),
                )
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
            let fee_metadata = FeeMetadata::<Test> { origin: [0; 32].into(), fee: 1000u128.into() };
            let leaf_meta =
                LeafMetadata { mmr: LeafIndexAndPos { leaf_index: 0, pos: 0 }, meta: fee_metadata };
            source_trie.insert(&request_commitment_key, &leaf_meta.encode()).unwrap();
            dest_trie.insert(&request_receipt_key, &vec![1u8; 32].encode()).unwrap();
        }

        for (request, response) in &responses {
            let response_commitment_key = ResponseCommitments::<Test>::storage_key(*response);
            let response_receipt_key = ResponseReceipts::<Test>::storage_key(*request);
            let fee_metadata = FeeMetadata::<Test> { origin: [0; 32].into(), fee: 1000u128.into() };
            let leaf_meta =
                LeafMetadata { mmr: LeafIndexAndPos { leaf_index: 0, pos: 0 }, meta: fee_metadata };
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

        let source_state_proof = SubstrateStateProof::OverlayProof {
            hasher: HashAlgorithm::Keccak,
            storage_proof: source_keys_proof,
        };

        let dest_state_proof = SubstrateStateProof::OverlayProof {
            hasher: HashAlgorithm::Keccak,
            storage_proof: dest_keys_proof,
        };

        let host = Host::<Test>::default();
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
        let pair = sp_core::ecdsa::Pair::from_seed_slice(H256::random().as_bytes()).unwrap();
        let address = pair.public().to_eth_address().unwrap();
        pallet_ismp_relayer::Fees::<Test>::insert(
            StateMachine::Kusama(2000),
            address.to_vec(),
            U256::from(5000u128),
        );
        let message = message(0, StateMachine::Kusama(2000), 2000u128.into());
        let signature = pair.sign_prehashed(&message).0.to_vec();

        let withdrawal_input = WithdrawalInputData {
            signature: Signature::Ethereum { address: address.to_vec(), signature },
            dest_chain: StateMachine::Kusama(2000),
            amount: U256::from(2000u128),
            gas_limit: 10_000_000,
        };

        pallet_ismp_relayer::Pallet::<Test>::withdraw_fees(
            RuntimeOrigin::none(),
            withdrawal_input.clone(),
        )
        .unwrap();
        assert_eq!(
            pallet_ismp_relayer::Fees::<Test>::get(StateMachine::Kusama(2000), address.to_vec()),
            3_000u128.into()
        );

        assert_eq!(
            pallet_ismp_relayer::Nonce::<Test>::get(address.to_vec(), StateMachine::Kusama(2000)),
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
        let bsc_root = H256::from_slice(&hex::decode("5e7239f000b1416b8230416ada9d39c342979aa5746172a86df00dda9fd221c9").unwrap());
        let op_root = H256::from_slice(&hex::decode("6dfbb6ec490b26ca38796ecf291ff20a6d50cc8261b0d85f27e0962a6730661e").unwrap());

        let op_host = H160::from(hex!("6bb05F1997396eC1A4A3040f48215bbC101ab7b6"));
        let bsc_host =  H160::from(hex!("C0291b0eD2E44100d1D77d9cEeeE0535B26AA45C"));

        dbg!(claim_proof.len());

        let mut claim_proof = WithdrawalProof::decode(&mut &*claim_proof).unwrap();

        let mut source_evm_proof = EvmStateProof::decode(&mut &*claim_proof.source_proof.proof).unwrap();
        let mut dest_evm_proof = EvmStateProof::decode(&mut &*claim_proof.dest_proof.proof).unwrap();
        {
            let mut storage_proofs = vec![];
            for (_, proof) in source_evm_proof.storage_proof.clone() {
                storage_proofs.push(StorageProof::new(proof))
            }

            let storage_proof = StorageProof::merge(storage_proofs);
            source_evm_proof.storage_proof = vec![(bsc_host.0.to_vec(), storage_proof.into_nodes().into_iter().collect())].into_iter().collect();
            claim_proof.source_proof.proof = source_evm_proof.encode();
        }

        {
            let mut storage_proofs = vec![];
            for (_, proof) in dest_evm_proof.storage_proof.clone() {
                storage_proofs.push(StorageProof::new(proof))
            }

            let storage_proof = StorageProof::merge(storage_proofs);
            dest_evm_proof.storage_proof = vec![(op_host.0.to_vec(), storage_proof.into_nodes().into_iter().collect())].into_iter().collect();
            claim_proof.dest_proof.proof = dest_evm_proof.encode();
        }

        dbg!(claim_proof.encode().len());

        let host = Host::<Test>::default();
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
            ismp_contract_addresses: vec![(StateMachine::Ethereum(Ethereum::Optimism), op_host)].into_iter().collect(),
            l2_oracle_address: Default::default(),
            rollup_core_address: Default::default(),
            dispute_factory_address: Default::default(),
        };
        host.store_consensus_state(claim_proof.source_proof.height.id.consensus_state_id, bsc_consensus_state.encode()).unwrap();
        host.store_consensus_state(claim_proof.dest_proof.height.id.consensus_state_id, sync_committee_consensus_state.encode()).unwrap();

        host.store_consensus_state_id(claim_proof.source_proof.height.id.consensus_state_id, BSC_CONSENSUS_ID)
            .unwrap();

        host.store_consensus_state_id(claim_proof.dest_proof.height.id.consensus_state_id, BEACON_CONSENSUS_ID)
            .unwrap();

        host.store_unbonding_period(claim_proof.source_proof.height.id.consensus_state_id, 10_000_000_000).unwrap();

        host.store_challenge_period(claim_proof.source_proof.height.id.consensus_state_id, 0).unwrap();

        host.store_unbonding_period(claim_proof.dest_proof.height.id.consensus_state_id, 10_000_000_000).unwrap();

        host.store_challenge_period(claim_proof.dest_proof.height.id.consensus_state_id, 0).unwrap();

        pallet_ismp_relayer::Pallet::<Test>::accumulate_fees(RuntimeOrigin::none(), claim_proof.clone()).unwrap();

        assert_eq!(
            pallet_ismp_relayer::Fees::<Test>::get(StateMachine::Bsc, vec![125, 114, 152, 63, 237, 193, 243, 50, 229, 80, 6, 254, 162, 162, 175, 193, 72, 246, 97, 66]),
            U256::from(50_000_000_000_000_000_000u128)
        );

        assert!(pallet_ismp_relayer::Pallet::<Test>::accumulate_fees(RuntimeOrigin::none(), claim_proof.clone()).is_err());
    })
}

#[test]
fn test_evm_accumulate_fees_with_zero_fee_values() {
    let mut ext = new_test_ext();
    ext.execute_with(|| {
        set_timestamp::<Test>(10_000_000_000);
        let bsc_root = H256::from_slice(&hex::decode("1f395eaae1db73f6213984c8a47b0e025a5fc47390aab06cc93144cac993defd").unwrap());
        let op_root = H256::from_slice(&hex::decode("f123c7969c1021781d4a3a2f9055786f309a051f14bc840789dc2a6c2713e501").unwrap());
        let claim_proof = hex::decode("180007b2b83b4667c5562ddb5b941d5ef659b9a81407902a42d0a3d497341795abcb00f20d6e3f79662bc47afdeaf3dfdfefaee4b726d6f6fb306c7988f9fdfc52b9390051e13b86c6665d646e66ef8f2c0b13a0add1d8443415003b504e52812d7c6c49005bc88369503fd0b86c3e5f0a710642b93e574682009c146ccdfdba5312707b07001f3247de3ebefb1c71549b3bd7f3361c67720e3d6294354f88ef863bbf3e29a20083b2b3074429a1d18398ac06cf3ada9f639b84dce885ac4f89f58e5f3f358b6406425343303e7b53020000000086770100205108f90211a0d2d43f0ef046d3bdce2b3c78c196dc8b102aa08d5a2eb70cb840606ef8bfcd8aa05283b6c455f1aa10ec65068118ff4565a6f2530237a6fa14dfc8ed312eb9f6a8a0ba6c398c4e495da344cb80b2306e0fe06315dd7adbf3dae960dd83b5603c5281a045bcf71080a5bc693cc7ccc5f08d9594b88a96e5d8f2f85ebc3f01e6481920dda0510355912b3705568dfa9de1307709462bab9c3c245f9dbed5a963b706f96626a00930c2d4d3b1146c9fc67dbaa211b9a8bfd415962dadc389235caa7f79364dcfa01ea5f6aa3d418ae67970fa1d94397809c6723d5fcce93face9b1464141fc90b8a0306b5c202c9b1135fe848bbfa4db7fd7227f17f0ea1ba7b8da7823e2b6a8de61a068c2cd86cba30b68a7d72e27ebfc16e80c8cba2a621daad90685f714675e61c7a0f12498f08565b5de5fec664948432a8595e65a23ed2ebb8d268b839c0037369aa01954b5874e80d5ad9fe732c0118c062b9ddf63054add22b3861e6bab829f4f1ca02c882dceb18caf08691cc5263b21945fd98404a2ccaa8c0213fbb437cb99f7e9a0e55c1c1b68fa55026c4091617027a33d75b49454355dc43f644da7660a881ad4a03df84125034f75a95ae7cc7ab4b24251f1fedb2f4bb038783790039b215f07b4a0503d0c41129b4473aa8d85f5e764655592acd4557a40dfe5b73f0538683ae418a02638aecf8df7319532ac650403458725a1001fe7e5df7dde07f1b836e8e45bc2805108f90211a045308778016b66e1959d0abd49df4f4ba6f2bc2923af167d0ad43d360341b29ea0eba15c709885c6c32ea0439431ae96f6c93bbd902b2c83ff1c339ced69af8c38a005b2fe3a6af225e709183c19746f348209d7f1bb3e604d4325d593c371e2ad10a02eb0a7f197735978c3c7c92dfabee561f96ae005af11d2300c841fc384d9e169a0f23ab4baae6c9738fe274840be1af122272a62f1ce7bb8fbaa8a25d938dc5085a0a51857eb76868d07c141428e658271f14ceafe74d7c9ca387ac052772c7cf03ca05ec719dcadedb3b0879f9d4402d9e39d7197056f91c8ba0a957e96a223f0d1f8a0687795fececcd10bb0f5defcc230a39998f9b03eb1bf45e1c0104958bd600225a09bc1ba97424b324fbcbd50c6c5f25cbcc05b16260e9630d148d6bbdd61eb5f4aa07c7679e201cde0ce6eb5e765594166c322ce1e4a4a7d5f91a5cd9913d2fe50a5a0203a1b64406358f80c03aa1d50f71c9170e0f880748de019edf5f487c490a4d7a0296e9c2d92526cd3a476cadda20c2a1b09f186ee4d83e38e49fab439d4e64b91a0944af4f15ee68bbc56f99ffda1041176d87876964fa54c02867dbe181bcd90eca09e1ec282dadd228022870dc2c2627ce0acc5dc1d1cef63243b3c6c728dbfd7d1a0b9896aad147bbcacdadf23b07f624ca55d2debdff570e750e2f988ffeee7238ea0aadfca805234d5af74995d7a56390331ebb6a55634b7712220847de43b7694c7805108f90211a0ba2113937900a951c7f05913d0cac090c417ff4f12ba43531666e1b7293e57cfa0ab089f597a7a564a1badd809dc7af1596e918f411f0187e5eda09ffbc505f94aa036c56aabe727d4d235a3bb0765da198110f9a73ca09c804dc2d2c6b8d65beb54a04062199833391ac6968c2954b2cce40372f3fd31a50c56610dd9918e39246811a05d30ea1b26c1661e7148df1b1cdf78ca9943ace9b1fce2bbebc711c051e77bd8a0f81852be33241d478ca6c8ce84b389fabb0480098bf86c0328d39aee9e3600d1a04473ea88e47fd9abfceeeb46273a3e4ad61b1a535e8f3a99fa3421ff816b74e4a08c07e8626bc22fbc837f11e7f9934559507a82f7396c2a52d6353eaea64ed43fa05c057d378f24e51da0d004827cea1b9dbb433f186c0e2b26085c82f9e14337f7a07475fe894b18a444bcbc065e2b8463cd0ce4914594595d52ce9d874a62d19d5ba0b7a3958432c5c3d1553ee74ef9dba30f54f6516ebf10fe2366f1de6b8d069ba6a006a115771e418ff7544e0235c123f329f2efa5c0de32c0c5ae3cf0ca55d10ea6a0dd35f81797ac844a7c40077233c8e92d2348c983331a61bdbf91a2baf5da758ba0cfc3134dc5c4857b3e1ea9b1de18a40ecced3faf02fb41e4b8394f7423eafc55a0b22ac72771842d37e55e233e5a634e2efd72f2645934fe70a461f4d7f79edb7fa03db3ec3d77aad163766f00803c6b378199d3521788f660e7574af4c328575e8e805108f90211a062d4202c4738e5ba1a6a8d297ff19ef321940e8a9d66951876d945be266f7497a0d1386a3e9579377607cbce5c4c8ddb738e2f96b25025ec9f9bb82593e68cad79a054b6b15dd348c80536cddbc9228ff59942d704eacffc028abacc3559e81fb64fa08acf6157586f40c3e9217a8206f46dc7d9b7aa4cb6636763df05f5a7ae84cc0ea0275fa06d6236625587891fe3db1df3e99317ba2395bd4cf9f6e8f99ffeafcb08a02d422b0460b96ed72732dcff611dba777c53ce313a672b5658065b2a6d7162a7a096125284d1738a4a46187daa5b92856fd7516a60ca29b776233d59aff007fe4aa0c7118fa6ea41b48e77d2798e14c31d063e282c7e0fe7e0d32686bf89ffd8ea16a0b1553003d313b939f383f8b3f11f04a9fcfb9a90fd6659c2ccf62e5fc93a596ba039dc552dc86c6817a410296e529c884877011ecfe1e2cb25fdfb5d059a2d2dd5a08c591157290f01b2d1ab4728c75176428fb62b5ecc7795f8ada2802187fd948ea0a4d00e872b7bdee8ad46d1591b591e247b688e8a701675247c8855a8cbe2012ea01822e81ec65d7f585fcf59eeca6bbff116ea6a1913e3db934501f69cf63a8828a03e52317901eb06c369ae44f7e2daad7c54649ad7ae14ccb020e23ee4fc2e194ea05395924d221a6fdb7cf6844ebabd19391894a55391e748d500ee3a5e66782272a0d67a89dd02e32d40dda4830099a79619a34a3a37272420f01ffdd40649991e6d805108f90211a0be59fb7819a59aa667223ea4bb2ab0e94e67f6c689b0285fa22f22ff0e04e333a0177bb5580399d5e1380e2e2149a72f53f75c06b2253550ae9a0b6aa0a6da8b05a0bebac55bea06b2b87d96b2b3458f7e5df8ff14ae940d7925928d0c4ec3a67c31a0131049e4f70540e59686a5fa70e9081ee7e5945da319ca4c9542a8ec7624e950a0c24b5c17a4a84087a078e550b6f84bee5cf7f83d7608493a84b6b94a80ac9be4a023dd290c0d41de809806de5928cdf18c364d6425dcb7fd02eb4313744a2d1c88a00c21f953d8b27655caf92c8cdd5167d84ba98cb3b0cbc3bb4ce12e69f9bae077a0bdf5cd6e9a180a12d2539847b8d945143b783f0888f7483fc3923966a527b284a054e008cebe2e71eb511cb202a370c13c36d5c9c524b0031de318f54475271641a070e6dd77e652a7bbd46e0c47d153185657485b1c286543d62489c4361431b8daa0fa1f294a356c569f4f0567547a8c9d0bf95b16c2683b6298c9fbadf419755d7da0cf75b1ae8373e149f115888a2699c9b2e1ee9f0c5768c89af1278762b2c483a9a06709bb667422ee2f02ee2d8178a325ba898b6fae6dd24c9bbeb672d2bb9f6ff1a0bfd0ec0c80d06bd1693eca142b5707e6b39802806079c514bb96f2a15c443e0ba0cae30ea58838aad9cd9357c397cf59ba6fd6fb4e394665ed5ccde5d5fa7159b1a03080afb567822bdb38ee75e2a8ce6de43847c3dcca717f51d0237e130ea0c713805107f901d1a05a0e57825895d122d522bb04e8b9e0535cc57d6e379c1d084991e4b83a4e240980a0b2c06cfdc9f3c067b2c3cdfc583f14022ab892b4cf3083ae28142d9355eacb4ba07112a502e48ddbe5acc3418a5f1f1015ce303f1d2c0d2badfeb02217a5838348a093a9f200fc8204f28fee3f55cfb36b407560582dcfb38fec397aceab45362c2da00e38d59ee4d2b60ae9d5179db3274fbf44a0c631d812c59475866e1a2a63cf91a012c78a9d91e1e28d51ff0a454f56af4cd7f83d68af04ab5cf0476ddf45bb3bb9a00d1f8b051607846e6106fddf421fd50e9a7a2226d34d00ba232e9bd5135b652da0dff7bf5274148bf61e2035fed1026a56b7874e73c13c0b4cc5d88e8b056b6964a079a8cfb8bd30dbec4f4d7eee59ec875942ea2fa5dd696d5f36291afefbe11228a0797a7ea729b220dc6b52564dc7ac6fa2309b38c4bae70c70d1138d0f2a2d086380a0980210c393641c4ef1f93362745619fbc48824c74fc82ce64c3dd495aef45383a086e51bbefd77b4093a49b3e3d3a8027ed696d2ac38421ac4bda97e231194d5bca0d32f272dae9f53d2302e912fdd5872659503222154931e7b3f3b852053699670a0453c157ac33c54d0642c869a75f7e7f815d2e418cd754679ae56368e8e280e5280cd01f87180a02f02f22c38a51f2530ab43be8cda6ce54aacddfdf1df59439ddc750b827ec36b8080a04bb105c0545d77fe148d39eee58873b6281661865479eeae35fb26f1c78b1c898080a052762be109384f91555a760464bf9691265692ab828513265e2fa11d1a026e64808080808080808080a101f8669d3004e7896e730c9e84bf879f86072f7ee1a93f0982bbdf44e9dd27d70bb846f8440180a074d8718c661009ccb9ed1ff451f84553b69c6c3ef9bf2c2b5066ec1ba9ab3b35a0741f21068efa81b2e4deb3b8f4e90222bbce73d7dd19685c0d0e857cc405539604504e5bbdd9fe89f54157ddb64b21ed4d1ca1cdf9a6f0a4e89e2023904c6f36f1f983c16910a9e6b560d404700955c82cd7d7c23fe2987388877f3f548ac2cbb0a4e89e347bf93c0d4974316bd936ea06f9dfcf2803e374ba631e8b795e9a8a2d7688877f3a79fb9b8324a4e89e3b88b68f2211d87b8af2faa15fe05f2dd8ef1358fb2328baffbc02ca19d588877f7fe6ae0cd704a4e89e3e58dd0f32cac583fdd6c5d3f21580bf0cabb277fcfd04f52b2d983ec1c688877f2aa7aa8f997ca4e89e3e5a1d9ca1b8f90d1274b6402365b27fac545a06c150735895e126019a2288877edff226dd1f3cd8f59e20970f51aa96e14201dc93bb632d9137df211631a64b7cafb98e72395e7f95949265b0d7eb94bfcbbe86f0a5e436249502a90418d8f59e20c2c5a4107b11e19c3b70353d18340be0264e127168be766a2c6ac8b1eb95940bb6876151475bf8168edb003986dceb61a1c025d8f59e30dcca63106674047cbb15af1b0b4071c56ff32db5924cea496679e40dfb95940bb6876151475bf8168edb003986dceb61a1c025d8f59e344a1b11ff43b5ee533d3976f9e32d9ac8366d2c2a438f025673f4ff7dde9594945b1a87bc8a149585e44abb450ae2eb818f94e7d8f59e373ee70425727138b4a06eaaf89c6d5844a1f8291c5c1dd2430bbdb21bd1959407e3fb4c9633311b018348abdc6cb151a75735c9d8f59e3f8c03624994201a264676e264d0e6a4c082950d3937a6509e453ec746c89594f1ee0cf36964246e7a30c154d6e914d0c6703d21dcf69f20a17631e65bb10e026badb7e8450c48d3bd02614d0acff31480bc7d06b3bb9594276b41950829e5a7b179ba03b758faae9a8d7c414d01f85180808080808080808080a0c9d809dff5c0db27004e6fb5b6b5fd54cb9d03abb8c7cc9e2ae434a64de281f1808080a0c0b11ba83261de8fb6fc8901a9b75e971c6a6c2c92cbaeaa398a0f40c90d538880804d01f85180808080808080a0f3c332dbe5245b583dda0d8a819a6afeb34fc69ebf10aba0a07b7a40c8bb5d1b808080808080a0d73293bceba9b7d9323ff5de5b1bc1ae787c4bf34d9aa07c8d00bed61af53fc480804d01f85180808080a0c1fd81a83dc214b7cfc4e8d6dc8c46036522f847eabb46d27cc9bbcdd833dc3780a0728ae3ea87b54186ac7ffeb6fdd2f04d075512ce6ef7dc0f1d99dd8835f8e8b2808080808080808080804d01f85180a06f887a8175ed2f352b961ff25d8ef8f300a9746d6f7765f4c70d87c9aef5260580808080a0e08b6e92f6edf715d329b8b04e61eaf24804b787271fa3839f886b786303eac680808080808080808080cd01f8718080808080a023db34061e2c50ca6fed3ccc774bb4e3e418e84e6bd2fbdeb56ccdd3dcd55fd6808080808080a05d9bb9ad616129371a14a13b8e698fa864e279618e182ef7749c935cec8297628080a068ebc0e6bda5ed575c4487dac1b289dceff7b9d4255fa25d2719e61ec212259180cd01f8718080a0eb6a23b21814cb1495ee490f80ca455c0c4036ece62a38ddb722eda7ab872b808080a020db32ecd57cf957015ec85f16d3ac3435fe43623b41cfecd7a7547af107a0bd8080a096283d06bf4768702744f9b45acb9b5abc8a33a79ce073ea492cb2b072808d7c8080808080808080cd01f871a0643d5a434a899b0a3b3a8f8bbcb91c596a51fe291bd6b272db4eeacb9839b31f8080808080a00898b199f9acd8c6eba5620a1bb9f5c78520ceeeca2c5651e8c643db2df737d08080808080a0da41d6244fbdb0938a0637dbffa8aebacc5b5f464e484fd4be6847d124fd9d96808080804d02f8918080808080808080808080a016e345e4720720c6ed41e52903912d51be9b3170091990855135ef96e6eddcc6a038a6b12a6b4e5b326e4a3eeabbac7ab83774e8653bc04a1e4ab4655f23aa4b67a0b7f114c652e8df8117169ddc7af54eee714fba31c1174f62d42a29ce7528de0080a05296feb6c9bcb48ff93de61ad3023ac5bef41bfe287fae629232d7c33aff171b804d02f891808080a0d224690747ae7b938966dfec4b1fc130d1a211eb58f8f597c9e29dd060ed4f5f80a0de0c84b391397b915f07caa05a6105eec06233848adc1049fe3266e94a0326b080808080a084f1cd708b85c62fe7e2566af48ae1e990d44176ef872bbc7d0399008f2e05af8080a064c459d927b734e027059b1bf73e925db2d3b9d3d102521670144e8b8d851eb6808080cd02f8b18080808080a03557578db93a6673c0d107db3540958e9e7d52e26c2bee5f2bc8f2feebb165948080a096b109d25566bcae60736078a52d075a89c1b4b76be61546cd443a7f9ba44e6ea008658848d75604aec0075bfc7f67ebac99f81fbd9cad90ee79842eadcbcc3ffda0cf4179d3f9a6498043adcf928b69a165ccdb757cd0860b9957d3fe2ab5252b7f8080a0d4a53286fd3ebefaf795c54cd2c042cd0ba53f0793cd32897651b1313444787e8080804d03f8d180808080a0173c0a72cdeb3cdef2d49f9c7132e926005ceeaf901a1d92b5fc2cc612a1eb13a064c1684ccc7f0df3ef5b0ba5efe17bc119eb8462bbbb9d92fbd3b2b2524b74b9a01b1aacda5bd31562923b0f00a07a0caeea3baea4316ae0e23b9cd580fd197bca8080a004c5be1436d46c79c60ed145e2d9adbd9ba572a5168514c80d5a122a7fb9acf78080a0b034fb36f1e60a78d0b90e77c86522351c80186f1ef4fd0a019480be17c6df2fa0d42c58826bd744a2099c34806e46aa2481e7d18999132800f43ec7a38f67238f8080804d03f8d18080a01358d119ab1e76cf70382887480a4748d387e45d210e7ca1f3a49fa32b429d5ea0d82adebf453ca912a8d2685aaf928323318644945ecc0aab8b5ea1ecccd81fd280a09bad1670b8fe32de90ef9e598fb6dd9eecafda48fab37fc51ccb7d2cd15f3df7a0b84997c3c31479adba28eebf2a16b531143097c4cdf558a51834a8aec9d8c56580a0cbdf582b13e6e3788c555bba9786b81506f3382cd61d13f4597658b256a229da808080a07898e003ae0f83112ee2d3620527af30f806bc470a3ce4d3eae5a45e843acf4a80808080cd03f8f18080a0f2affc119e65aee8eec302f759ef367e4bd4b2b733ea7ccecd28c39f32622b5780a0a81441cd7f237208ac8aa53824f899d16f84601d651829422e6d15cce026f8f98080a02f5ec022654306e660e8f221ed5ee6358da3d8461fb526308b23ea5721bc31778080a01f84d7ddafecf2e76fd4fdf237f0b95be11d389c2990c22954ed0f300c0a182ba0b09ecbadd0deccddfb9cf8e5bbdff6d28e1c29d6db056943582a824b7e51dd0ba08c1985960a5a8949fb9914f75f41611f38bea971e906e92345d4069ffade968ba0e71414361c3ddf24052989e6c51b8d4cbe04cfe7582dd76a270f4f77f1c2fe6d808080cd03f8f1a05aa3c666cc0fb6a500d3841b148216f1b8b22bcb64d1ecb728e56c7804a7a0b980a0962450373b3ff36485bab0bc40003bf37c8a9ea261e73e16f027df6e38f4c8f7a022de91baf3c943195a21fa18b09bc8f28b5a6f1fe72142121ecbb27c495ebec9a00bc426c306b752d652eb324aaf0ed5c5923c357a23dab53c33b77cd415b67f8d808080a09cb3648f0c60f580def09a9440e2c8c36f3cf626d60d8d8f493cefc4284b2b7a8080a0646e28ee5cfa5d9667b4402d1c2bcfd0a7cda32f889a1b314317913f8165c588808080a05ca8ea777c825e0ccdbb34ce72f3a61ab4b9b56ad072480232d03a38bbb50859805107f901d1a0bde392f89256caa495a32a3cc178012a6cf43fdd4b10a91e2fd73a052e6068dca06edec13ea23ccdd2c16de1b3f2c119577558cabd6e3225e90992451cd5752894a0cbe8a0754c59507481ea50ec37b2ae66668c7e98dd4b8431b7f559e0397f9701a063cd88c195972449eb51ba552cb24b76a71408f13ed5327fa80f1f27d5340297a0f2f89a4f2536c6588ee96c7a6a6cbcc95325609d84d0c57d40c79bdd7e86279da0b58bebe6f26d4f23407245c2c4857b44e5c779125443eeeef98a0df4490dc564a02be6e67e56b441d50955b497b594e43c8daffc21f9207139f98e2252d0c120c580a08f259cc7d888bdd6071edc4d908bde3fd8e594cdedbe7640e94dd4e6841509c8a0c19af9491e75c46abe72a5b440077b1c93d896a89d18ecfc6bb92e39c52ad010a0cb81ed702fe87bfd12fa14ddf25d00f588e51ce4fdd1f547c1663bd155060d5d80a0f2baf3a10a7c54f3b34c4fe0f45c51b25e12897fa670fa75f1ce2c204af5f1dda0822c1176590e26823be8c63e525a27bb9556eed41ea7a518269f195d92116e54a08e7a8dee130a14fa5f29a49fb6ab16d961a444d904cfd098d238d255f1e548bea064e8b120cda1fa718ad7d7cf8a1949ca95f0e90879073f3f386de9243a27bda3805107f901d1a0cc6e8457b591ac06afe62988d6bd9574a6196d8472590a8197dd73e77d4923fca0b526c4294046f608d5a31f58ace3057d35c943269cf0b8b944ede14fb2fb3306a0ddd4737008497e3c52efe4d5a7e0ef79f3a002fdaa5ed6c7e805bc5626aec198a0e4992992398eef8c1932fb2f0f2a80a0e5394a6b3c1bbbcf761c105574c5f400a0971ef6fc903b46e3d4e6aee3d6272c22bcdd4ed9ec8fb9da570f3b4f5d0167d0a055450cbf044c2174f1b3249306edd97c152d8b3a7afb16c3eb2f74415d47d22ea0698b72e48a819d4bbab4dfa862bad3da85f61330ab30072e5f083c47bb6bb122a0a4defea323e37761c726ff8c6b167bd8b2618db158bf39425371abd7b002215a80a0a040f4fbd49da0b1c3ad565c67e0c8b6525c76220dd6cecc9d030b5d81617c3b80a0a37ef19051d9cb3360f727bfd9c1fa6c758629fadb98de444146039bc935dba2a09dfefb298d30318ab34014a62ce373107555cac09e63aa813760e164e36625f3a035594dff91423db1d226d567e8ae2a093cf57e8f85921bbb6da84d5adf03ec21a00168467526fdb4e0c896c1604b011558f3d977a3e4863be0457b5b2ce16483dfa015020ef362fd3d36019a0b7eb3df769c2ffecab1496d450322b2566f20e8e63680d107f901f1a079664f5d8edd1ea83de3f21634084ceaacd0552e160948826d3462aac5d20ffb80a04fb89b4358e03188a234fb8534c697988258a0e8c0eca0c65f2a92c73bea6ec6a04610d824163e6c56c765487cea8f840f6ec9c13711279f37471cead062ce4f2ea0a75931d6f0681b02ef9aa5dd9ad1bc0e54b2239bcaaece327707597f3d83ce50a0f792119ee06ed45e0b7be21e028a2166e7426e27e56d7b15283da40970c7f828a0a11b77cb9722357422cf9e21b1423dac0741d844dc38a99c0912e1dc2f07f627a0aa1554a6a41bb0ab9951de300317207887de4f504692da1bccecf954c9b7a48ea04a12530a92405024467e1886e47bece01104abe6f0c199a83ca02426d4fba858a078fe6390af6e21cd0aca2ae69dd0810b1acde141df1ec914d75af1f44d692870a098f74ddd9da45c163824131414fab747a9421996db4e3947c44d792e229e495ba0b34382ef3d297681103159d4f267a1795c2e6434e179bbae27f37346aa90dd62a06ae941f156107344723143528121cf510f4b30ea3daac5f13313edf2049fae32a04d82286d81a816ae8391a46c306050a403c1fade57a93e9c37ee471995426d65a0ef4821aa52601c3cb2f171899006494c832f4534be02d0d7a53f30d505e5038fa0b4d0c0e3079d7338d6a9cca4afef3c56c8ac356d1ce6bd6007daffa39810841f80d107f901f1a0c00decabd3959ecad9d85c7fba3ba7226da575cbc12ed97ffe65409b0346229080a0d89932409ddb693572b463d4859bac40d68b08a82da79fbc2ba2e7434d3bb540a0b9699ddf7fdebf9f04d89be6f9d3dc090f9e15becb065b29716edbc1a119ef7fa0684b5d2b1664484a214d2bf7463af020a13f2955db4a8441fd73ece9635ac888a0138dbda7405f274dc4891560a339f31d541cd6ee5dfd2e2fb0713f6fe29f24b0a018078bab9edb1a538dd5ac16435e0483e1e23d9828da89df5e5cf12130dfa4dfa0ee981a8f12c39dd9d46848d0427b0484feba9675385903f43145c26083d4977ca02d27017072139bc1cf9e56945de36c8c2a0128471193c39206dbdfa53a971d3ea02e399b5aa0502807f26703d3cf11908388e6f38a24cc35975e9f00dd177a1978a01d52c8dfb85864405421479d146b96ef1ab3f36dd3fe6542593b742c069e41a7a0e81f8a2e5b80c763bc1664ee24d80640ad54c3c1bba62e77000219e206154780a0f4a7898ea4b5d67689ebb1a7a7f30cc7afaec54f28dbadc878c06da32abeaf11a08cd03f9d875703a197a3306e026cdafdd8581d149f0d48d743343b3aa4a48af8a0664c4625240beb973271a31778163476d63925fd25cfd407297a00acd2c3788ba009ca97e2f8c8070e03d7ba3265f34f41e599a71c84d186d52912818889f0e388805108f90211a0032b01199e62f4044775979f16cd51c0dd622209441b7d03b418919d6d1ccbc7a092e27728e545e62683192cf975d537675f7471f569590b4652fa61bee2b19e77a076121e7204acf04306425a14dd459c896fcfe0d113b290e12b5aeb136da7a200a0fc4068349536b5e31bb212bc552c797da3d1b9401f8aa0df7ca0153e05093b68a091508bb5bcbd7448dc9b33eeaede2a87a7b1963f2d32de4dc4284860d05f937aa09445b4cf7c4985150a04d4a3e57f1a6e25693e1baa15cf87480aa44340a7ca73a06351727ea993ed6cf2829ad6eb7cda573eb21a3f080dab82f932c7e0e2b4b70ea02bc5b3dfe2d73b647d26e74956de35f89c6e93b7ece502ae4e7c0105dcd16aa3a00ecf0e38540692f9266687cca60efb49ffda1dd2dc0abef0a07e0a51087e66c9a0deaaec1dcf170b20bb86f25f71ad36b21bd7931a5568e522ef19e42928789863a0f4e1ec38723c2d6cae2fe3b2d4bed286bf17b30f1d268d564b9a90cb59e1b6afa0db8f1455e45f54d97ddac90448444b49d446f9371234ed16cf14faed7a10e2dfa09b0e2fd7525ad23cc4d9176a6120fda14bd4b43835c533b726e90a55afb33ae0a09eb563bfd371b2da74e5a50796d853b4c699ea7d9b6b2b7bb3a889024744c1b3a0f0fb5fcaccd3616838d21c97855c7b9930eda264101805889461bb45d012cd77a05d7e879d7e8fe669487faef370be7da8613bc0380d31205828ddf9dfb81f1575805108f90211a003793f6a25ae05fb6b0d439a0dd84233ad6e9243a48767d5cedc69c8c2e85acda07e9bf4e481d273b563405b8ae551d9b24b070517354296c2d57fa381ddcc012aa08073d3cffa6b1de1c53f6d14c60024507ebdc9e2ba1022f386b1b2ab0c4e1c07a0253c19a59b453b446eef24dcf34324c572ddc1049aac109f58f56c3e89401542a0ab5f5e63773def39bf7ddccfe47d16ec62d60ef94b6ab4671a03898a60b206a9a03b77fa9a19dd6c9fa5dc1e0d0a41ebac0309587cc4f5980ebcdc8e1f322a51d1a02103f009cc72515ca89802dd0aeb9b07efdce76b0f03cde4f767b92205989f64a056b4f33a6c933d994edbd62cdd67863826e21d00a9bbde67abbd6eae0b75e2dba098d86bb66837657414458df4b4659b2cc524c951b8cd7050e7aa304b996b91a1a03888b872e5687bc8c49a1755ccbc4f687ea50bfd325b14ce44c5c3438c244727a023ef93b9a2f4db3f119b45e27746a25bc5b58d59d94c9189f5ab7409bec67698a00e279c72b9e9c65602fe97391428db03fe07ec317f93f747cb1625fb026b6981a0fe9cf2920299910292660c466b801b85e536feb4c38e49c44d2d4ffa8f2d0d08a0ce944e9488a2cfb6d729890c0e04b012f9b35a95c8b6b3bf614f4d5fe30036f8a0b85ed984faca69e9a5b179e3e243115cb97eeb642daf6f269fa3c85828113e7fa068ea919cb281841938cbf63460151c230e879e8604dcc69be14264944aeb340d805108f90211a004eac5d0d12a5d6e9899f4cdd45d146542860ee9de7235f3946e07fddb438b6ea0faab9d0a0e1ad044de14cc78c55495109c4ca89da1069dcd365fb6d9e6333e3ca06a53622014189fd444b6c0ef0f695f1db32d6f6d71613077204b4e19cc606a03a04167660397311d32da7a97219e0485b6369dee5b330d5c8ea981d84e7c13a096a002aec1d6a2b4cf39f4df8f3e394743a0889c4578142c70a325becf5771192f29a01ce111a4171cf86a5ee845b1891031b8ee9f6632b621039764cdc89c70d69aefa0edb5ec25ce40ff8150d7f0558b5d864ae302571c4d9cf5dcf23f7c5aff8b07a8a0e10debdcaa5d337ddcd4c4c1b8b8edab7b362e2e7baf8d3b20b6821f0f71d049a011341e934732c2030cacdc7407a1ad23467bb657ba01c39b89611e6bfee7d2b7a0416f466f906b7a7d87d4790909d0b15ea21307bc553a621f23f81540749b4ae6a0123715ce6d452abf3d520015840b4e940d2229485570ff43f224a0e5241dca74a0ebf4f098aeff4ab7c25d4dba023e161fd8009cdd13576b1b89134d35b12c86c4a00315cc342ff6063ef64dead4a55ba6c435f89ae8b5bc0a14963a129381bf900ea030d5b268415add9ad92ed1211faf66b4f92cb77691557f505d07a61e1c378309a00ab68d5c168e2d9e3de4484bebd9bb19feb627d304975b373bc60adb1091a2e2a0b07849bd7405031cf4a535455bd4498ae2c70728987faa3dd735f790d30623cd805108f90211a0067df11b0e3aed3184e2c0820a534b9b49f01bc49b93c9cacaf6cd20b2d99e88a0f68617bdd2e3fc7e9d2669f13223f677805ab83f1f78f5aaf25c71e053be06c9a08c3d33af91c3f7df008546ce1a1794d6cfb237d32140d6557f26a9a9739a1986a0876fd0f5ec24f7eb2fd9b38800528ad5e81df97da11b8e5fc6705330d5862dc6a0202a6c627f907cd4202bb91470d90c7833aba87805ead2b2f75ef6b88ccc3deaa044f81c102ddf3ebba8a701afca562e208720958c3ebd56aca250acf3c4354737a02cf93da13f3739a5f70c91cc2d2deeaac922e47c0965027ca6fe9a4bdbf25dd8a0ee4dc7d002851637da7df0a41f4cf321e1a365680475956495d969e36d285ba2a096ca2d803ba04b6b4c72196d52eb9f2ced1fe3cc559d7276ca6d47de56402141a08976d0a1a7b4cc81ccf3e722de5f7e65ce09b754923b9d0088f2cc53304e1faca0c2fbec69ca56ff5bb6b30816365200e6b1a7db19282aa8d48b3d426d572dd4a0a0fab5646e9ddaf1068601fafe2c10472e4d3defd57b88477748222053f15e8838a06a848455cc7de8a6a06cb57ba792b4873699fe469defc7a9247aee35629c6766a0c8de6b0f31a3f1cc4e2d39dabbf90e1d74ebde58413f18f7703b39dfbad6678ea03dbfab8427e662ca9dc51db66977dd97a7bf79ab6c9527eeb73b5c456951433aa022a0b0bbb9272cf88d7e7ebae0d4debf5494ec2a8bef5b41332d54b3ca02d0b7805108f90211a006c797b3d55c8000f5928b6a16e2100add0d1d7f9f94f1662c14e60f16d84100a037755fef39c9a68c22c08b84f19c43b43329c3952c6801bd8932491500550b19a009999b597f05d668668e6ab3cc1c9a56cceb2225aa100fdd29ac164f31834cd7a0a51e51f9cb66dd09f1f6ce3a5dc806cdf1a47cffd57bdaebd43adc1438b068c9a0d9c14e9469458775c613b2ed7576a3541830d26e30f10ae96409435c64ee10b1a0881a1b0bfd777b9bc8adf599330bbf439e3035c593fb7bab8b375783a3f4da89a0389dc971c788762adac8feb217a3c65c79b3b6bcef4b3b3c37b7f1a7465e0026a06e26d6d092b85e158c47712634c3675d725808f1d1d659e6ea89a2b42dcecbc2a00974ef8f06c687f1669ca08af6285a8d8f920038d67a67dca5983f9054062ba6a0492e69af7f0252a9edc8a7fa43c10cc93b42b51091af3689b15cd82ad537710da0128e647748b26fce99d61ce48480bb912c66cf40fda4eb243c4a8ab181dbe9d5a0bf69402a4b88310526cd1096f80f07acef41491c64c7be3e5830ac5eba635e4fa05683802d781485652963a9547a76c792edfa0eb783209283c9ab59edd2ca937aa0a691b268cb9958c4fb70a03bb8a00b7110ab405bf300b3de9b73aa00555be7c5a0daecc03f10ba0252f9116818d6fea5fe8fc84f883903050467cf0a95b62081a0a0e0937e862e1cd82106acaac13472658ebc6d8f41db5d4ef703a4fdecc2de3cdd805108f90211a0134d6db5078fc66c4b267101aa000caa5a70183bf6e25f82a0db08c1a219c071a071ba3d85a3f09c5589e5193901f464d5277a204ba549e1325b662acf8fde7c99a018e8aac2b08664cf4a65d8e5f97ae9b4978bfe16d3e1896f861518643854e999a0e4745da8012f27c1cafbec3e54429390871002518c49771a4153da986898611da0794fc0b125a3cd42e755cc5e23f7c8483838990849c3dee1e10a0bbcd8bca966a0e2cc1fdfac9822bb8d61d415e6a37933dbdc5f825e883451527d76dda519cda7a0872b51a5b258b79332ed4a1c8aea9b8fadaf27d2eb601ef09554e229be9cc326a0ce3e6f97a7a1c614f23d22d7e588ce150f4b227ee25b7ff57e2b30dd58067aeba0fe74f213c9450447c80015abfc9a0260330b87b56841850c300b7c87c94438e4a05487a07900193301d5143b577833028908e9c119e0e5ad4a036a4e51073dc050a025f25d05b6447ed4b668dbf53b5a09c5499184e3677ff2bb264edb9aafff2912a058546dbe5892d4a85778ab9f7c07f6af15c1f7b72c9453e486c4a0c232cc3c8ba01d5b20a04bb7d5127d1c0495b0d660497eed84b401130ed6e1268326ee56ad21a0630a34402b09fd50256f78876e1e2f39f995d46f370ae5f3cff170713d1d2e3ca04f2a236c645723fac37e097dea8765dca704fd77a8b8875ce611daecd780c6dca018fae6c6d8cb9418de20ee9520408ab6d4f34aa7ccc305606c572d7ad237f3fc805108f90211a0148bc06d0bc08062ba994da86bc14318f9fe8b0ed984bd0d023a3a74255927fea0a18a7170274a4759e6cd331fa930a3c8838e27c675a3e076e4dc3f9ed3514c15a0008f36ca673169f7f0e857fa5397ad413db7b2676b0e9f7cc750e92ef5d02245a027a7ba8450cdc81709465b6adda0f8cb877fbd953ae3f241ee37696ce444fc4ca0dff46239f775a2ec34be5bfeed9feaa91153381c8a763580bc65cf7b6e0605c7a0092481284b62d897a2194b147d38b6a1c1cf48b3d2c25bb4377ae4f32c7d8f30a0d168cc809516a44e58f7cc032e0695bb681da4762061be97475a58d686d12868a06b6bbb544e01690e1bd6cf5e15515b56ce9678a008e20a7d56d1a228a1569bf7a0ca194e2612cd18bfb616bcff360f958392b002fa64cfdc8f9fe64c745f85d142a06d39d12968ed68053c77283ebad019baa7b11a0ab4a3aea0b2a582c30b39e3eea0524aa06296ac3dd231ea8da1d066e857eec420f709b578138b5dd14ecdcfdef7a04f2482080a19abfd4e2b3784dd2ec2a47288b554973e696150a573674c7d2382a003d775ae57d2c6b7ade78aa0fe605e559825cab0795c458605c5a320d7075ed2a07faf22f950e39698646ff61b78cdd586117c785c8b010588129c6e7a147d03cfa0f0d40b29554640e60e5ca4b48ad9a62efabfd847ec1bd548fa2080d64478f71da0b4e005f12b39492890ab11a9a2c631e0b678d949c3390416328606851e03abc6805108f90211a0214ddaf4304d2d0ace7d66fa7ba4b91bf7d9e5f878004b114998c46c74592b59a091da251473d35ffbdc3d9d64a6af9039d4cc8ca047b9e787be86a76011012d1ea0b5f171b470a567567c66de9add272e6f721bb8c217c1ab479e5cf3474d66c6d6a06c72f0d9371a33ed1d5b0822406702076d0f188997571ff33544936b9d720509a03ca0a5e41ddbb8b622fbcd14e513d5b76a727b7576380e83423f2a51bc21d9aca081d52ef6d36831b16c17fc0de0e241ee1a0692667096f3c4abd93a587bab3f79a004f17c2cd6953bb9d968488004ebbd64322584516361a4b3210dd1382d685c48a0e7ff09af36d6417fb8b0b60da226b30cf9d07415ff557be5b2be2f38cd78db6fa0a20fa6177bda367a551e676d08cbdad1dc429d902bd43005f69722ba0b016d27a0efe5f3fe0937ce2a5d82c88c52e90869b76198bdfc5ed97dda9939dec082c10fa09d164d48de61058c3e21a7e99acc091b376b4a4ee167a47b2ce5e4217028922ea03c904be4f04871f6560899f309ddf50ff62b12bee1eb14f52301b6460794fc77a0f10b162fc9350b412363c0bae80188b6d77d6fe8993cb691f45c500af84a4c92a041b2dc22d8d854be548490937f6385a82c49d94096d55324244aea0e12725be0a03869527e09e26487cef2092c83abc4bc994bb17ce2455de3bb0df82ce12f52b4a04530ee2b51dc3aa84430d542253d88ec9d67a20fe2115bd57ce2b002dcfe7d6f805108f90211a027d7f8365e156c58e26541380a43281678c5dbe22982f938a9ee1187d72391b5a006c8fc8b75a4b1c01bb50e9112fca7a9c767ab9a2c38777bccc6ca2194a59029a0e60a1217af90c37c89b58459caa56f3bf5aef8d700ade01a2a5295f646c2fa4aa0201d94b158adba62e3d3413d5ecf0ab8a2d314f3f6346e923b6a4b86671d81c7a080154313e465d4d2f9ab23dd5a1048050e6972cd296c7ff2c7b92f9b10ed38bea0fc613d599c5667d8c9d5f8ebb103c08632f121cd7fb460844c946c517d7b3016a08cb653cfb9b2fbfc2d28b991eaeaaf2ef9d3f5065df235abe8651d6aae8fe1d1a0048d6c869ec7663a59df8cdd1aa8d50aa5b4de5dc6e0c0483fb1674f32f67b3ea018001df287bd1e01c320ff89abdcad237865c23597681422deb2d40ec6d9d88fa0d9974083ab2ebe0319ed2d9aa77404a55243be02bbe581b93d09eaab53e1f298a0ab922f013db543010e0ee0602d51418bcf440d3e8c41bf9011fee2affad656cba00f54eb3de3d25beca0a0b7448447f90383f861b75d7289330d90f1cc62dbae98a00b6cae12cb47a5b896b06199af6ba02534b23178b3107f0ee6d4c4f78484bb09a022029b0caff6162d129672f332deed4b44bff9c4c30a80640d6ad5c23fa17f05a00ee894ccecf4f070cff689f88938cc080ff8b3adb47752ad99c03455a2c529f4a0aa1e9ef9f2f8acfdf458456e68f5a4bcc124662280e6bbb11c76f02ec8062652805108f90211a04b0a619312032b33d4aadf7466b583ae538f47be26c991e9357473d3b94afe27a011b7f3cc70ccc40872e644f1278054229dcae5d6cbf5539fff6e954f927a3c96a064a3c465d102551161a202ba49c3aaaa0cde06598ac74bf47e1830c66eb4c3f5a05530b3f3fe25aed054f9fdbc02a90f04a112b640e365c5d219c8edbb8d30d0dea0259f2417ef576c32c570ae71f12752265a852c359d04fedb77872020fa519b25a0fea104b39b5cfbdb052282239c120269381f0844a727085695431274b03abe8aa0f94716b7e08386d35be536c2763ba0e5255e7e14fdfc9dbc948d230673434f3da064516be68787e50ad47f895b57358b92778f88505f7b77b8e5d77ec83f086b8ea03269deb447c3f931a421bfab885e2c4e7d600452f640c645c4d835763cc51c6aa0dc5853773af23d3203be74cf79756849f8fb35f74de2d55f98d4e9f42aa5ca85a06686194c6c0ee2dbe8dacfc1a70f769db6004557034779311684be2053fd9b78a04583ec4f821bbc97b8d041ef99d5634f6ac6ab42ce9a8e5fc1218690b7b9ac1ea0d17efe5f93a4a1734ba9e6b0aee7b55141aba0e526266db6752eea3ca710223ea040eb5d31edf1c08def9c3ebd57c6defe3139618abb99af9abc7ca0fd2bf6eb04a014ef49733278d7cda1d7467b7ef56297e56186b1c7f13eb5be66c28a7e1a5b62a06653e9c3c74f9aba532f49e92ff698f612e437ba8b95704a34bca1172bf2b1f7805108f90211a04cb6c5d3c95c5c1f5b941f94cebb9623b1e3687f61808f69ee8ff3f71feffcfaa0ae61a924f679fb6bb5272758b0cffa1fc01c34af2cfa43827a9d40442e7b6f98a0c7e8f862a86335278d5ea5ba4b12e0074c95949e7ba8d2fe9eaf3e4fc06a1838a012aa23b1a995b007b67ab7b45c6a7a53fd8eb14006ea232f55122ed8ccf96710a0ca172faf290634dbc2a150061b8103c34635393da974b6771dd2a67d535e088ba0acfc7583f7aed33c7cb2202ee4b5b48c2f8e612788eb1ded8f9a8df895b245c3a0b625e0ceaf86a2c3843530b47f4ffff1982836e5ea944041db6c8a11ef61e98ea08b8e9b8852653ed0debcb6d141065884aa24ef2eef3c795e372bc3737174da51a099c9a5f02e2f254d23c98624bc1212a2b02b9e697ef0651b785d7f2a4286cd32a011c2e605182090cb133ca9b22312ebd375e42944ed5ad276fd639d015a2a38daa00a0e301210fd8adaba39e6e8972260e486ec5e98f453a2a493e0cb2516029601a02185308016d0f5fee3f2a320f631ca1a6d88538f576a98f3f171e82e9cd72e81a087912346570786beb59c9842ba3660c9def7aa978a78fbe0339ce3ad63514463a043455c1b76b5b9e2a0d765296334fbdcfd94b97ec2f4ec82d28630e1b8f21fb3a02b2d78573042f377202d7755564d2a85ce1a309bf6d6112dbd615134eabb5b0da02997f3e997e0cb1f352f97e687ec06a2b9e0410e669b4f36ee8d240c14d98231805108f90211a04ce280f8352e5bd5d04c0f23689788a34d1df02719b557c2a44c891acd2dfda6a0f1dec98b8cd7b62be60a272fad35581ca368b59fc6b799683f030e9ffcedccb4a0ca8099b9a3377829ddbc4e5275009daa5314a91f1b167cab9a9c9cd7908ba8e3a0772863290dfc6bf241d3d0909f4f5850b820a13928b8bf429bf5ad4da24d9991a07d22fd87da536ad244e8b7b4f498fea17ecd7a77cba04ddbbe76f28e2e879557a0a4f0b2f86bab5e549a3ee90544a6f3c465b4ed94cfd3ae4b8fd19c6acb668412a090945315616d32dabdcf8fa668c33ef28a82030e35a3d8a6907c1c2205d9d883a09fa3fb85439c794e7e8b392fece489fe36f3e990d51f79892f7efbe6eb8f9874a015911a54dc274a9ccec9d003d71aef6a31b8cc7e3d9c66f745d8a2bd5411dc28a08bda99d1dd87206f462da3b92a63312ba8885e597a3520b9be9dcf4170e72b09a0d3518ad2bf8a97760bb9f1d1c67d4d9005e65ef049c333939693aed8e0467801a0010626b966b38148dec13acbbca55a5e40b9c31b0444a7a3c5610560101d0566a01b9266d0a56364bb699fe727dd52121cdb8478fc858c417c6148aaaffe145e44a081ab9d23d09856119cb249285619ed885b1cc2fe2ea302ae4709d876d6f089dda0abda5a0a95d47be5c66bf89cae8e17d1e1c33d402487c4aeb3f1349633cbe69ea04de7d31dbb8d2a0d8e4e0cdcfbdf7ce900cd6922a37cb0a5589d3f639030cc8c805108f90211a0561407694a9f0275ad0471ecd29901158c60a7d52b0d8e51524e601c02817708a0b122a68dc3963da6f54badb2a885498245dcd7dc019b15032ced58f457b8a2b6a006bc5ffb99151da3cf4edbf032c035e167db3a48f06090c05253cca51ca776eaa04a6c64c11c043e5056e7c9d334b651ef49f7cd8ebe7e61a3c493f9cd3d85649ea048df6b2b1b1764aa9f7404b5a4afd28c3bf3050c184b6828825c8569faacbe18a079edd9701167b12d0a6f08bedb7164f063ffa2ea9f9974552536867248335333a0090d949783640bf8c66b88fe08e64b20610f2479f85d7afc1ad7bbfe16616281a064b41a013bc1086089fc87edcf521472f93a992ebd868e3650218ef4a4ac1380a0869b85452427473664a77977dfc976a2553717d64d6cd9c1968d0327fd7d84e2a0a3ec887d08300c4cd112bcc0d8ad9f631213eb11087b42a288f0f8336f971315a07e7068e4e699e3cc5113fe8446bc89798be5c851ee3087e86be63ad9a35e1bd2a07a2f361571dea8abdf16d2aa1cf922af50695e31a8a4db6edb86f5c60267bdd3a07f5eb712efeedee2fc520fbfeb7a9074efeb9dc0169038d557cfd56280e21fe5a01e938f5c31f68629264eab92f26ae40949c0d1e75d58c815a8fe6f3016645330a0f0fa49d05168ecbd901da0cf9788e644d154aeb016434ec46debb0190a6012dba00fd3ea125153ecb43db5c351f34e1b2ec40149f9686aa60fac1ff874d34d65f4805108f90211a064edbc4566df5368ba5677505ff8bc5ada894ea08a86d10fa4dc5b18937d1b38a0d1ef71755a93ae2e8c799d712f92d89f44d3e3a1b11b0707419057ae52882785a03d0777a1d46444779c0bdb45ca14639464816e8188ffeaca3039191b5e18345ea0a2ee9cc8036e71f90aac1b7b58a0672b7a848015b70de84eca4ac7c9221f48aba083c61f94d35a9732ccf65880c06660eb3b6f0e080f6ea8a7715bfc09a3b0b8c1a05f92975cd01f882684654631f50fc3a68faa5beb8c297947de09b81b5ea06feba0d89bb1c0ecfaffffcd8a1ac5ba18b3a64a75f7f93d88ef61e03b0b55566371bfa0f97371f6e7fa168f57058f4140ff48977171a28f0438a81ae7e1a1f790db8caea03b052d3b3ecadf6c60d906337b5ea395bb34e50279e10d5989eb1f1842667a13a0eeeb7065109869f463f4c8f9424730789ede5f22e071b73678b1950b3bdbd84ca06636096175a01dddc3232be60a080d8fb3e192d9973936496bb57a228133b1eaa07adbe89e3f67866518b29eeebd9ae92b1c160655c92a9dacfe77b628aa4140b0a04a82900adc2648a61d8137fc613f6caa6424221e487cde3a1dfea81fc8048926a013d983351e36a0489d53f03ca0e4b8c6bef560a5fc63b7ec736708197bac22b3a09858b10863be5314c5e95263fcf6289080d75ee5a0469cb91b5945331634a042a0feb03666ee5b0d6219c3d3fb2ff535595a86d31f8364c71f467cfa41542e11a1805108f90211a06d4252f600e2938ed351a983a90a15048ee249492405e021f49683bfb08d540aa02815fd4134857a1113f24a21a5ccef4da3935a68bfaadb02933f1bfbe4d28bc8a05cecb72ebb7f1fbdd572fe074c0ebc803168f2bdfd8ed9eb5f12bd8351fcd532a000e78c3a3ca6dc2fe023ea7f46a42c0b82241f5f00c91ac9737a41532a8d3a42a0c33e0cd0f39a75abf307414b87b5887690e516e0661dbde6a9b5cb0c086615c7a0d6f7a002a4490a49d5850b0c746ba51b019da5d786e856933abc67365d506f72a06f19facc97eab7d5be239bada0930e0e3b58f9b4ca770e4c7914c24791a8bb35a06e4401429e142425e38cee1202b2b135c267fbde71182853300783e475687ac0a00a379220c8ab7a95ace18730aab4f548bdb7386dfe1574c213132edcad010be2a0ce489a1d2ef240b543a1cf2c4d9dcfefaf2ae2128de300770c475ea998f06acfa0fb6ba1b88cfac8be01ba701cd89537ff49ec7d0898a742bc2085afda368fb705a06e749b1af6ac1042a66ef2c123735c109dfcf3b61e0767890503787620139618a04ff4ba753b5ffa2e65da498406ad7a230b7254257da1ac8100e2a3d12584f16aa0554816aa0c2acf36b914adc887301223232afb545133d65b16f95d532f6ee941a0a07a5e915f7d628a0871924bc2ffc558c2daa07e0653588908cfe03fa38c878fa0a8448a2ca82ce014f3bca23e40db9ba2885cb2447f76107c26dceb889e26c8ce805108f90211a06f897d15a11e6e1c113bd482ccd5ca38be00161f4201b3d3bcd388d37c9eabe3a0ea7fa2ac8fa5cf0b43c4ad31327178a62a6a742fe8c21df26b5580f38a0fb742a027aec98d65aa42ffcda1e4cd003d222b640fcbf8042205145c7012bef954ce80a0a00333900641a2bce6005bca318bca48f93fe6900e3ec4c0caf4bcee9574cf18a09614b3236f28628ecbf207f87dcac45840a25db6ed56f68732fca71726ffd48aa048d7dae59ea73e3583ea080e10410e7c5cb76c63c1999c7fdcc11278427910c5a0feb3e9046b94ec28b679b111207ab93955b3640d993f5151e66dfdeb7b4e312ba088a65558872fbdacb2f5bdc3ef1136c231d43f1b17f9f4ac089ebf472a79348ca01f15dec68113c52fec170e0b7601cd76f5c6997d29938c907899d09bfb60588ea0f463401865b286c3e54626a96549cc2b718c9df110052a9bca8e9b669b286ff5a094e332ad2188a4091fea6171871905948d6d813c3ed11b0012bfdf539c8128e0a0600ad41dfc1134884ca0a64736b99c7c1c7cefda8a01e2e0beeceddaeaa15de3a0a00cbe2a4cb6867b0c21afb099566a031d83a77e41b79bc0f06236d51b83141ba07c1e66d1850839394c33e7f8e868b64878102ce2fec4367629ca5e9b6a319dada0900d25f6d61899bae68f5b6622d55ebe27cf06bcfc6bd1c0c592100d1588c376a077005bf920db19ce55dcfd66760dcb275acc458ef82ce61f037d5ec99508c184805108f90211a0731eb580d127657607cddeae7d48356a1095b7d9dc05e62734868d63699ba532a04ca4bce0633f8430f566feded88164c718f9f69b610db8ae8cc02f6d882f5f90a03f1836e3222e36d909f38a59fe9e7a8bff7bb83844752a1a78c215b829b7d298a0c9bd491b84fe2ae0c7a2ae65449d4d353d38acbfa8320b19f74413b0c6609dc2a026676773c9f32543011d26699d59e03499ba547501b730f2c5a47f0882b519cda03c651a33a513c3a1765d350cec5f7077de21e3d56105bb69fdc165c0481c1e6ba0382dfbcc051daf87ef7c7a23c98c2eaa2cb07dafe64b62299b3922cf8c6f2be5a05518468727074ef195fe69e90a62ee561ca04e3351e9093930edc0b83164cc3fa0ed5748057941fa524aa8c18db9d38a979b738214de4d2f233eb80a8e93ada6a3a0896c35095f9c35054e6a45be0791b8f366bea5a39cce2ba1ad9964f91be15070a031387671513f39973d2842d94782285769f54ea7a60ead367b5f93970a8bee3ba0bd0d3855da1ba1c0c5d83639b2660e0d5c5f623f3444ab501666e7e6af71f6e4a09f54e575a023414caaf9f06b01835e9e089d9926d260a5bd37a77625d08b45a5a0dc1739d28e7687ccc1468e4c9bd97ae2d12a1df92cdb90846a842e3e128cfb4fa0b3bf09c1fb79899dfa2894fb9ccac80c75d88300649b4968e064d6a1329aedf3a03d4b653be9434abf505d1760c0d1d6d2760871d82e3a23ff28e98c6813e5264d805108f90211a075f4488c5ea63f94d7603594bfd0132833b5ee1dd842ecbe9bb4058451afd542a0c156853b241944317554874c7d39a982067716d65408429bc544543fe18ced01a0e54f5b39b3a4dca65ed56c3bf0bd6fa7ae3629a01394086e554c8e9aa251c23ca0ce3d29962828c643545314d0c699c472aeaa0865e7d3c1a30830763194495c56a02dc46523ad028676640b93c0019d4fe2268343ac8250e8613379962c1e8b7958a00c923022285457cdcac657e949595bcde377ea365ab882528c4e078b1c782ffaa02d41f873a220887aad0409bf48c5f17aca57c48f5d6806e7fb9946e677f95c5ca0d93d374e7baecbd5959f38961f218d3200976c81f86acae4deda468faf50a545a02b7d67d954eb3f0d8ec179732f5019a87632f5cac9d68f6f76db5a935f206097a057784984048db1fe2f68b398345abda00f3531548f834c47bac22a8c71d11498a090a63d52ab853e646b1088a4411d6449a608bb27067e8f0a4d1d52a914e95b48a06a6a02ed37acb3a614c11aa86129049c7d4727fb305ecf0828b48df7833783fda007d6b4425307497eb350f10bc486622a913266866a1351abb4c02a70f2a62da8a0e5600cfcb8c76a2189a678f0db1a448a77a4953c444cbe04ac195bc95e606a5ea068c6f3565eae0df9c0a8bc6967aa2c1fec082f687d5d1f003c9f1130c218053fa059a084d094d5777991946c43897993aea9bc52bfbfb9a475096dec1ec44acae3805108f90211a078e7ea6d1535e38d3426d206aa50aabbabc540a4551b6382e3addf93803c842fa0fa552c53e704ef4301fd04473157edf834328c60df4817268f4b57273eff154ba07b8e0e237848d6953fd96608aa1e1e7831cea2f0182e2f75ecfeabf1e4678027a0f7a334c15105dc3d2d9bea93f713f3dae5d13bf7b48b0e7f66e707c5a5e65bcfa0fa6eb278f3652d90f2603fc7e57f05a8f88262394fd7de223082fb591e9b85e3a05143bc6ec83572657fab7131b9010ead562fa8a4436fc0aba571867f32bae429a0cc59e35abfb57ac467b8a9d972be4fd7bb7a2fa5afb0edfdae6193514d367c69a0034cde6e06f2b02a3abb47c6305669b412ec6730fbd1ff147bf9d1662092041fa01afa64348ee8bfd2a5e4c798f0226a138e25dc54d8c16df5475c776deaa64998a01811a79832ce9388a2cfa45cb17e0798b2160766adef8ccbe919dc9da06947eba063cf4e7b75cb81402c2ded14464b8e930a4c33003fd58d951b9bd5458c64ac0ea082e31f6d0a05cef609263e5fc1ee55e551e57741903e67762fe5f7a4a49c869ea02d390fe6b585f73a2a2d71302dd442f2d9a683d8e57b199ca677324bb6bc1173a01ae181efe2a1680e402079862417a8828a7a1481ab2023e25f0a008a62a62824a02e7703bcdc33ea120c78c4962396c4fda57b5f449dba5bb6894ed453dfb60217a02719fd14488775d82b5f7cc989f7ba5b980854e9a3d3636b27ed50c7676ba7a0805108f90211a079defde919b089e75ee634fe5e899b345d1646866edc4e24d33f18563a0ee1f2a07fa9e52f350d3b42de5c0b41eadf7573a7d10314e9bfbd1e22f15904a17a896fa0b33ff06d39d08656e2fa4ee8e129c0fed93a18af2cb8a622f437fa54989872e8a024a683655f02ba7fbb5f022377e2e67b5440da9286fd4957bbad41370006e4ffa0c45d7771d8e4ff7b60a5c55e5dc4da75ec6aa92b8d5800bb10160de4c0f3fa6ba0e97d5f3b5ee01557c8268f6eeeea4f240a288adc9b86603b8862b2e9b3166b37a0d38fe8b06d57f3ad4f524cd7b88fba19bfcde83c0b17b4e1622c91f986d0d735a0a874bdabf2c2c0a1611a37fd7af832a8af3f8aeccafd35a6238f136b42ad160da0c1dbe9e15010c0d5f9faba9639dc5213b7baabac400c7fb9c307f371ab952cf6a02155834589d220e7b8e24d7465f684894c0389c151fb94d11d1b3620bd277851a04f8352bd673cfa9fcfff95d9557369ed205d51146e7ba15345b21e098a2b885aa00c840f45963bfc1b0c5a9fbfbf03ed25494c4cbba052851b7aacd6ea5ed25ec5a05699e47c4de9a6241a3e93d796b13c511170288a72a179bbfbd59b2e13692e58a02e14ae74a87f7608668bff5296fd3b72f0f782a71c92737fdee657ba2818231da0fc5cbb53b9f0c2cbe84995fcded6d385ab806aa7b5b1bab1d55dd7918820aa9ea08c260cb378fe7dc302a57156dd35f6ee74861796c1987cfa000f71cfe1749240805108f90211a09f15080153c1bd8363f14d4f4abf941ad866d21f39d7d348223a5c0dcb599a27a0390c2ebb788dd02291a72ec7b17b2d0cd24ba335704c939f0a3e4f51d175397ea06207ac343f2686faecfb6d254d6a28511b53587dad0161f90718fd6674fdcc23a0c3741b65f24742a1c07f8551c4dcc5347833c70bf153b8df766441147d12477aa0e913ad85e72c8fb8a0ff758bffe7ef4f21570bf5873e50570e482b3d09e43a7ba0119f2edb6a116b92b406f6a1d6f0842bf6359daa8b8ab9f7061be8522b2eb0b8a06bc5d0b592ff4183389b412459c41d4b9e137353a72e3cee03b4fd10406ae464a0adf51e97849181e162ed3a72496fb9051e334d8066bcc25d0742661a301f9d95a05b06d14c13ce82cba7fb57c452a94f8155a21999bfc52aec0cde382569485db8a0632811ac7b6126ea5af049b44ca0ebe56beec2edb5cf37926ca43ffeae061ea8a033be4dd9c3b53bd308932a2dbd84688684ae986ed789b7d3a7a3af48da72db40a0ece7d64aa3f088714dac34a1c8feb1d64f42f582af22a492fa1e8898910033b9a054c105852fc92720f730d4b88c559550ae30e0107bb7a652044a8bec681b948da0b222eb30af75624cd5eefaabb952f2c281c32949739193eb61a1453f05af19efa0f7fb929770b568cd2c1db11fe61ba1bc499ba0fd79a452ad53cfc32663c89208a017d27de25e17e89406bf01e124dd1441f49291a13f1cfda9ae5f55ddcd2da2bd805108f90211a0a0836eb6c53d5a10a36e6bce753cc89ed68e70c1ecd2c2b4651ea0187b8ef494a0ca74cb06d53c8f59cca66a8cc6dd79c78b71ed7f174ae43ec2ec110940f545b4a0308408cca1e639accb0b62a6fe4ed019f327bdc4980169a0d69d78f5c2ce7fe2a04ba20b5c5da8901a83ff4a94fddfba5f4c354ca5d6e318638d213255b9c914afa052523880b5b0fedf3dc8844c7baa8c56573d55165528f1349fa994b3a13766baa0ff807194072f0737f5c507f9c956cbeffe232ff73f9c2da6f8b230f17a9f250aa029a8bb11602917611e3274f9a759802bf4b8fccdace849aeae8a49c9eb79674ea0df73f02a9b9b438b202af2b783ef6bbe9e5cd32a2306b9d2cc2604c318f0d3e6a0961176f28a0abca2bb93a969aeeb038876496f6d6e7a9d104664245230acfc68a06fc1bae3a0c99bf4e144cdb2fdcd2c43dd21659f75b79581aa53b9ea3fe34c82a0c521953df01bd4d3aa6cb6e63fd4a53749d720183ba2c2e41bee2b6bd3bf8d14a03620151636998a431bb31a7caed763885ce8973fc215bd6c7948fef955131946a0f1af0744d3d46156e7a3388098dd44ac19253f5b088b34227d9b44c9b01d914fa046b8ff40a1b449c6b7d1cfd74734ecc7ced89975cba567c78b4a5dd5a61c3787a083dd5d718ba6e73c11017b81e172fc086d719c6643aefae37cc29525f8859ee7a046d95024050dd97f5886de0396f49aef7e2562038e60113414e670a15b288aa7805108f90211a0a143a72543ddfb8a173a2bd7509f7793953cec9adf70e2b73f4aa246b690ad93a0dc702ed441d6026b7bcee566a2e5903c9178a9e90b58a40536256a66f49548f6a08853413f6c605bb6fb953f580a15c7def36fd89400eb292bc6441e4bd045899da0b220aae3f70f9766431f68691e88d6b5f372027945b97c6a0f86c6ce2c985f1fa09aca2a649c85cfed267697cf2c5d397888cccbd06910383b5add2662716b7b29a086cd1924ba6327a456195dfcdd3f2306acbf31c139f08eaa3d66d07cb45f3be3a0129d3765e6f7a1bf1625967cc74999fad4b637e94b5a41e2000adf4ae4cb82fea0ab9d5179a36302086e44ae77efce0a0dc12b91365f34f11ab156f8a5ee01ae31a0718f111fd71e6113b073f9df79c45fbb174454aac9c39b99118568ff2a6a057ea027324572cf3768dc74880c7259434364b90df4de246a7ea9c1bce769dc80bdb3a0dc510b9e4ff6fba839d0300ead9a140dd4dc08811e6eed948db5915ab0352c61a07b6e68cc53a8eda3ce32a337c4267b79828e0bf4df7b1d1f1893804b00bbe1f8a0784ac6307a23438a76b3c00a444eacc70671ebc1d7da0298477c172e9157ad96a0f8d0be0431e41cef0353da8cd312351f9e28057e78b0fdeb65da79bb39fdea8ea0b139288dcb7797c3f30124e7a30ab54d298ec1edae8ca0b9bc6c324cfae4c13fa063f5e03ad01d6392e74a5df0899eea516dc11509149897fbe7a33fbcf9a9025a805108f90211a0a4c855fe8770910e31844457d0ae12a352df92add9668459cda35027b21ed408a04246d2c17609ca0a7c910f82f9e0ea52aa7797dfa17ac59c06c03934880c0896a0ac29ca33dc9e9ae7f53997d7651c5011388d4728398b82f467c1f654d20eb1f7a0fe917af124ab06006264d315834fe962b3e5dd391995db50ee64dd241159d492a0c7f3bcff94a0e81ba86078e08f95697ca98317cc191862fac8e4b2d0240ab139a0236121988be6ddf1170675c6600cb3f2d306ecd089f0173d4dfadd9d6ef15a48a084c814cf0a81d9b84521874206e5506d5006daa86043678d5df0554f7a0c0ad2a00ade4e7733db64cf7b9b06aa77a32275555248b18f40d32725afdb72918ae49ea0d33e64ff4aff70b7c301cc09e7f9e69fdd60a6e6f5b672a0366388a2ff6a2222a08e4d954566fe95cfd958a8903b3e5abc39f94186b63f38fc1f5ff13a15ebda59a01eec0a469fb920392259f7c2fc4afd941cc34d854b8ab8ca5e07f2c8efc57ea6a091b4a5b09c3acab0ea62e5fc138d7b6ac344baaeda2cf41ad252c37954c20242a0a1ba643ef3b2336250375d6194b7b666e151344fd046857dba97eb3b97892ba2a0ff22666c6503c551da69cd606fb09b4d6456f1c8af3511205d8a0a06963b665ca03cae4d963e37c15b8e7d4738c7afed358f8ce24c6e3191811447b0e513983cb6a0cf7c8e04e76e8b7ff3e1337339c1fc67e461873d87cba46e60a888adc0cecf09805108f90211a0bf694e7b2704ef46a1405635903125b1c40aa5cdbcabb3f120ca7ac03e5812baa0ef1b971621844693a222cf4e1ef73f1d140f0207aab10624f9d7141f8a18dfdaa01bdc09995453fb076fc731b328cdea85459f30fa89fd0c1b4aee9fb2f1fc6086a03cce8ffd7a68849d6ce47ee9cafaacbdc9e53a75b95ea46e25b391669f1f644aa09e6e4b411281b5ab8d284f8553647c0c1dbc38342186aa408d1b4d3b55baf280a00c6c8ebd2a224de8499a297b2327f61fc8ec04eb8b712dbb627ad425296a5584a00a8ba4198a9a41cda60240554e7cd960105f0d3de19b98371bfb5c739ff2c0a4a0d4cb1255948be9a8b783dc995de2a9fc0f20baebb475d62831abe1f428682f98a0d3c1487c2f2a710d1e839ceb0c33cb89cd8d19fdd96fcc3b36b78931f5b3c4e1a0ee325f58f3549ea84c5cb9bdcaaab6d427acecd7946d5cae9f4781d326fb5d61a0d8516b98454cf2c5a1d590a6c373669255986b33eb23213435e477a166bf2801a03279f6a632a78e82ad536c520294889e32da4dc923404dc8fb1bc785f04cfd81a0199f67ddb6ea931209952b62357982e6536373336fc1ac0d2648d542136c7625a08ba14c570bb3f3c93e9e25582e4cdff04068c9863600ea1a2b3063a19be937fda0d69170ab0a16c41c8c81ca842217c47c4656703c1024c792297173e48e6a8a59a0caf13365de8806d8e535b43dce0527cc27f0f61a3a58226b273df42f7d68504c805108f90211a0c2c17ef2b9ec8405818d83472bdd0f2cfae5ec734812552954b2a92b0a21b6cfa03a2ef87ac08ab3eebb37d6142c77dd60c8c5995d6c7e8ca01b74f18bac592590a05ad8cc17bf68509a00f748c260bcf762526b86d05c2a323b2d4fb4753bdd9502a0f69aca4bd48e6e56a19cb23b93ccdbbd4be0f0192d5bff627e8bc175c813da26a09cc18bebbe82f675b6c9aefaa7c2e0ac9d319d605d0ed4d70ae3bc1fed8ae52da0e146bf0cc89e931ab032f081fa304551197ff515187627a20e4b2cc63bdf8e75a091d03bbb2d3dd9cb734b5d3e2419937ae42d277aa72959ee1fbcc309771f0345a04e711b935e0a23a3117dfec513257b6660f3e04ede97e0b18e2fc0361705a8caa020cdbb55afd0c51848b7c6b0221ec5304d15a94b0fc264f9cbc5aa60c9d41a43a0d07f5c8cf902e2fddd4ee8aa0c033e02eb5006aa26ff7a4c76534d9d8177018ba04624bad29294e9dbd8752760ddd687569ade8ec3141da590835345b53aeb9cf8a08584386aacd6dc76887f100d70ba8a8c3749d6b7746332eafc6ae3b52808298fa034d49549e48c4b7aa8bae411c363b5f4be154ac19a2c52386055639277b59f77a06dc2310e5830dccf6962a24c569fabc994988d38ce30f417181003b5b7a34ec0a0f373a21afdd6e210bc2239d68417e93060822a19fc24a770a5824ff22eeace1fa0805db73eed9b33d1f936ae7fe783b81b4cd4cbc4bddc5b730e8f6be18f308d2c805108f90211a0c66eac70377723da20392dbf2ffb139d215828a2699cca4dea85689d2afa15f2a092fb3b352e16e91c448521300965258e46ce91cb7a2001609976dee442e66f01a002fd63f55b29f52df51bed1ec2e961280bf516e50b6d2dabc486c846b21f5730a03847e6621ac00eec542b8c7ab592e4ec8e3159e877f30f38dc677812badee1cea0b9e6fceebe536ae068142821f8df5a41bb8a14da4e3dcab8ce0059f1ec08961da045d124e02a1728248b2c2e0385054c061939183c91112ff235fd47c33bf3bf53a08fd4e3cc1cbac83ea7ce569bceba696dcc55d82976b0dd1223b9844b61846c50a0d0cfd9b6d5f30dc6d4619859b57455560e1921f10e3be4e180f2c9cb28eff8caa0c5fb2127a13488e28ba347d8c9dd425d4995c46b8994d657f3d7557d6b349cbfa0f0b1b64044f16737e1453d57d8cd0e4c95307458b364db1f206af5bb66407d77a0894f04217d4736def1039c8bce536f72f2d2351d727c5ce6924060803e72bc2da0283567504c734d7a5c77cffd1e9633a65665abb5741d77fb80fc59b77fddcf0da069d51f82b6ef3a4e7a661caa19390804b334242074dcda0d1539f69f5c35e79fa0e0969af2e954a3fd95b0bbb208d048110af55ae509bbe64a1a73d4ff25d0df86a0ffd67e21e9348b932614d8965c9538ed4129f3d52c937573ed908f4f9b4d82dea0933012bb7fcc62d9b336afc05197c39c42f734d0454ca1addb085d1bedece6d4805108f90211a0efc044b947642afe134264a12d93b1190f346ee7841fd8f0cf4dc565419d9daaa0415c527da726e54fba06d3e6f60ea35f82e87f6030d44baf96dce74046773bc1a01c3f541764301c79d4f172a0ccf312a5bdd2f84b515ba835b8c9b9116a9767e5a07f200fc50f5b8c9e99dbdc5ba3dfd673f9581c213e95e84a0a63b176afa6df50a062f587a5279d313f6b948c8095fcd425efc851e83b113243bff929df2cce194ea0032a60beab9ee3968bc48bf2bff604358104b23781aa84c572f4c87207b49f19a00e6ba74b10cf5979c9833dd3b4c119ac311da430112e04c593082e608f6a4109a0d94d4b1aff9a781d1b67f9a5a368b4e56a99f4a2da484c6ee2035e3600ceab83a041873b216cdc92b0d73faaff3aa0915a4313b208724c61867724b401b974651ba0bebe159cef63cd572868e227d72624724e8aa74f3f644f4cfc14eedd3ee662e1a0635f16e465174ec6a2d0f8c571f8522476dfd09712994bc06935948872d0d833a0ec95234095baafbe1f08097732a2ce443baaf553af6d7794fa830e38541f7504a09c1dd7a174e09503d3050f5e96e51d9c67ce3e0d4151ca4335ee27258ff4fbeba086a406f97145d65f38e1b8f0d1f03364d07e15fc8a58afbe7927dec71fcf7374a097200c23dfbdb0abc67cd470eeea37e38c7afa17fced010bf6db6da84d057d2fa0213b34ff77d113478f1c0fa2b0f9c5c5d85a39dfb848dad7ef51cdc5387b42c2805108f90211a0f230ede0779735af6db8d98b098eb04a11f27be6abdabca7e6ec4ae0508da6f7a0f8fa585e4d0f68bdec9a9f44bd6ad86ff42e1ff1169daa407f52e79363b7f83ca002bff38826678d3dc768df9e3e939d93568f23488aab17b6d98196c4c9a8e822a0bfa196b50f429b099605cc3ea023a185847da9b504854050532a9bf81fce630da047d48ba9ad37aa496b11798880ff2ed2d7e8749af43cca48b2a1603f19cf83dca00f1ce27963c3b62d104a8e63f5b3e3ea8e00715c71fc8b4c86571bff0cdd9171a0b0b80bcded81ae28ddf79362837d9ff705d0e0a9926cda2766baeb9b39edbe20a0a5ce0ae5a9740ea10a3e603c1f3b977389c2f80d6d25001f0b70ac22adc10959a0f000cff88d4773907093d5f6c84779e263ed997c9c5ef286763ceb6a02bacf8fa0da16381d9efbf448ce634b5723ae33e3e2fc816206b5b6d27f44294f3cd78270a0764832e518b5498124fe56ac8fa067820931d2ca36b583af43a6029b1b9b8129a0ec6f6327c9b827f58b331fb585473de592525cf4f4c0dcd56e24f266d91284e0a080bcccb73cd855397554b122fdee4a7e9cb1aee61497771c3a12dbb3b532dd69a019baab21a3f328fdd20ad775cf6291a440128fd4fc61fc3498066971e2674393a0e19f7662cadcd8ca18e420bb70ad7a089037d1dcef46413c46ceca534f5056f8a079a20e7b936661793c742fbdca028e3593d4fd4fac2e6617b9e0237f89aa8e92805108f90211a0f60ca2545d3d634084957a0de076a6f1f046193dad4ef07528362423328695eba09c03b16568310c33ac518b1a7f3d57888e0efa8c84ed87925a460c2ac4fc67c4a0e849a29c374303771bae9e8d7d9b4e0e9e8230372c2a089f9ba7e77b9d8d86cba074003144187a5b826c7c1397489d654d53d34b47656ce566d835f2ee35dc0a05a0c7050863605dc5d7d99d675f60978628735b3fb7470467bfee7a5bb3e01ffedea06acea389d8e307f4a804861be2f81e06dee73fd69fc7922ab289cfe3c76a51b4a021dd6c5ea464c97967eac41704fa12fe91a534d577473819a0aaa05dbcae80bda0096712506bfcd4438f8d5f4ac8605682e0c42a6b1f19210e1f93b5b0cf2a42c9a004d9740a4b58e066e34bb73b51651fe1376f1ea34aedf21729f78b9dde93e1c1a0fbcaae181e74d8129b748e8b566edb6a276cb283c80300c7d4a4900e76425228a05842918813bb14e874c7aa37fc16072b01065037f936be5c75e7832d7b247efba01717e556504be7d9e12f748ccccf9d47dbb264f08de97aa70ec9cd373f6a8836a0aa3bcb12bc7818e424a5bb5e04abfcc3dac6fbca9b62da18141dd9873557bc69a0f1437b65eb0b0a2286622f7808f2d5f16c4e2166a79fceb5bd75b635500afffda055f8f88e1543f75f828d824f08182f6050937d6688d8b0b1a8b7dba038e70818a0c68cfb53ab7a4771b44f376c372b98cfe14f9d36def3dd0e4e9816b3a7ced3868000034554483030377a0000000000edd01c5108f90211a0d66dc386f5607514117b897a7b48e229003d1e5f307428a1d320d4620612957da05c446fa8ba089fb0e0177bbe37c1e98222b390819ec450848707cd2979b216eca0ae1659ce984542de88ae154d2079f4e7cb11f51cbb2111a515bc14697212670fa022e424547ad2ebb2f6224d496b9697ab3cea6428c3fe20c3c8f370e8f092d9c2a031cdf413607b9c065be7778cd13745e144fbccf8f7d0778b3bb1f314a2eb590aa054e9d306cd3d7942ec3f9003c0f419d18436d99284d0c3edfca95e2fbe46c275a093d6f4d14e44b12c5585e3ae0101c47d903e5a9be6fd7ac9ca9babca85bb795fa068eb59e85e6fbe1138509306231be432af49087abaad6e0b3d2e201affff28b1a0779bf8471c63016ecaeb7d0e2f4d4612b780a144a526136c85985917b91f4d9aa0ec3cd137a542c7c44d19470f01a11ecf13026ddc894aab9679cd0cffd4f653c2a0b4ca88ae868f4033dee200f60476c5dab809bfb598bcd72b2cda0bd7a61e0899a07f90a8d517cabc0a0b093cda1d317fb3fd6b0e070a0b03daa1fc7e1c1f645051a0fb4b0aa155ee259e169b37491666879c7ba408fb6a798d59c585dbd887046ec7a0cf3340415193a7b3afc8767d797934eec7490623750899e156d3c8e7b847bb7ca0b9c80757c8824cd2c72388d06dfde6c14342ad8a5d5d78f0e1f57c05d2cb264ea038efdc16b1ab523be33dfe81c45f3f3885034a75da81754490100a06a8cf9581805108f90211a0bb8fe128ced4692ce30b70acac794694d1454a5ca0822230130d250b960135b5a02317e3c027206175c6ac3e4e90b8673b1c269bd5a241c80049ae6b80d4b7f0daa0ae35ea6ae3cfc0ff1e09adfd64cc107feeb7ae1f366f639fadf97c7223ed2e5ea008e090a170cfc53a01d084f8d0fe5631eb2d9f296c7fb4b17577930d5f9ef760a09525a3a8144f8b1c24f7cf3f67464d99b694f3143d3a25934f78671d74e8fadca0e8a69cc9263e06c04efa3be8bac2008e6cf585e9107a0060ee20803bef335f66a0c958e335fab52f115559f8fcf48d592825dba0ac4adee93e293b0a67564dd1a9a021ba4dcc9891b2fede7792170ccd32df0b35589b3797932a748f95c20535718aa0f2eec0fb5a2774e28ef018f83e7516535ecb6ac51b746ddd207e2c17862efe84a08dfdf3373ca92e08583252ede791d059bcb2e85e180ee2e3b003feb3dc17855ca0150eae324c2e68a1c56a8fb90018160bac253bdf47d19afd98d8e3931538a3c6a0ad0e74895d86ad752e4af90848ada701d8c4aa84ae8ed33ed5dbdbf88a2d97c1a0ef51e96f21f788d114735753e17b83d98df059224da264a155a81a50c8c9e389a0afaae8f8d463a9b2720216e4be39d1be6bebc314b605804cb45af396bb04eecda0d5d7a22def17171e5746a4f2977fa383d73f91982fb08368528adbbd4e0d2bb9a0ec581e0d9efb80364dd7f63abeb27205f43040cba1d4dc56601789ebe88ad38f805108f90211a0b895bc0ee7dc8b53f90823366d3b2a9a83e31d47f59d43e20bd7d1eaa7d3b8e5a016a15b947142aa5dc81bd709c4e0c0fc2e8a5a663fed8feb059e35e41c8411bda0a1f5f659bc2460ea36b0cc3dcd6d778401fa4824c1adf325a1b60efbb123afc7a00ba7c5898124dc1c618ba587b61512bf8f73d7d0d60e0aa38b87812946acef14a0caaa93efb7fc4e8e324670ca047cd78185351fe8a4fe31bce501ae4ec15be6cfa0a36208805bc06433b9763d76b287beadb33036c28c606b65441e69defef62f29a0790cb5d2a9bde42ccf605b6b5f5e2d1dca4e57362fbd40a649424f237ae39748a06631132bfcb14bf5116fc1550cda466aa6e2dd422d245f135e85055603b24f3ea0e4d6465b81509e7d7dc2326f589ea7e4d0c0f17a9b02ffc1f5a0e2aa8b8740dfa0fcac506ddc8adbd20a6cc84218a3c32c5baaec9dcb695e793ebe3ece3139605da09a72c304720797ec8bd1ecdddc0599ebcef6b0395d1e9874dd7cc660c2c1a4aba011c6d3122e97acabd663168a882b177c0d59855fc71ffc324b4b3889f2cdef24a0a298fe6229df0ee82f9e3a1f739f6ae3e8a11aaa04fc2ec964cb840b8990ba0da0b32bd79983918db17362b58655ec9197f245fcb8c5a282f9739f91609530a9cea09ce4a44f5d6ce6edb6aea97769e2064e5c16761d9f10b1d36179e1b3b36d22d6a049f4f0759d7406172100cb9e5d732dc8fd1cd2c718fb9c0739ef4b28163dfbd0805108f90211a0301ec6fa46c204421a9b8f1d795b6e8f0006c840d53722fd3085e148428a28fba0d8bf5b5ec93a591ff5756de620350ff3a65c16bb3a5f5b2027b1f6ceccfc9138a0787b1ff5328f7145d3c6a06a4c6cd1c6f302c6d26f4069238605ca0cde0ed069a007811043aca13ae3c548825754f6dc7f7e3a4fb8cf0017a6654f41308d4cf4aca07243698b707365d467c77959b89ba4ac87b4c6d74068ee3522d25ee57bc5ebc6a09c467d192143a433dfcbb0409f38813c15c693cf1e7cd350087effa8917e4650a09aaa8ad9048a56b0ed5927a6c6cceaa147f466eb9873715024d18dec4a7402bba0fc65a7293f742d4311d7614b0a538bc942d396dbe23fc4085593cc26c0eabf11a08271c96cffbc759b8f048da9c6721ac5fb5a462b2b85ba3a23deae610c7addf9a07858e5de1214d5d275346806d9bf454f26d881c971a88ca1a3a9ac5a1e85fcd2a084d82856bfc10eaf9c66495e9845c0740f0f1e819b52468d84101757a346458ca038981e9188a849badbd85a3f63514d080f7f175b55cc8d7ddb88e3f862f107b8a00b7e54b2a973e7795f34116bb50c082487bd09bbce530c238b6201e9fd05b837a0cbeb12f0fbc7281715e06838cd968e018943dff0d14b92b4a4a6c86c27c7fd78a069ac106617cd6963dbc0253bd9cc5b2d369a8d494fc957c3b45d4ee160635713a0c248ddf2e65d4c130f1fa3fc7c98c854cf8248a2f12b14ba94d7d0bb9f48562280d105f90171a069f285a7e93a9d55afe2aa41d095f73a553ed404aabbe2984dc55b27691991df80a00e563bc3fbd6aa66634b24546ed39326706cf461e7b0f39227d895101591fb7aa0a9f8730c0bec80b6309992b1deda26e82cb0ee5b03aaa4240f61452b20f7b842a064c48e0442bf74428f07c8783f536b91c3d8573f0c303fcbc2ef3d2cb8358bc5a0122117aaf17b17289f7a08a8687ce5393718d2fc31c39706e9837f3c11cf53a5a05e6a7d2bfc9f0a65feebbacdce04985f054157b64147517ac3e23e088f17c9b38080a090372edaa395fe240d9a45f9255ea3035e1151e49425198472a8648f3c6f5c94a03a8ddf8d6fe0c612a7b595fd633b3f427e39b614f49980a2116eda5c3fbcf06ba0a5b8885d33deb5c61273745e5ea31e4a05674016e9dcbc57f40da6b4a30adb8080a002786bb5acaab5f03342abe205e540657553cfcb414b7ae9a1d4b358cbb36b59a091f6e0bf9cb18f6ac144655e15cd851786a06b7fd3fb58ab740e35a72774e37780804d01f8518080a06649c330142059b835a841c3b3440b31d024dd209fb01b54f54ea9010abb209b808080808080808080a0b2ba8232a141faf9a6d31d9e9e9ccaba53c73c597472ade2afd0d08e83dc8d7880808080a501f8679e2084f717ca94e44634698f1d26e6763a7e7cd08973c16cac1cb72edb8d84b846f8440180a02809fffbc2f7f15a481b4d28dc090dc41cf0e20c07d4c472dd0bec6d07a38c5da0018a8484fedb650f092a03150b7a6312f8ab60523863358b0fa9b3ae1f5d73bf04501d14e30e440b8dba9765108ec291b7b66f98fd0978d8f59e2094c663d3f756572a672dddead2ea5bb6482b12d3c74630915af45bcbfe95942fc5816c8e1f7289a011df977ffff054e0ebf6b2d8f59e34de1a84c81f19f197e9125037caa8115a8095bc1849eb55ac4bb29228df95942fc5816c8e1f7289a011df977ffff054e0ebf6b2d8f59e3acb0f52cf947645539e8ed9bd9d0ee353bb3a4e6e4180b666fa418c215895942fc5816c8e1f7289a011df977ffff054e0ebf6b2d8f59e3dc6351811d1f70a4e1d4889d3ad3132fc6613d3b25c168f245393ad4e4295942fc5816c8e1f7289a011df977ffff054e0ebf6b2d8f59e3efc2d0f1e34242134132c1dff865baa513b5605bfae98ebdb180882486c95942fc5816c8e1f7289a011df977ffff054e0ebf6b2dcf69f20f810651feeb1b256449e25ff2ff4a7806b63563b206dd402b6d88396ff3695942fc5816c8e1f7289a011df977ffff054e0ebf6b24d01f85180808080808080808080a047cda47c2694dd551a5f852ae33358cc17c9db12b38a1a37880adeff3bd32e0ca0e164eaaae1560c4432fad20020d07d721e947bf3e1039995e6ba71edd7dc2b1280808080804d01f85180808080808080a02390e0bd3a808ad8b90936d320a8a80d94b42a31a58a2aebd1d20b9743689d6380a0460be04e3def3ccf6efc0a3c6ec3d86329298824eb5264d29cce3c3208396577808080808080804d01f8518080808080a0600314d8616401d8967938b081d55a43f795a1de79a56c4a039258670f6cc81480808080a09e79d0fa3409fdd6307536f3b40081e02afbed80def475da294cd201efac772e8080808080804d02f891808080a060328a5a188d9a9ab2dd6ec4590f21882e4dac5358034f235cc7cb9a166e423e80a0ab54d659b562eb92982ced53328b3e3de5fba57500aeb93ad3bd8291123b414980808080808080a0860bdf74d87f33edcfade43377a874020975f255da63336184f017b4466b41d880a061cb8bc753bc313e807ee65198a722e76c80dc0ec81a97225444eba80389ce85804d02f89180a099eba4394374441707f5106822254c54fbe25c5a4a22e7027c3968641b439846a04098e3dfd417d504bfb789c09ff5e5e194f0797487fb3e4ef33c99015e3352038080808080a05067f9ad00d192b9c2ff0a20c2f05142a18c7a43e6db00e53f8d272dd1f3588880808080a0b9fdc33d7cd5a66c6d4946177c9285adcc70eb306903b5bb8cd8b1aab0a17bd6808080cd02f8b1a0476884a580c0f32cd1cdbd05385078213221b9633295993b96bcec32409b52f380a09d4622c11c450cd7206009dd555f5307b678affb62a0d619e0585666bfdcbedf8080a016fbad01bd6639bea93fbf1ea1bc24cee15513882f15d2a74c34977b67ba8f3380808080a086c9b512a3ff78b677662cccbe99abed4e5e08dbd0b9fe84e728cd786c83ede380808080a04f815deba265bfc24fb23fbe093200ba9bbd3389e62eda2ca0a800f93d21467280d107f901f1a064c4a4f13e9ebec06e7ebe40db9498b642ee57100987d2a7df5d7fefde091af5a09865a95e2d9af16652817ea9bb6da53b66602cdb86e3978f635cc915f23bf8e5a03012d132e5f24de9a671c7a632b869e535a5fed37fb240556c099524b60a8db9a03e5942c0d95897c49c4b733a8fea954d435da73fabb6027d2002c0d5ee48894180a088237e658a160bde942017010d5a37a1c88c1aac46191bee33553c18d7108ab1a0383e807af3c5718aa15981dce8b2bf8a79336b696b60ec5b3273681498b3455ea03b5eb176cba5712c389e85aea768b7abd04dcf4f0c533554e73ac83277c86f40a09030d4cc1e6f4db49a9d52a60bbc6aac428f61bea31d64d1e4a009944a841f57a0dfcf041fd59c8bb5ad1f869dff2944355d26d894c2bc9f2d31c4e379f1727d20a062d9e312cc73c799c6c085f45866f95bbf632311c427bb843cd21c5d75a14ceea047997fb5d8e3f0f2c9f488c92da0f241e813a9674d057237f9e8d3b2241c6a80a071f7764d27996f67b2d949a02ce79bd96bbb3e0c2cb8d2f461f4a6651d9949ffa07679fe3595763a526d45068f5ad8029ce996790d5ad00a4b40e791e088bda0c0a018fa6d12ee92671429445d1a237b6b0d92a6a76eafb561a1dea354573b73b280a054c69d763273b5108ba134945c3cc8c3ec1ba252bdc9a3de3af390fcdacc687d805108f90211a001be5e4960d8ccf9d153c6f09b4a88fcbd54054d709bf6a6c397d1ec6385e58ea008ae5226ea802aa9ae226b3abb36ebc78f71154ffe0ac77d14289a7d6602ad89a01421747e0b4b922231494c2737a7ec6d15c39213295ab0126cf02c1492abefb6a0a158abe3361bf2498fff6fa30aa9db8a755393b7502e8b90f6b9fb8f5883c362a02dd6db3a354253c6138ee49d9199460315780d17c633cceee1c0e886a0ef17dca0cc359442949d9c18b37123a10ff919b2a2daea290a4974d1002900704fb35a53a09c67d9b1db935899b0b37585710a986be1207bc13f52d266f0bf962a55d97504a0815e4268355987359c930e2d601108fdef4a0dfc88e1d4d02135dbadbd33ec1da0803968007a0daec9aff1d79b5a9872849b73494c9712090f28963343ad04e9eda036efd8d9370b0a27facb077b23dc1a5b1997ed88e95f24c902691a7eeb76f42ca0a59a3809accdc0b204ba3078ffe8aaf9996692d54c817a95362659a73e23e2dca0ef8ff32fc8cc5e4b810f3de6cefd6a852492739f5caac7ace4f7180699c3e700a004b137964f780c0cb36f48c3553872cf59f158aea18a5bb1994c4a60ce6fa5cba0cc4f21fd13617bafece49f8cf0020fb309871fc24737539574d3137369d12eb5a0edf6f88f024296ded114c90c637e4fd092351a12def9ca9a3b1d778ef2db511ba045d2433cd2fe3d9cf11920e0f81fbca5a37e19dcbc63c4113d2c47c2e9903d45805108f90211a00a5d34e439117f7fad22964c96baadcbc64599b470ff30b5bafde4060b5461fea04cda6931deb982a8d2022e124f3a078193e5c40e8e619bac7b34fca6063b4228a0309f21f1f3a6332fb2b6cca52635d6293aad4c4e7bf77ec13ae8a2b0b8d9b322a0082f7e5678429693045a0a86fa1a56f2f77ee997eae31cf54c70e4dbe25fd27ea04ed86ad423a5a00e53af4ebd069639e0930cacaf2765e7d46a8fbe429d8cb991a0c5e9959dedf9c879402da70efe7b8b5e7322695e2308e95e689b76c48d66a362a08db94b66eb916a95867f646aa56ed13eefd4bf08d5cf617f6a37ef187b9789f0a00591c1c550a43e43d818dbca30048821730b7b735f38a1934a64440c46151a9ea0c48bc93056bf4d98a279631d90a87e64008bdb657443c4a128ddf0c016c6ab26a08618974b6b91d548e8aafbc77c6995b522a47f9660615d47209b30b90a369f46a0879b8a01ea75909efe51c4c18d2170f2d5432df6617947cfafc1c818963737e6a049a56550fdfea3ba5a7cd2e98e38d69cd39f879ac46910368def57e6d5b52258a066e46cb23e1d387686fe56b40bd1036ad8e2758687cfab9f0aad5b4206dcb892a097a6872241454d74b069839a6d538e20b4ac600064ad4ad5bebf991ebc4c3314a0fc7be53c81923f12b51015ef12e8e75839f4e16cd20046b651eb372a257956b2a0a7535678173e24a3c4304d5a9abbb9e75ea5ddabe01eb75d4a7361510fdb15c0805108f90211a00e2b582737a73aa2a0d85439573e4c1384040a32a222e8f163d7de8c3ab040afa07c2f96f84f06b328d630d8d9ff389a45352ae0ba3faa3dacbff66aa4df107b1ea092babddc8af6e663101d20c7ff305bd2450fd823fc95f0a622d43fd5ac5b1b69a00b2336b7e262071a5f6922af24c7e418d521b4e6b44e4e2b8f35b66133b1bdf0a01b5dd0f1687f4d0ff55deb119f6376ce126a2d52d42bec29109e6eee419cb839a05aa5bd27a0d98bcc001c3f0d3015a046063ae99281f2495c40b6952aa06addcda02d3684563c864f7161795f2229b0196883e2e2af0aae0b2a52d5a38fce94db4ca0da8bdb35a4046c6935a4b7f1112f1f527fb45299fe574b3ed843724b99d8d9cda0e70e8fda70f9a00ff45bad4f538baa3414134ce39a6ac08cba810a8139191fdca0b665d1481035a833474c306997ed84996e8e22223f798d7f29ef476918a9862da075c5661ae27c2136ab1c5bdcaf163a1066f2623f8673b263264c01a017019260a09bcf47911c24b5ab24e11ba32ec5c31d2f64ccc12b71748654e3216e6e4e7adaa013c8999dc7df30a8d6cb6b3aababbfdc7f6f197d1cd7d983cf8e7e5a705e0664a09747f623c2889c725b245bf4013208cb4a7440009116e005ed10855c8c49c76ba01dda74318ce96b20c9ea6cf91747ec01ad8b1512de9c175e6119731c84ea6d5ba00419a75a95c7a7d3b6664213dd46baf986ab1869c6c2681c35c44297dd9cd316805108f90211a011a87174708548826a1ae7335c161990e1ac378af9c60895d966f116e2aebbcca06930ea6d643b44a564e3ada4130910f2a72cd3c6881415a2b46401325c504218a037e48d5da23d91b37aabf6ae0d6a7e15b1b3148c710e42e3078a0da3f2257573a0e59c2e314915cce12d03f09ab9b2e76c7df6901ba16fdf9cab4403119ecc30aca0b8df1feff049b21d8c8bc96f77161ff75c05b221d15f8b0f5063e912f2bdafe1a0104cce57479b6a3e58a72b2f680f4012445314e754eb03edc739a451259c7bada05f52955f20977f6e82c42b0b893a623bc9a0c9e0f55dab214a3f92997061f48ca08d758c24f9fbdc1fccdac9217c1abcabca528bf881e0c1a6262c8808fc3e7735a06cc6a1a63591029abccb40a03cb7d9a7a2b55af3e7299e674ae284f4200ca089a0f68064ca6637438635ec1e90ce101d154771094a5031e864dcf0b2b9c79d34e2a0bdc2d662321d90032d42115150073684791a7bf5c5550f8761fabbbb1c22406ca062f0b7ce80508736f11979309ea124e0e159e20dca43c8b1c410bd84dce16504a08ce6da1f7145fc81bba971388e120e5c03d218a311e80bb5b73d30250e9a5554a0e998b0344cbc9c17118ee75ab60507580531d9d383783a1cfc93431fd73a7522a0cab3f90230a8bd2026088b8c5cb42a97934bc5044cb4d3e04a6bfc10c0fdc8f0a09998e3c0a76178671e1e8e2c145eacdc45332691c971adb4dc6134eca14399d5805108f90211a02607ccee8720d413aedf85cf762792997afaee4ed65a4d8020d6bbff18394d6da048f1ffb82ca0561162f78bc046baf36077def0129a61b245be18b5d2087006e7a009a9a32968db9421816edaae94970e06abe577890e43bd8709736946f486570ea06eca0536c842f59b3b50ed569dc9a68d6aff0cc4a598dd73eefe6ff488d0d3b0a0e3b8dec1c0b975d2d3a09da064e706c9a3a0c30f4e8aba3f47a5aebf6690f60ba013d7b173a12147b622c1f1db82ce1a1676578419c4aa16fead89e13525ee92c8a04a0719cad965f0996588b6bb154a60aa6e69da7b1106add43b03b09a07c4cb11a02b3733a12c05146797d6b7b52a5153f213632e915b979b7b90c835f892c081baa055daac6dab6d4428fa7badb486cd0c0c111121ad50380c363a8451228be67262a0f6eae3fac924a91de7cf85afa961772a01a01e6eaffe818242e2c4a428530368a0d1212aecbc0d9470623160ea1bdb32ec08d1e20e4dabb03fd4bd6880a1cbe5c4a014d12e576548bad48a4efae14064377ccbb661ef07356471b292e5ac29a8f969a097b31da3b75349f69c29984c28f4c1446743c6396daa24812fe283fe2227a9daa012a0e472b70f8daf6757d3599c584f3df7cc8f98d0c459e9addce85806cdc721a05429d88f5a701bf5e26fa926fbe005814f77b14db988e35808678d43bbc6404ba015f37ebe783a9d49cb97aafc7f95335cf839d1f6781bd88ba6f923019b24ecca805108f90211a033dc06e8326ade9da3b0f92457a89f5f0f2a12df19e5ca3a26955c2ce968755ea04b48f47e6c26ba52378bfcf8b476619a53a1a06ad16380a0a66c765c3602002da07d068e35d2c434aea6ba57aaf8f3b6e8ce26ebc9065fdfec7252b4d0d6729f4ba01c46d2f63b83054fae6afca7691dd566495942539b2b218c5d18d2a4e9c7dfd6a0ba48ce14ae439dff1c8486453c12957b1b3ec158c833c7cd0c9bcf5fc782cc87a053f0b9d5829d7afbb160cb9157d53f6a348602cd233b60b941c4b07f469dd994a084f741a04f1572e30be11aacb971fd7275ce48165e5156447e83b30ce875636da0fda9a738000f1a9b1a874efa5c6e394d873f357f3bbd09ec52422360c0f794c0a0de99d09ae25392691075c12a225243931679ab836f745e8732cdbe94e7594598a01bc0fb39bb90c56bb7e0fbb785bd6831ca8ccaa9242d4233c3774c4cbe551b28a0a64a52e62a5258a45d6248dfdfed192d8e0f62edb613893324c0e608332915f6a00bea0ef516966bf839adf740a7b05be5192f04050fd396fff3dcf2c39119ddbaa084b5280dd8fbcf1250117446babcc76c2fa48636c4ccd78bc9a5d92f7559e38ba0a9d242193dd7cffb4b1f60a4c0dbbc55b1124ea576ec181bb09bea018b346fcca0482932c5f713a63b08f91b31db7bfad0ca2a0de7a95a9618dc550a29b330e053a0475a874fd30c68ed7912418e294284501424622a9ac8ee54fd13f207ea95298c805108f90211a03e8ef8086ddefdc2d2802dbc6a10bbbea870255747295cb640ef221ec1d0885da0af08a63b5e25f9ed2b161b2f938e486ac81c61f9900e11b41edc4de652b00d7ba02dccdafb0fdbc0443391a3f1d1fef4d11c3b86e713a7ef72e2663bff39c9f53da0eb6a0838ff558a94bcdf4fc53a58b00cf4cd432d9a754cb1840c5f243998b4bba029c4c6588e4eaf41bd1be3de120b796435a477c70612728f1d33bec2f7ff86fda0e4275cca635d3e3ea1fdc01ee71ca16a58efbafd76abc11cd33dec84e1b41d97a017f31fb603549d3c7721e39d6e3f34da53a4cde74b2a5eb6e19d3cff13e39ce6a0378667c3cc704a2d16d6ecb541d0bc3b70f122c258c337952f42b465bf00370aa05fa0899635c53ba981fbd973adf1619fbeb041b86dff11385f13d6676f307c5aa0d7d8412f56513256d28fbdead3a006fe23e704d3958263d3c56114905c5a9139a02702bfca25236b2c97eb5c1fbd7a6a5bb36cd9389b5efd80a27749f4bd56c332a0011e5491e3c177075b3b80ffc3274eb6c21766e390cbb050800f48b3ef3761e5a04f5f89f5da1fa9815129cd5558c6e4863d0a1a872a15567ea2c6cffdd2042274a055cecd8dcc6b78463b76575bc449e2614d9cc7b23c7f1f3db086b63be2bc0294a05f56dc6eef742292c0edbd8a189009372def625cf933732dffd69a6270cfdd5ba00b2d5b51d1bb272bee346a50f65b7ef972850b8b73924c7d9400acb7410e112e805108f90211a04b0b75d0c6dba86158cb12d23d371be231fb7d6d1b2c7adbc71b55cf4dd72cd8a007c79ae5c44caa5605b4dbf2e2a23a2e145e6b65e54e16380493682ea97ab4a5a0125018ff9ebb0cac08a14fe719c8de05f5ee68bff4b78e03f5277613f56500c7a01fe21e0447328df5a8640f39e9b062c024260a636536a679a1b154e79aeaa048a0bd3dffc416e609ae07c2d93356dbc2f78978395ba458d401711ede61acbb5b46a08435a9370884f19a4cc9657e43a93f36879494771acc243227b8c106cee16975a0c4b5e2ed4f7afe984861cf5ea174a6185f211d1e82d6ec1b198134a5c7118d55a01175e8c1d72f9a62cc954ec4764060d8d1b29c49e776b42761ccf0f3daf590d1a049c1c274771c1d8de8ea00eecbd6268703a79c4f17aa867ffd9d8ad25e95a97fa07117061442a57eda8bb62357dcbb33c184be69e8de82de14a6409b831931a6e7a0c1f800c245560fd56c9a3571106ff0bbf575974b1c4446400f730def5cc9374ba0d292148784b210afd9654dbc7884dd3c66182961b69aeb7da47eed64401e67e2a0e8d27100bf1e5600d66fd4afcb8b0904929e1f1828e66bc559582fbf65e07f7da0dd80ca201378ce3a2f07cdf4d4bd8beac3f2ee8b789d137241a18d1cf858ff84a07154078b4e0e4809ff81f88e3b1e2b01ef32002d2e34d6c8eba05ee138e408caa0f60aa5bd77addfb9dce7250642fdebf429029f5369a8c996a283efedb3f14491805108f90211a077d1dac50662da0d1a8410cf6b9c88cc28ba244d5c12ff825c362b932a273e0fa0c404ee6f736d2cd88e5910c38c005d58f0a5eb29fc4e93399cba0f5eeced8b2ea05dee36d47930b478246edbc1569bf05669cdb9e0ba0b33a93beed42471ab5b6da0054831398fbe02edcfdc12a416e0af03f45d81bce4bf9064f1f5f799da82da94a04cde6ec5f2560431bc546454b76d80cd1c7e07b55757c5fc0c3f0681b49a1c2aa0d3e1e322486f7c0cd3d257b361c8c801ab02f38de7d44af58de51d375fc68c8fa0242cef9bc6f1d759b7decc73e7e3402f682afb669fbe9176f30ff35f68f0af73a0732b1905a7661926ccc6ff9d17278b69705c185490aedbf90f88526c66e06cf7a04854410b10c8f44d3f0c32fd3eeb820e8e4e967814caff4d9eff66f6caadc250a0d1e79433a1794b6e2dd3d65d30714a0f8983fcb86eb54cfeca2f944cea4ce52fa0b5207a6f39288bd6d5c43bbc0251ea466cab7d58e7800299cd9e574d6d624da4a0c65c1b409dc0d212ab9974561c00aed156f8138008760636c1e6b39013f7a6a4a0c78dc35cb4e15b1149e494e2c5560460f2ba3a5b389582d111fb3078b9137c9ba0699bacdb50639532eef57c13ee29d52ae3d2548832030480e73c5b82a952fdfda04aeee84fe866bf78c0fc2b774701a9a6d7f19107de1ec78d7f059e7c73919a2ba0726c0c6a1c6136d5299633b42f6f8af377b8214dcc4d9601521a9213cb1bfc60805108f90211a07f3fd35a7a67f0fda46611b1bf4f53081d22fb6eef2b0602a6244b8e616e63a7a08601fd0da711ee84aad31fff62229b7e18f4d0ec72b452843dfe12728d24497ca0aac47d39cecc14a20ee00692a1ef471f1bb15061e5bc51634cfb1aede418ccb7a061edd128917fa3fe0e0c0a1b713cecde0f0697692927c8e6095c9255188033bfa0e01fe5ac06d17aa518265673c699a088cd7f1bec9a1d060bf27b6d9234809fe2a08392bd3885594caf9ff5940600adc20f351c6d5f6352a5fe2c71f68e2862a76aa01f0459151c3a76e6e6039cd889ffe64318ff7be0570b0d1f26bc9108d0f7aa4fa0c134f58dea26634469b459c05843b4dbfc3f828c62e8cf595606fe6650922feda04a18634843b582774f82274e3a47ac710625b28763a1393f97177af07b41d754a043a42913ecbca937f2337f9b4452a9fe7d70ca7e45b64dec1f13d61c290d4685a0de00447b27785b02e2b3d0b7d66e5c59cfb21067ed4011a385b2a69c86346a12a0641e24b51075e6eaca159b8a95f5165d1d312b2a4ae042067377177142b2a603a0c92e3109d01c8101e4d8504aa8e82611742987bdb94d66cfeb64f28e96d118fea0539ea24740e7b356a3b5ad94cfc2cc9bc7831ed8a57c430a1b85ac6ce15b17d2a0b6659ec49d9045bb50762bf253dde9951fb4dccf6dcb711924cf7e9477b67631a037cc72dbc4e05a034d43f71cdec5731c5ac9ddf673223332e2434c4cf2f0daa7805108f90211a0892c96fcf8d05342b3f3286b558b9bc3b9637cbcfd2f9b6bf3537e8c46fe8690a061e40f4114547ead89c73e7c643a04d1ea5bbf13dab9559a908307e945dd1910a063c89c4999314fded65ece4155db6d093a290e10a2bc4f28a4f750e4be0fb4b6a04eaa428b48c053eb802fe55d43eb3ccc650116f4db22f336fbbfbb9660de8778a0911ac52c7ad768a0b159dbbdda811dca1ed8a39747291592514e8b9df3534e85a0005a50a5905d73ab9b6607f542f4d8f551e2627c0d433fd97b5008ecebb7fae4a0ab32e33c943e29618910dcf3bfc8c35c295db97a97d6f0583a77e826f12bdb00a06430d8ed701a3049028125be5cd5d63b9f832adb8acf968bfebf2f0678cd555ea07feb19d846341868be89e604d929a7e72ee1885b2a72548c602eebe9f6571b30a03749a61b5c541a547006a1b511cb17fb6aecf669cdfd16836ec080cf89809639a004f473dabde5a53d96a949958e06dd303027fb5d99b4b2a1e591bf34ad680ee7a09dbab9c4278d1331bcd27955a26eb8d92bf74256b7abf18d7e16688129352027a0abe3980748d63ad11cc83871fbba8bef87893192fa2633ffb7f38d91c32d6d1ca0cc572bafeb01e21afa3fc63e52a3954f4770ef6b65f979f5f54517416cd64161a0eec3f56da3087488672165fa7425b0f020552929b6968ae91c3bc1eae398a116a0afdbb9a9f7dc31e4e7f9ec907f07420249f7235321a777a3b2322731ac6ca1ff805108f90211a089d713600ea2dc9778fe91cb144157b1157a5cbece9a6ff577c16498461248c3a0dc92ee0e57d8cda7f3d065abe2a29b09f1ba9c1eaadf8705dfdcd7bd768ddf21a065f9caf6bb24cbd1b713a0febb86e106de215cad1a825efeb240cbd39f3ab387a03651d1361310f881c6bde41ada7e9a792ea79067b79269cd478790821c03a6ada0d225d407d206a2fec2bb3cb005ac559e154bead66b7b55bdcfa8e47ef3ad62b8a0b5e52edc8dc9329f7f90f43fbe4b6d6b9794865f24689848e1b534ca6ee52aa3a043b572b0398650b08268a7747e8b11b5f59d6e25f8b1c9d5f6f9ad2b13dd1a5ea0a2a618a35d2014fbd5e06331deeb9a5e4da19d9a4c6c753fe7ce520560abce6ba0eab81a4a50faf35b4b679c4e6bf12661eff80a1b4f4b05ac9eeb9f9a3c4bad7fa02e52c3b8c7698ed2465fa5196143b066900311f187c6f0ca9a333999369ebdeaa019da5d1412ced184c426e6571ef26fd0aca566378519a708596842ddffb90e50a04c17905a5410f7aca815bfd71a742e706cd06984fe02bcafbd7bb6fbef541b52a0747a76350eaab2897439f20cf31014c33b6602d52081ad6accd23a3fca4cdfd8a05306743d6d6b74ecd8424538110ccd5f1ca5c98ae531b931908a5e9ef91dac42a00ef5a6102039906679cc8559a04b4e9f3d97918658a712d6882af88963f66bd6a02b030f71de2deb3ab7dcaf240750c42d4c73c540a9dd183ef1fd2d2e29b33cc7805108f90211a09e14b2163a8cdf48f875e4b7ff02def663b18236070728567d27e2eef29a689aa09548dfe007ed4d83350c0b63214670b51047eb37ba9d32b7067c2f0618b06137a058fd1103a7e10ab016e99624397bbda7b919507d39025f1260668db90b67606fa00b36932817b660a19009882de0d8bde8385d698408309ab3bf8fd96bac4ecbaea0fb153c1cf6d542ca19795ca081c76ec99bf825cd49495cc66eb8dd98c8c1a5c5a079961b39c0de0887fc75cd979cbfa552d1882ab3738de7efbbc80119fef6dcdfa05de59bb017eae11dd646f6bd3d858a0ef38bed7b757ffae915da979f5825b2bfa0a9d28e8d402f513b3b26fdc16c5590e4dce6d662adae8c57876335f9d30eb4a1a033f49b991191a74361f3b9c586aa574480f1fb87b0ba51523f883af9ad240a58a0400fc175cd14f7fbdd4ee189748fe0a68f81af5b9c72f2fc5bee77056d1f07f3a0ac811bca4c67ccea1177e2fb65f2bdea570db09fc1c211992baa906841765151a021e2d5f67d1dff26062ef0bbef893790aa3df92e0c3cd2874a780bca3019ce57a082a14ff58c58a891ec1929b0702b55f154a4edd2a56f83302b103e9876299864a080dd3b61a232395e84a1a9739dc4ada621777e629c081d6d2f75fae6eeb9b878a0465a712529dc8f9345c31dae851f21e2b17db0aae25e4f4955fdd3da7e7ac824a0d114317c1bbbed180b6321d89a715e0490e5d3925012db92e2d373426c85c611805108f90211a0addea2ad08de5d8cb6f1de2b1d2cb67f710d3af31bf280e8ad2bebacdd4e8849a010c8da330fc43afa1a52aea59647a04b4525730ddecdc0096c36d94a02da4949a0a1468b7327af19b92a953ad113d7205eac14bc6f6f9df9f67dc3e043aecaf7aba0640844e016f1676a9cf04b9f0e4465f5478b74dce256345f39e2644b6888398fa0d05993e7181e36a5883d748dcb3b4aa547129c8c6947003f0a8702b141d74d16a06c255ba21a7568c83ff96d7d1db19d9a74054b863df3635462bf11a4b654a625a08f154797799ba4894a1278969b9c98da31bdae8d173acc6c487648de1593f347a055791f7dc7d9e6a95c622b69832945d18349dc84a0b6e172955a9022bd68e595a07dc99e908e938ee75300c5e0e959ec13d0155644b4897985fb7fc465335ece9ba0a6348ffc83807a2bf1960557a409997970791ccb0beb6bf2118e195a0c3ec228a0e2d6db29521613fdb7de288a01f00fcdf601dc1b057a5ba2460df0e3f54a3228a0b080ca35ef6acc8108ba70434bd4b200fb8f6f2270e7ed8a0134a0175b8c01dda0022c9522aa4e3168418e8562034ac3e03e3fe6d40dea00bbd1f8ef05f93764daa05391af29c2fe958bcd592993beb710440279c1340d151784801b1fd81a70d050a0e3b9eecd06bd4515282270b02acf2689c1047decad0b79d036a1f3aea7752e67a0b54d70e6f100f82af2868b0ba07ad7c09f14cbd41d46617640257358b2437f74805108f90211a0ae2d1e2029a837e0019c72a71c382eb58692c7f834c5713fe9e1e086708803f0a0c0b63818837574716318adf01113c02c18a5fbf1c478ed808bfd055e19ec018da0727072d0bcb8a82ba1a825c9bada1a70dd6d8a3f9d89bb06a5ea8d30711087f0a057baf89d73bb35f05ac02ca7c694445a056204937221e12a927014e7c38d0b00a0e91e3559f359e864a2d99f9bed86611c326b9d469bc9cadf5ef8601129dc07eea014fcf072c8c4455c5836358af6ad37da865c0abf1f4b954b8c9de7c1f85e1215a01bd2168c928330cc4dfb24aa6ee38533ee0fba7419dc8982fe53ad2974265129a0476a03c86fd36b374b8ca14007f54bc064089664003b5f49a420e3991354047ca0a9e9f4e7dc962eb3ab31bb9f1787b6331a7933cf6c81e535e507a93305e0de63a0f91e75563dc3d91bf7da09a2765c4c9680a57143acdb11f21c465976d298cff5a0a33d99ae84afe21e636c011f38de0f80f1b1d6a08da50c274ed80b7b598e1086a0a8348d3cb796384a40781838c79d200971eefc31cd0b99dcde291ee5640d16dda0102b174c835185e6f86a45b09a160486a098ab2b0189c7044c4ab866e025c14fa0fb30a1f165ef3db56f0d87f468612f23c7867429d9fb960af37bbc48bcdded82a03f987aee2c6119174bc792c8ea27b01fbda2165eba867113cead98e018c39b1fa02348879e702d403003936bc990330028baab6b2ce8d8359e0fed70454d553ca9805108f90211a0de34aa377c84fe59a9797eaa855b4288f0b2b33f53de090ac3944121599a18e1a0c7929ff7e8871ee5b5d980d5a0f6b4d716930fd0c86b3de466e8435060ae9bbca08a8d6a34cf809050811bc73c801b85cc0fb3c5eb3db82a09ec9fd0466856e919a0e26e23259b627e371b8370a3461544aaa1450103e6cbe1511e7043adf18b18b5a0b679d03b6ec00c2f2d5e825193eba862d184877f01951fae4992da7ff3f8f84aa0c773189f98aae1ddbf860272702b6753e93ec54d03d7b97046ac3e98cf3e8ab2a06a772400444ce1e899a37196423c7cca430f566116784858c012b5a7c3f2292aa06b07252fd2474128bb446e2df31edd135fac931352bd78cfa29ee94d5a337a83a09feee2f012fdaaa753260124a952f9c37a42cbfffc4ba177f16d2d83587de86fa0933fd6566a0d0ebe29dfb75ebe9cfd28b6a2396fd489d10943f2e87f41cacee5a0f651d243865f090404cab1c27d8452a39b09feb0b0e2c58cd1a215b638fdbbb4a000f17b0fa971e174fdc464991ba20f04178b0b283529b0ce9ab62288f5382a6da0f3e4dadfedf9b53ace0da42a6c01682e72fb913315bf55cff108c8d06e896181a085bc4cc7f5f4ae2ccf0cc22f0b19f556457b8b48c2fd400e09fb0bf224d02cbfa0d24b2c4c610d3c277ae42dd915258c1e642546407272ba6fc25e816ceca18ca5a0665577f1691f0c24161ba7ec25e86db0b195091df762ae0fee43de4ea7d13182805108f90211a0e79d8bd2bd9b00f8f36c54355e1f80ea1b0aa2dca125cef5dc9393983bb484f0a0778003f7094bf33252b86ea86679fb3cd65938ffefc1a496d57b0ea843659683a066c1f9bcaaa6ede41599d538ae6bac15729e41fae5d9623676c6b9d7cbdfc5aea0b67eee17efe1f718fab9c8db6269e55730a311ad2d91bcbf6b8aaa6e8730ce32a0e23632639c8a7e383de305953ccd9287e160466267ea2fa7dac52cbeb183c9c1a000dc82caf45cd3785c5f0bb39de9c48d5d05fb2a7bce7fcee22c193980d1c5e9a0c9cc8d5f9eccfeb07d78c848829bbcced85dc3d1c8a2926d8baa0473494edd12a0552bae418a508cf274782615143828a10b04f7233b78e8aca30285866d4d8a9ca061106bc1757b4bf6bfe4d4b1102c7a77f65f391d03cf51af7ceefa13433aa9f9a03b8d53b5fb05e8159f6942ab52aad62939482d8259933c22a8fc0793bceba3cea0e6fb7e83e8f39ea339c2dbe04fce00b11cb68564696a31ca7d14a4aa6f20365ba0d6db6b34d892bd99613c4a4b845308d937331a0cdbf8d31ce3d2bc7bd0cbdc5aa095af566b755557eecb7b6bc438bb8d47acb02e0a68b0133328416b3d4025bdd3a0837e1ee637b6c8629a0c39272f9f6d4ee55bc2980207b3d5dc883198d3de8edda060359652d0872cd8bd27674347ec7643897c45d4c7e737291572f5e60757b469a0f8f9b5bda7342199fc7e3cdac1b18764ff214e099458b712b91567e5cd650ed680").unwrap();

        let op_host = H160::from(hex!("1D14e30e440B8DBA9765108eC291B7b66F98Fd09"));
        let bsc_host =  H160::from(hex!("4e5bbdd9fE89F54157DDb64b21eD4D1CA1CDf9a6"));

        let claim_proof = WithdrawalProof::decode(&mut &*claim_proof).unwrap();

        let host = Host::<Test>::default();
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
            ismp_contract_addresses: vec![(StateMachine::Ethereum(Ethereum::Base), op_host)].into_iter().collect(),
            l2_oracle_address: Default::default(),
            rollup_core_address: Default::default(),
            dispute_factory_address: Default::default(),
        };
        host.store_consensus_state(claim_proof.source_proof.height.id.consensus_state_id, bsc_consensus_state.encode()).unwrap();
        host.store_consensus_state(claim_proof.dest_proof.height.id.consensus_state_id, sync_committee_consensus_state.encode()).unwrap();

        host.store_consensus_state_id(claim_proof.source_proof.height.id.consensus_state_id, BSC_CONSENSUS_ID)
            .unwrap();

        host.store_consensus_state_id(claim_proof.dest_proof.height.id.consensus_state_id, BEACON_CONSENSUS_ID)
            .unwrap();

        host.store_unbonding_period(claim_proof.source_proof.height.id.consensus_state_id, 10_000_000_000).unwrap();

        host.store_challenge_period(claim_proof.source_proof.height.id.consensus_state_id, 0).unwrap();

        host.store_unbonding_period(claim_proof.dest_proof.height.id.consensus_state_id, 10_000_000_000).unwrap();

        host.store_challenge_period(claim_proof.dest_proof.height.id.consensus_state_id, 0).unwrap();



        pallet_ismp_relayer::Pallet::<Test>::accumulate_fees(RuntimeOrigin::none(), claim_proof.clone()).unwrap();
        assert_eq!(claim_proof.commitments.len(), 6);
        assert_eq!(Claimed::<Test>::iter().count(), 5);
    })
}