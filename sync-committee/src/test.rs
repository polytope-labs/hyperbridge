use crate::{HostConfig, L2Config, SyncCommitteeHost};
use arb_host::{ArbConfig, HostConfig as ArbHostConfig};
use arbitrum_verifier::verify_arbitrum_payload;
use codec::Decode;
use futures::StreamExt;
use ismp::host::{Ethereum, StateMachine};
use ismp_sync_committee::types::BeaconClientUpdate;
use op_host::{HostConfig as OpHostConfig, OpConfig};
use op_verifier::verify_optimism_payload;
use std::sync::Arc;
use sync_committee_primitives::constants::sepolia::Sepolia;
use tesseract_evm::{mock::Host, EvmConfig};
use tesseract_primitives::{mocks::MockHost, IsmpHost};

#[tokio::test]
async fn check_consensus_notification() -> anyhow::Result<()> {
	dotenv::dotenv().ok();
	let op_orl = std::env::var("OP_URL").expect("OP_URL must be set.");
	let arb_orl = std::env::var("ARB_URL").expect("OP_URL must be set.");
	let base_orl = std::env::var("BASE_URL").expect("BASE_URL must be set.");
	let geth_url = std::env::var("GETH_URL").expect("GETH_URL must be set.");
	let beacon_url = std::env::var("BEACON_URL").expect("BEACON_URL must be set.");
	let chain_a = MockHost::new(
		ismp_sync_committee::types::ConsensusState {
			frozen_height: Default::default(),
			light_client_state: Default::default(),
			ismp_contract_addresses: Default::default(),
			l2_consensus: Default::default(),
		},
		0,
		StateMachine::Polygon,
	);

	let chain_b = {
		let config = EvmConfig {
			rpc_urls: vec![geth_url.clone()],
			state_machine: StateMachine::Ethereum(Ethereum::ExecutionLayer),
			consensus_state_id: "SYNC".to_string(),
			ismp_host: Default::default(),
			handler: Default::default(),
			signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622".to_string(),
			..Default::default()
		};

		let host =
			HostConfig { beacon_http_urls: vec![beacon_url], consensus_update_frequency: 180 };
		let arb_config = ArbConfig {
			host: ArbHostConfig {
				beacon_rpc_url: vec![geth_url.clone()],
				rollup_core: sp_core::H160::from(hex_literal::hex!(
					"45e5cAea8768F42B385A366D3551Ad1e0cbFAb17"
				)),
			},
			evm_config: EvmConfig {
				rpc_urls: vec![arb_orl],
				state_machine: StateMachine::Ethereum(Ethereum::Arbitrum),
				consensus_state_id: "SYNC".to_string(),
				ismp_host: Default::default(),
				handler: Default::default(),
				signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622"
					.to_string(),
				..Default::default()
			},
		};

		let op_config = OpConfig {
			host: OpHostConfig {
				beacon_rpc_url: vec![geth_url.clone()],
				l2_oracle: Some(sp_core::H160::from(hex_literal::hex!(
					"E6Dfba0953616Bacab0c9A8ecb3a9BBa77FC15c0"
				))),
				message_parser: sp_core::H160::from(hex_literal::hex!(
					"4200000000000000000000000000000000000016"
				)),
				dispute_game_factory: None,
			},
			evm_config: EvmConfig {
				rpc_urls: vec![op_orl],
				state_machine: StateMachine::Ethereum(Ethereum::Optimism),
				consensus_state_id: "SYNC".to_string(),
				ismp_host: Default::default(),
				handler: Default::default(),
				signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622"
					.to_string(),
				..Default::default()
			},
		};

		let base_config = OpConfig {
			host: OpHostConfig {
				beacon_rpc_url: vec![geth_url],
				l2_oracle: Some(sp_core::H160::from(hex_literal::hex!(
					"2A35891ff30313CcFa6CE88dcf3858bb075A2298"
				))),
				message_parser: sp_core::H160::from(hex_literal::hex!(
					"4200000000000000000000000000000000000016"
				)),
				dispute_game_factory: None,
			},
			evm_config: EvmConfig {
				rpc_urls: vec![base_orl],
				state_machine: StateMachine::Ethereum(Ethereum::Base),
				consensus_state_id: "SYNC".to_string(),
				ismp_host: Default::default(),
				handler: Default::default(),
				signer: "2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622"
					.to_string(),
				..Default::default()
			},
		};

		let l2_configs = vec![
			(StateMachine::Ethereum(Ethereum::Base), L2Config::OpStack(base_config)),
			(StateMachine::Ethereum(Ethereum::Optimism), L2Config::OpStack(op_config)),
			(StateMachine::Ethereum(Ethereum::Arbitrum), L2Config::ArbitrumOrbit(arb_config)),
		]
		.into_iter()
		.collect();

		SyncCommitteeHost::<Sepolia>::new(&host, &config, l2_configs).await?
	};

	let mut consensus_stream =
		chain_b.consensus_notification(Arc::new(chain_a.clone())).await.unwrap();

	while let Some(res) = consensus_stream.next().await {
		println!("Received new event");
		match res {
			Ok(res) => {
				let BeaconClientUpdate {
					mut l2_oracle_payload,
					consensus_update,
					mut arbitrum_payload,
					..
				} = BeaconClientUpdate::decode(&mut &res.consensus_proof[..]).unwrap();
				(*chain_a.consensus_state.lock().unwrap()).light_client_state.finalized_header =
					consensus_update.finalized_header;
				(*chain_a.consensus_state.lock().unwrap())
					.light_client_state
					.latest_finalized_epoch = consensus_update.finality_proof.epoch;
				(*chain_a.latest_height.lock().unwrap()) =
					consensus_update.execution_payload.block_number;
				dbg!(consensus_update.execution_payload.block_number);
				let state_root = consensus_update.execution_payload.state_root;

				let op_stack = [
					(
						StateMachine::Ethereum(Ethereum::Base),
						hex_literal::hex!("2A35891ff30313CcFa6CE88dcf3858bb075A2298"),
					),
					(
						StateMachine::Ethereum(Ethereum::Optimism),
						hex_literal::hex!("E6Dfba0953616Bacab0c9A8ecb3a9BBa77FC15c0"),
					),
				];

				for (state_machine_id, l2_oracle) in op_stack {
					println!("Verifying {state_machine_id:?} payload proof");
					if let Some(payload) = l2_oracle_payload.remove(&state_machine_id) {
						let _state = verify_optimism_payload::<Host>(
							payload,
							state_root,
							l2_oracle.into(),
							Default::default(),
						)
						.unwrap();
					}
				}

				if let Some(arbitrum_payload) =
					arbitrum_payload.remove(&StateMachine::Ethereum(Ethereum::Arbitrum))
				{
					println!("Verifying arbitrum payload proof");
					let _state = verify_arbitrum_payload::<Host>(
						arbitrum_payload,
						state_root,
						hex_literal::hex!("45e5cAea8768F42B385A366D3551Ad1e0cbFAb17").into(),
						Default::default(),
					)
					.unwrap();
				}

				println!("Finished payload proof verification");
			},
			Err(err) => {
				println!("Failed to fetch light client update {err:?}")
			},
		}
	}
	Ok(())
}
