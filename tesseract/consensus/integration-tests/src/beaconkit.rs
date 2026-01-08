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

use codec::Encode;
use ismp::{host::StateMachine, messaging::CreateConsensusState};
use ismp_beacon_kit::BEACON_KIT_CONSENSUS_CLIENT_ID;
use std::sync::Arc;
use substrate_state_machine::HashAlgorithm;
use subxt_utils::Hyperbridge;
use tesseract_beaconkit::{BeaconKitConfig, BeaconKitHost, BeaconKitHostConfig};
use tesseract_evm::EvmConfig;
use tesseract_primitives::IsmpHost;
use tesseract_substrate::{SubstrateClient, SubstrateConfig};

use crate::util::setup_logging;

#[tokio::test]
async fn beaconkit_consensus_updates() -> anyhow::Result<()> {
	setup_logging();
	dotenv::dotenv().ok();

	let beaconkit_execution_url =
		std::env::var("BEACONKIT_EXECUTION_RPC").expect("BEACONKIT_EXECUTION_RPC must be set");
	let beaconkit_cometbft_rpc =
		std::env::var("BEACONKIT_COMETBFT_RPC").expect("BEACONKIT_COMETBFT_RPC must be set");

	let evm_config = EvmConfig {
		rpc_urls: vec![beaconkit_execution_url.clone()],
		state_machine: StateMachine::Evm(80094),
		consensus_state_id: "BKIT".to_string(),
		ismp_host: Default::default(),
		signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
		tracing_batch_size: None,
		query_batch_size: None,
		poll_interval: None,
		gas_price_buffer: None,
		client_type: Default::default(),
		initial_height: None,
	};

	let host_config = BeaconKitHostConfig {
		consensus_update_frequency: Some(300),
		rpc_url: beaconkit_cometbft_rpc,
		trusting_period_secs: Some(82 * 3600),
		unbonding_period_secs: Some(82 * 3600),
		clock_drift_secs: Some(86400),
	};

	let beaconkit_host = BeaconKitHost::new(&host_config, &evm_config).await?;

	let config_a = SubstrateConfig {
		state_machine: StateMachine::Kusama(2000),
		hashing: Some(HashAlgorithm::Keccak),
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: "ws://localhost:9990".to_string(),
		max_rpc_payload_size: None,
		signer: "0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
		initial_height: None,
		max_concurent_queries: None,
		poll_interval: None,
		fee_token_decimals: None,
	};
	let chain_a = SubstrateClient::<Hyperbridge>::new(config_a).await?;

	println!("getting initial consensus state");
	let initial_consensus_state = beaconkit_host.get_consensus_state().await?;

	println!("creating initial consensus state");

	chain_a
		.create_consensus_state(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: BEACON_KIT_CONSENSUS_CLIENT_ID,
			consensus_state_id: *b"BKIT",
			unbonding_period: 82 * 3600,
			challenge_periods: vec![(StateMachine::Evm(80094), 2 * 60)].into_iter().collect(),
			state_machine_commitments: vec![],
		})
		.await?;

	println!("created consensus state");

	beaconkit_host.start_consensus(Arc::new(chain_a)).await?;

	Ok(())
}
