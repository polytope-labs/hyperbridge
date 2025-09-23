#![cfg(test)]

// mod ethereum;
// mod grandpa;
// mod bsc;
mod ping;
// mod polygon;
// mod substrate;
//mod l2s;
mod util;

// use std::{
// 	sync::Arc,
// 	time::{SystemTime, UNIX_EPOCH},
// };

// use crate::util::{setup_logging, timeout_future, Hyperbridge};
// use futures::StreamExt;
// use hex_literal::hex;
// use ismp::{consensus::StateMachineId, host::StateMachine};
// use pallet_ismp_demo::GetRequest;
// use primitive_types::H160;
// use substrate_state_machine::HashAlgorithm;
// use subxt::{config::extrinsic_params::BaseExtrinsicParamsBuilder, utils::AccountId32};
// use subxt_utils::gargantua::api::{
// 	self,
// 	runtime_types::{self, gargantua_runtime::RuntimeCall},
// };
// use tesseract_substrate::{extrinsic::InMemorySigner, SubstrateClient, SubstrateConfig};

// use tesseract_primitives::IsmpProvider;
// use transaction_fees::TransactionPayment;

// type ParachainClient<T> = SubstrateClient<T>;

// pub async fn setup_clients(
// ) -> Result<(ParachainClient<Hyperbridge>, ParachainClient<Hyperbridge>), anyhow::Error> {
// 	let config_a = SubstrateConfig {
// 		state_machine: StateMachine::Kusama(2000),
// 		max_rpc_payload_size: None,
// 		hashing: Some(HashAlgorithm::Blake2),
// 		consensus_state_id: Some("PARA".to_string()),
// 		rpc_ws: "ws://localhost:9988".to_string(),
// 		signer: Some(
// 			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
// 		),

// 		initial_height: None,
// 		poll_interval: None,
// 		max_concurent_queries: None,
// 	};
// 	let chain_a = SubstrateClient::new(config_a).await?;

// 	let config_b = SubstrateConfig {
// 		state_machine: StateMachine::Kusama(2000),
// 		max_rpc_payload_size: None,
// 		hashing: Some(HashAlgorithm::Blake2),
// 		consensus_state_id: Some("PARA".to_string()),
// 		rpc_ws: "ws://localhost:9188".to_string(),
// 		signer: Some(
// 			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
// 		),

// 		initial_height: None,
// 		poll_interval: None,
// 		max_concurent_queries: None,
// 	};
// 	let chain_b = SubstrateClient::new(config_b).await?;
// 	Ok((chain_a, chain_b))
// }

// async fn transfer_assets(
// 	chain_a: &SubstrateClient<Hyperbridge>,
// 	chain_b: &SubstrateClient<Hyperbridge>,
// 	timeout: u64,
// ) -> Result<(), anyhow::Error> {
// 	let amt = 345876451382054092;

// 	let params = pallet_ismp_demo::TransferParams {
// 		to: chain_b.account(),
// 		amount: amt,
// 		timeout: 0,
// 		para_id: 2001,
// 	};
// 	dbg!(amt);
// 	chain_a.transfer(params).await?;

// 	timeout_future(
// 		chain_b.pallet_ismp_demo_events_stream(1, "IsmpDemo", "BalanceReceived"),
// 		timeout,
// 		"Did not see BalanceReceived Event".to_string(),
// 	)
// 	.await?;

// 	dbg!(amt);
// 	let params_b = pallet_ismp_demo::TransferParams {
// 		to: chain_a.account(),
// 		amount: amt,
// 		timeout: 0,
// 		para_id: 2000,
// 	};

// 	chain_b.transfer(params_b).await?;

// 	timeout_future(
// 		chain_a.pallet_ismp_demo_events_stream(1, "IsmpDemo", "BalanceReceived"),
// 		timeout,
// 		"Did not see BalanceReceived Event".to_string(),
// 	)
// 	.await?;
// 	Ok(())
// }

// #[tokio::test]
// async fn test_parachain_parachain_messaging_relay() -> Result<(), anyhow::Error> {
// 	setup_logging();

// 	let (chain_a, chain_b) = setup_clients().await?;
// 	let _tx_payment = Arc::new(TransactionPayment::initialize("./dev.db").await?);
// 	// let _message_handle = tokio::spawn({
// 	// 	let chain_a = chain_a.clone();
// 	// 	let chain_b = chain_b.clone();
// 	// 	async move {
// 	// 		tesseract_messaging::relay(
// 	// 			chain_a.clone(),
// 	// 			Arc::new(chain_b.clone()),
// 	// 			Default::default(),
// 	// 			StateMachine::Kusama(4009),
// 	// 			tx_payment,
// 	// 			Default::default(),
// 	// 		)
// 	// 		.await
// 	// 		.unwrap()
// 	// 	}
// 	// });

// 	// Make transfers each from both chains
// 	transfer_assets(&chain_a, &chain_b, 60 * 5).await?;

// 	// Send a Get request next
// 	chain_a
// 		.get_request(GetRequest {
// 			para_id: 2001,
// 			height: chain_b.latest_height() as u32,
// 			timeout: 0,
// 			keys: vec![hex::decode(
// 				"c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80".to_string(),
// 			)
// 			.unwrap()],
// 		})
// 		.await?;

// 	timeout_future(
// 		chain_a.pallet_ismp_demo_events_stream(1, "IsmpDemo", "GetResponse"),
// 		60 * 4,
// 		"Did not see Get Response Event".to_string(),
// 	)
// 	.await?;

// 	Ok(())
// }

// #[tokio::test]
// async fn sudo_upgrade_runtime() -> Result<(), anyhow::Error> {
// 	dotenv::dotenv().ok();
// 	let config_a = SubstrateConfig {
// 		state_machine: StateMachine::Kusama(2000),
// 		max_rpc_payload_size: None,
// 		hashing: Some(HashAlgorithm::Keccak),
// 		consensus_state_id: Some("PARA".to_string()),
// 		rpc_ws: "wss://nexus.ibp.network:443".to_string(),
// 		// rpc_ws: "ws://127.0.0.1:9901".to_string(),
// 		signer: std::env::var("SUBSTRATE_SIGNING_KEY").ok(),
// 		initial_height: None,
// 		poll_interval: None,
// 		max_concurent_queries: None,
// 	};

// 	let chain_a = SubstrateClient::<Hyperbridge>::new(config_a).await?;
// 	let code_blob =
// tokio::fs::read("../../hyperbridge/target/release/wbuild/nexus-runtime/nexus_runtime.compact.
// compressed.wasm").await?; 	chain_a.runtime_upgrade(code_blob).await?;
// 	Ok(())
// }

// #[tokio::test]
// async fn sudo_transfer_tokens() -> Result<(), anyhow::Error> {
// 	dotenv::dotenv().ok();
// 	let config_a = SubstrateConfig {
// 		state_machine: StateMachine::Kusama(2000),
// 		max_rpc_payload_size: None,
// 		hashing: Some(HashAlgorithm::Keccak),
// 		consensus_state_id: Some("PARA".to_string()),
// 		rpc_ws: "wss://hyperbridge-nexus-rpc.blockops.network:443".to_string(),
// 		// rpc_ws: "ws://127.0.0.1:9901".to_string(),
// 		signer: std::env::var("SUBSTRATE_SIGNING_KEY").ok(),
// 		initial_height: None,
// 		poll_interval: None,
// 		max_concurent_queries: None,
// 	};

// 	let chain = SubstrateClient::<Hyperbridge>::new(config_a).await?;
// 	let signer = InMemorySigner::<Hyperbridge>::new(chain.signer.clone());
// 	let ext = chain
// 		.client
// 		.tx()
// 		.create_signed(
// 			&api::tx().sudo().sudo(RuntimeCall::Balances(
// 				runtime_types::pallet_balances::pallet::Call::transfer_keep_alive {
// 					dest: subxt::utils::MultiAddress::Id(
// 						hex!("5cfad400109a799b73f9e5702f80f74b33f7c6a888affcdbe11bbaf4a988ce69")
// 							.into(),
// 					),
// 					value: 5_000_000_000_000,
// 				},
// 			)),
// 			&signer,
// 			BaseExtrinsicParamsBuilder::new(),
// 		)
// 		.await?;

// 	ext.submit_and_watch().await?.wait_for_finalized_success().await?;

// 	Ok(())
// }

// #[tokio::test]
// async fn set_host_manager() -> Result<(), anyhow::Error> {
// 	dotenv::dotenv().ok();
// 	let _config_a = SubstrateConfig {
// 		state_machine: StateMachine::Kusama(2000),
// 		max_rpc_payload_size: None,
// 		hashing: Some(HashAlgorithm::Keccak),
// 		consensus_state_id: Some("PARA".to_string()),
// 		rpc_ws: "ws://192.168.1.197:9990".to_string(),
// 		signer: std::env::var("SIGNING_KEY").ok(),
// 		initial_height: None,
// 		poll_interval: None,
// 		max_concurent_queries: None,
// 	};

// 	Ok(())
// }

// #[tokio::test]
// async fn test_state_machine_notifs() -> Result<(), anyhow::Error> {
// 	let config_a = SubstrateConfig {
// 		state_machine: StateMachine::Kusama(2000),
// 		max_rpc_payload_size: None,
// 		hashing: Some(HashAlgorithm::Keccak),
// 		consensus_state_id: Some("PARA".to_string()),
// 		rpc_ws: "wss://hyperbridge-rpc.blockops.network:443".to_string(),
// 		signer: Some(
// 			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
// 		),

// 		initial_height: None,
// 		poll_interval: None,
// 		max_concurent_queries: None,
// 	};

// 	let chain_a = SubstrateClient::<Hyperbridge>::new(config_a).await?;
// 	let state_machine_id =
// 		StateMachineId { state_id: StateMachine::Evm(1), consensus_state_id: *b"ETH1" };
// 	let mut stream = chain_a.state_machine_update_notification(state_machine_id).await?;
// 	while let Some(update) = stream.next().await {
// 		println!("Yielded Event {:?}", update);
// 	}
// 	Ok(())
// }

// #[tokio::test]
// #[ignore]
// async fn set_invulnerables() -> Result<(), anyhow::Error> {
// 	let config_a = SubstrateConfig {
// 		state_machine: StateMachine::Kusama(2000),
// 		max_rpc_payload_size: None,
// 		hashing: Some(HashAlgorithm::Keccak),
// 		consensus_state_id: Some("PARA".to_string()),
// 		rpc_ws: "wss://hyperbridge-rpc.blockops.network:443".to_string(),
// 		signer: Some(
// 			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
// 		),

// 		initial_height: None,
// 		poll_interval: None,
// 		max_concurent_queries: None,
// 	};

// 	let chain_a = SubstrateClient::<Hyperbridge>::new(config_a).await?;

// 	let accounts =
// 		vec![AccountId32(hex!("70f4edfe03752ef15576b1bd42dcdcfd112a768b1dcdd94d1bb5f8fa82d6a06c"))];

// 	chain_a.set_invulnerables(accounts).await?;

// 	Ok(())
// }

// #[tokio::test]
// async fn dispatch_to_evm() -> Result<(), anyhow::Error> {
// 	dotenv::dotenv().ok();
// 	let config_a = SubstrateConfig {
// 		state_machine: StateMachine::Kusama(4009),
// 		max_rpc_payload_size: None,
// 		hashing: Some(HashAlgorithm::Keccak),
// 		consensus_state_id: Some("PARA".to_string()),
// 		rpc_ws: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
// 		signer: std::env::var("SUBSTRATE_SIGNING_KEY").ok(),
// 		initial_height: None,
// 		poll_interval: None,
// 		max_concurent_queries: None,
// 	};

// 	let chains = vec![
// 		// sepolia
// 		(11155111, H160(hex!("3554a2260Aa37788DC8C2932A908fDa98a10Dd88"))),
// 		// arbitrum
// 		(421614, H160(hex!("3554a2260Aa37788DC8C2932A908fDa98a10Dd88"))),
// 		// op
// 		(11155420, H160(hex!("3554a2260Aa37788DC8C2932A908fDa98a10Dd88"))),
// 		// base
// 		(84532, H160(hex!("3554a2260Aa37788DC8C2932A908fDa98a10Dd88"))),
// 	];

// 	let chain_a = SubstrateClient::<Hyperbridge>::new(config_a).await?;

// 	let since_the_epoch =
// 		SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");

// 	for (dest, contract) in chains {
// 		println!("sending");
// 		chain_a
// 			.dispatch_to_evm(pallet_ismp_demo::EvmParams {
// 				module: contract,
// 				destination: dest,
// 				timeout: since_the_epoch.as_secs() * 60 * 60,
// 				count: 1,
// 			})
// 			.await?;
// 	}

// 	Ok(())
// }
