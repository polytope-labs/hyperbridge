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

use alloy::{
	eips::BlockId,
	primitives::{Address, B256},
	providers::{Provider, ProviderBuilder},
	rpc::types::Filter,
	sol,
	sol_types::SolEvent,
};
use anyhow::anyhow;
use hex_literal::hex;

use crate::{verify_arbitrum_bold, ArbitrumBoldProof, ASSERTIONS_SLOT};
use evm_state_machine::derive_unhashed_map_key;
use ismp_testsuite::mocks::{Host, Keccak256Hasher};
use primitive_types::{H160, H256};

sol! {
	#[derive(Debug, Default, PartialEq, Eq)]
	struct GlobalState {
		bytes32[2] bytes32Vals;
		uint64[2] u64Vals;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct AssertionState {
		GlobalState globalState;
		uint8 machineStatus;
		bytes32 endHistoryRoot;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct ConfigData {
		bytes32 wasmModuleRoot;
		uint256 requiredStake;
		address challengeManager;
		uint64 confirmPeriodBlocks;
		uint64 nextInboxPosition;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct BeforeStateData {
		bytes32 prevPrevAssertionHash;
		bytes32 sequencerBatchAcc;
		ConfigData configData;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	struct AssertionInputs {
		BeforeStateData beforeStateData;
		AssertionState beforeState;
		AssertionState afterState;
	}

	#[derive(Debug, Default, PartialEq, Eq)]
	event AssertionCreated(
		bytes32 indexed assertionHash,
		bytes32 indexed parentAssertionHash,
		AssertionInputs assertion,
		bytes32 afterInboxBatchAcc,
		uint256 inboxMaxCount,
		bytes32 wasmModuleRoot,
		uint256 requiredStake,
		address challengeManager,
		uint64 confirmPeriodBlocks
	);
}

#[tokio::test]
#[ignore]
async fn verify_bold_assertion() -> anyhow::Result<()> {
	let sepolia_block_number = 7587899u64;
	// Initialize a new Http provider
	dotenv::dotenv().ok();
	let rpc_url = std::env::var("SEPOLIA_URL").unwrap();
	let arb_url = std::env::var("ARB_URL").unwrap();

	let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);
	let arb_provider = ProviderBuilder::new().connect_http(arb_url.parse()?);

	let rollup = H160::from_slice(hex!("042B2E6C5E99d4c521bd49beeD5E99651D9B0Cf4").as_slice());
	let rollup_address = Address::from_slice(&rollup.0);

	let filter = Filter::new()
		.address(rollup_address)
		.from_block(sepolia_block_number)
		.to_block(sepolia_block_number);

	let logs = provider.get_logs(&filter).await?;
	let mut assertion = None;
	for log in logs {
		if let Ok(new_assertion) = AssertionCreated::decode_log(&log.inner) {
			assertion = Some(new_assertion);
			break;
		}
	}

	if assertion.is_none() {
		Err(anyhow!("Assertion not found in block"))?
	}
	let assertion = assertion.unwrap();

	dbg!(H256::from(assertion.assertionHash.0));

	let key = derive_unhashed_map_key::<Host>(assertion.assertionHash.0.to_vec(), ASSERTIONS_SLOT);
	let assertion_created_proof = provider
		.get_proof(rollup_address, vec![B256::from(key.0)])
		.block_id(BlockId::number(sepolia_block_number))
		.await?;

	let arb_block = arb_provider
		.get_block(BlockId::hash(B256::from(
			assertion.assertion.afterState.globalState.bytes32Vals[0].0,
		)))
		.await?
		.ok_or_else(|| anyhow!("Block not found"))?;

	let arbitrum_header = arb_block.into();

	let sepolia_header = provider
		.get_block(BlockId::number(sepolia_block_number))
		.await?
		.ok_or_else(|| anyhow!("Sepolia block not found"))?;

	let global_state = crate::GlobalState {
		block_hash: assertion.assertion.afterState.globalState.bytes32Vals[0].0.into(),
		send_root: assertion.assertion.afterState.globalState.bytes32Vals[1].0.into(),
		inbox_position: assertion.assertion.afterState.globalState.u64Vals[0],
		position_in_message: assertion.assertion.afterState.globalState.u64Vals[1],
	};

	let machine_status = assertion
		.assertion
		.afterState
		.machineStatus
		.try_into()
		.map_err(|_| anyhow!("Failed conversion"))?;

	let after_state = crate::AssertionState {
		global_state,
		machine_status,
		end_history_root: assertion.assertion.afterState.endHistoryRoot.0.into(),
	};

	let arbitrum_bold_proof = ArbitrumBoldProof {
		arbitrum_header,
		after_state,
		previous_assertion_hash: assertion.parentAssertionHash.0.into(),
		sequencer_batch_acc: assertion.afterInboxBatchAcc.0.into(),
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
		sepolia_header.header.state_root.0.into(),
		rollup,
		*b"ETH0",
	)?;

	dbg!(state_commitment);
	Ok(())
}
