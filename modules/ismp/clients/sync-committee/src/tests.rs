// Copyright (C) Polytope Labs Ltd.
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

use crate::{presets::NODES_SLOT, utils::derive_map_key};
use ethers::prelude::*;
use hex_literal::hex;

use ismp_testsuite::mocks::Host;
use sp_core::{H160, H256};

#[tokio::test]
#[ignore]
/// This test ensures that the key derivation works correctly.
async fn fetch_arbitrum_node_state_hash() {
    // Initialize a new Http provider
    let rpc_url = "https://rpc.ankr.com/eth";
    let provider = Provider::try_from(rpc_url).unwrap();
    let rollup = H160::from_slice(hex!("5eF0D09d1E6204141B4d37530808eD19f60FBa35").as_slice());
    let mut latest_node_created_bytes = [0u8; 32];
    // Latest node created is at slot 117
    U256::from(117).to_big_endian(&mut latest_node_created_bytes);
    let latest_node_created_position = H256::from_slice(&latest_node_created_bytes[..]);

    let latest_node_created_proof = provider
        .get_proof(rollup, vec![latest_node_created_position], None)
        .await
        .unwrap();
    // the latest node created is the second item in this slot
    let latest_node_created = latest_node_created_proof.storage_proof[0].value.0[1];
    dbg!(latest_node_created);
    let mut key = [0u8; 32];
    U256::from(latest_node_created).to_big_endian(&mut key);
    let position = derive_map_key::<Host>(key.to_vec(), NODES_SLOT);
    let proof = provider.get_proof(rollup, vec![position], None).await.unwrap();

    let mut buf = vec![0u8; 32];
    proof.storage_proof[0].clone().value.to_big_endian(&mut buf);
    let state_hash = hex::encode(buf);
    println!("State Hash {}", state_hash);
}
