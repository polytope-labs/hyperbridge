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

use anyhow::anyhow;
use ethers::prelude::*;
use hex_literal::hex;

use crate::{verify_arbitrum_bold, ArbitrumBoldProof, ASSERTIONS_SLOT};
use evm_state_machine::derive_unhashed_map_key;
use ismp_testsuite::mocks::{Host, Keccak256Hasher};
use primitive_types::{H160, H256};

#[derive(
	Clone,
	::ethers::contract::EthAbiType,
	::ethers::contract::EthAbiCodec,
	Default,
	Debug,
	PartialEq,
	Eq,
	Hash,
)]
pub struct AssertionInputs {
	pub before_state_data: BeforeStateData,
	pub before_state: AssertionState,
	pub after_state: AssertionState,
}

#[derive(
	Clone,
	::ethers::contract::EthAbiType,
	::ethers::contract::EthAbiCodec,
	Default,
	Debug,
	PartialEq,
	Eq,
	Hash,
)]
pub struct AssertionState {
	pub global_state: GlobalState,
	pub machine_status: u8,
	pub end_history_root: [u8; 32],
}

#[derive(
	Clone,
	::ethers::contract::EthAbiType,
	::ethers::contract::EthAbiCodec,
	Default,
	Debug,
	PartialEq,
	Eq,
	Hash,
)]
pub struct GlobalState {
	pub bytes_32_vals: [[u8; 32]; 2],
	pub u_64_vals: [u64; 2],
}

///`BeforeStateData(bytes32,bytes32,(bytes32,uint256,address,uint64,uint64))`
#[derive(
	Clone,
	::ethers::contract::EthAbiType,
	::ethers::contract::EthAbiCodec,
	Default,
	Debug,
	PartialEq,
	Eq,
	Hash,
)]
pub struct BeforeStateData {
	pub prev_prev_assertion_hash: [u8; 32],
	pub sequencer_batch_acc: [u8; 32],
	pub config_data: ConfigData,
}
///`ConfigData(bytes32,uint256,address,uint64,uint64)`
#[derive(
	Clone,
	::ethers::contract::EthAbiType,
	::ethers::contract::EthAbiCodec,
	Default,
	Debug,
	PartialEq,
	Eq,
	Hash,
)]
pub struct ConfigData {
	pub wasm_module_root: [u8; 32],
	pub required_stake: ::ethers::core::types::U256,
	pub challenge_manager: ::ethers::core::types::Address,
	pub confirm_period_blocks: u64,
	pub next_inbox_position: u64,
}

#[derive(
	Clone,
	::ethers::contract::EthEvent,
	::ethers::contract::EthDisplay,
	Default,
	Debug,
	PartialEq,
	Eq,
	Hash,
)]
#[ethevent(
	name = "AssertionCreated",
	abi = "AssertionCreated(bytes32,bytes32,((bytes32,bytes32,(bytes32,uint256,address,uint64,uint64)),((bytes32[2],uint64[2]),uint8,bytes32),((bytes32[2],uint64[2]),uint8,bytes32)),bytes32,uint256,bytes32,uint256,address,uint64)"
)]
pub struct AssertionCreatedFilter {
	#[ethevent(indexed)]
	pub assertion_hash: [u8; 32],
	#[ethevent(indexed)]
	pub parent_assertion_hash: [u8; 32],
	pub assertion: AssertionInputs,
	pub after_inbox_batch_acc: [u8; 32],
	pub inbox_max_count: ::ethers::core::types::U256,
	pub wasm_module_root: [u8; 32],
	pub required_stake: ::ethers::core::types::U256,
	pub challenge_manager: ::ethers::core::types::Address,
	pub confirm_period_blocks: u64,
}

#[tokio::test]
#[ignore]
async fn verify_bold_assertion() -> anyhow::Result<()> {
	let sepolia_block_number = 7587899u64;
	// Initialize a new Http provider
	dotenv::dotenv().ok();
	let rpc_url = std::env::var("SEPOLIA_URL").unwrap();
	let arb_url = std::env::var("ARB_URL").unwrap();
	let provider = Provider::try_from(rpc_url).unwrap();
	let arb_provider = Provider::try_from(arb_url).unwrap();

	let rollup = H160::from_slice(hex!("042B2E6C5E99d4c521bd49beeD5E99651D9B0Cf4").as_slice());
	let filter = Filter {
		block_option: FilterBlockOption::Range {
			from_block: Some(sepolia_block_number.into()),
			to_block: Some(sepolia_block_number.into()),
		},
		address: Some(ValueOrArray::Value(rollup.0.into())),
		topics: [None, None, None, None],
	};
	let logs = provider.get_logs(&filter).await?;
	let mut assertion = None;
	for log in logs {
		if let Ok(new_assertion) = parse_log::<AssertionCreatedFilter>(log) {
			assertion = Some(new_assertion);
			break;
		}
	}

	if assertion.is_none() {
		Err(anyhow!("Assertion not found in block"))?
	}
	let assertion = assertion.unwrap();

	dbg!(H256::from(assertion.assertion_hash));

	let key = derive_unhashed_map_key::<Host>(assertion.assertion_hash.to_vec(), ASSERTIONS_SLOT);
	let assertion_created_proof = provider
		.get_proof(
			ethers::core::types::H160::from(rollup.0),
			vec![key.0.into()],
			Some(sepolia_block_number.into()),
		)
		.await
		.unwrap();

	let arb_header = arb_provider
		.get_block(ethers::core::types::H256::from(
			assertion.assertion.after_state.global_state.bytes_32_vals[0],
		))
		.await?
		.unwrap();

	let arbitrum_header = arb_header.into();

	let sepolia_header = provider.get_block(sepolia_block_number).await?.unwrap();

	let global_state = crate::GlobalState {
		block_hash: assertion.assertion.after_state.global_state.bytes_32_vals[0].into(),
		send_root: assertion.assertion.after_state.global_state.bytes_32_vals[1].into(),
		inbox_position: assertion.assertion.after_state.global_state.u_64_vals[0],
		position_in_message: assertion.assertion.after_state.global_state.u_64_vals[1],
	};

	let machine_status = assertion
		.assertion
		.after_state
		.machine_status
		.try_into()
		.map_err(|_| anyhow!("Failed conversion"))?;

	let after_state = crate::AssertionState {
		global_state,
		machine_status,
		end_history_root: assertion.assertion.after_state.end_history_root.into(),
	};

	let arbitrum_bold_proof = ArbitrumBoldProof {
		arbitrum_header,
		after_state,
		previous_assertion_hash: assertion.parent_assertion_hash.into(),
		sequencer_batch_acc: assertion.after_inbox_batch_acc.into(),
		storage_proof: assertion_created_proof
			.storage_proof
			.get(0)
			.cloned()
			.ok_or_else(|| anyhow!("Expected storage proof"))?
			.proof
			.into_iter()
			.map(|node| node.0.to_vec())
			.collect(),
		contract_proof: assertion_created_proof
			.account_proof
			.into_iter()
			.map(|node| node.0.to_vec())
			.collect(),
	};

	let state_commitment = verify_arbitrum_bold::<Keccak256Hasher>(
		arbitrum_bold_proof,
		sepolia_header.state_root.0.into(),
		rollup,
		*b"ETH0",
	)?;

	dbg!(state_commitment);
	Ok(())
}
