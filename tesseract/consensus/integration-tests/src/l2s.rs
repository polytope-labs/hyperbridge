use codec::Encode;
use futures::TryStreamExt;
use std::sync::Arc;

use arb_host::{ArbConfig, ArbHost};
use hex_literal::hex;
use ismp::consensus::StateMachineId;
use polkadot_sdk::sp_runtime::{testing::H256, traits::IdentifyAccount, MultiSigner};
use sp_core::{Pair, H160};
use subxt::{
	config::HashFor,
	dynamic::Value,
	ext::scale_value::{value, Composite},
	tx::Payload,
	utils::AccountId32,
};

use crate::util::setup_logging;
use ismp::{
	host::StateMachine,
	messaging::{ConsensusMessage, CreateConsensusState, Message},
};
use op_host::OpHost;
use op_verifier::{DisputeGameImpl, GameTypeConfig};
use pallet_ismp::weights::IsmpModuleWeight;
use substrate_state_machine::HashAlgorithm;
use subxt_utils::{
	send_extrinsic,
	values::{messages_to_value, state_machine_id_to_value},
	Hyperbridge, InMemorySigner,
};
use sync_committee_primitives::constants::{
	ETH1_DATA_VOTES_BOUND_ETH, PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM,
};
use tesseract_beefy::host::BeefyHost;
use tesseract_evm::EvmConfig;
use tesseract_grandpa::{GrandpaConfig, GrandpaHost};
use tesseract_primitives::IsmpHost;
use tesseract_substrate::{
	config::Blake2SubstrateChain, extrinsic::send_unsigned_extrinsic, SubstrateClient,
	SubstrateConfig,
};
use tesseract_sync_committee::SyncCommitteeHost;

async fn setup_clients() -> Result<
	(
		GrandpaHost<Blake2SubstrateChain, Hyperbridge>,
		SyncCommitteeHost<
			sync_committee_primitives::constants::sepolia::Sepolia,
			ETH1_DATA_VOTES_BOUND_ETH,
			PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM,
		>,
		ArbHost,
		OpHost,
	),
	anyhow::Error,
> {
	// The beacon API and the L1 execution RPC are separate endpoints on most providers, so they
	// are configured separately rather than assuming one host serves both.
	let beacon_url = env!("BEACON_URL").to_string();
	let l1_exec_url = env!("SEPOLIA_EXEC_URL").to_string();
	let arb_url = env!("ARB_URL").to_string();
	let op_url = env!("OP_URL").to_string();

	let config_a = SubstrateConfig {
		state_machine: Some(StateMachine::Kusama(2000)),
		hashing: Some(HashAlgorithm::Keccak),
		consensus_state_id: Some("PARA".to_string()),
		rpc_ws: "ws://localhost:9944".to_string(),
		max_rpc_payload_size: None,
		signer: Some(
			"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
		),
		initial_height: None,
		max_concurent_queries: None,
		poll_interval: None,
		fee_token_decimals: None,
	};

	let host = tesseract_grandpa::HostConfig {
		rpc: "wss://gargantua.rpc.polytope.technology:443".to_string(),
		slot_duration: 12,
		consensus_update_frequency: Some(60),
		para_ids: vec![],
		max_block_range: None,
	};

	let hyperbridge_grandpa_config = GrandpaConfig { grandpa: host };

	let hyperbridge_chain = GrandpaHost::<Blake2SubstrateChain, Hyperbridge>::new(
		&config_a,
		&hyperbridge_grandpa_config,
	)
	.await?;

	let sync_committee_chain = {
		let config = EvmConfig {
			rpc_urls: vec![l1_exec_url.clone()],
			state_machine: Some(StateMachine::Evm(11155111)),
			consensus_state_id: Some("ETH0".to_string()),
			ismp_host: Some(hex!("9AA003594d59C62EE17A73A569Fd7B1DbdBd71E1").into()),
			signer: Some(
				"6284acbdef4b15b21b64d9fbdcb7c7d4fa05f1a96364d12c2988bddc18356d84".to_string(),
			),
			transport: tesseract_evm::transport::RpcTransport::Standard,
			..Default::default()
		};

		let sync_commitee_config = tesseract_sync_committee::HostConfig {
			beacon_http_urls: vec![beacon_url.clone()],
			consensus_update_frequency: 60,
		};

		SyncCommitteeHost::<
			sync_committee_primitives::constants::sepolia::Sepolia,
			ETH1_DATA_VOTES_BOUND_ETH,
			PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM,
		>::new(&sync_commitee_config, &config, Default::default())
		.await?
	};
	let sync_committee_initial_consensus_state_message_for_other_chains =
		sync_committee_chain.query_initial_consensus_state().await?.unwrap();

	let arbitrum_chain = {
		let evm_config = EvmConfig {
			rpc_urls: vec![arb_url],
			state_machine: Some(StateMachine::Evm(421614)),
			consensus_state_id: Some("ARB0".to_string()),
			ismp_host: Some(hex!("3435bD7e5895356535459D6087D1eB982DAd90e7").into()),
			signer: Some(
				"6284acbdef4b15b21b64d9fbdcb7c7d4fa05f1a96364d12c2988bddc18356d84".to_string(),
			),
			gas_price_buffer: Some(800),
			transport: tesseract_evm::transport::RpcTransport::Standard,
			..Default::default()
		};

		let host = arb_host::HostConfig {
			ethereum_rpc_url: vec![l1_exec_url.clone()],
			rollup_core: H160::from(hex!("042B2E6C5E99d4c521bd49beeD5E99651D9B0Cf4")),
			l1_state_machine: StateMachine::Evm(11155111),
			l1_consensus_state_id: "ETH0".to_string(),
			consensus_state_id: "ARB0".to_string(),
			consensus_update_frequency: None,
		};

		ArbHost::new(&host, &evm_config).await?
	};
	let arbirtum_initial_consensus_state_message_for_other_chains =
		arbitrum_chain.query_initial_consensus_state().await?.unwrap();
	let arbitrum_state_machine_id =
		StateMachineId { state_id: StateMachine::Evm(421614), consensus_state_id: *b"ARB0" };
	set_arbitrum_config_on_hyperbridge(
		hyperbridge_chain.clone(),
		arbitrum_state_machine_id,
		arbitrum_chain.host.rollup_core,
	)
	.await?;

	let optimism_chain = {
		let evm_config = EvmConfig {
			rpc_urls: vec![op_url],
			state_machine: Some(StateMachine::Evm(11155420)),
			consensus_state_id: Some("OPT0".to_string()),
			ismp_host: Some(hex!("6d51b678836d8060d980605d2999eF211809f3C2").into()),
			signer: Some(
				"6284acbdef4b15b21b64d9fbdcb7c7d4fa05f1a96364d12c2988bddc18356d84".to_string(),
			),
			gas_price_buffer: Some(500),
			transport: tesseract_evm::transport::RpcTransport::Standard,
			..Default::default()
		};

		let host = op_host::HostConfig {
			ethereum_rpc_url: vec![l1_exec_url],
			l1_state_machine: StateMachine::Evm(11155111),
			l1_consensus_state_id: "ETH0".to_string(),
			consensus_state_id: "OPT0".to_string(),
			consensus_update_frequency: None,

			l2_oracle: None,
			message_parser: H160::from(hex!("4200000000000000000000000000000000000016")),
			dispute_game_factory: Some(H160::from(hex!(
				"05F9613aDB30026FFd634f38e5C4dFd30a197Fa1"
			))),
			proposer_config: None,
		};

		OpHost::new(&host, &evm_config).await?
	};
	let optimism_state_machine_id =
		StateMachineId { state_id: StateMachine::Evm(11155420), consensus_state_id: *b"OPT0" };

	// OP Sepolia's factory stopped registering the output root game types, so type 9 is the only
	// one still being proposed there.
	set_optimism_config_on_hyperbridge(
		hyperbridge_chain.clone(),
		optimism_state_machine_id,
		optimism_chain.host.dispute_game_factory.unwrap(),
		vec![GameTypeConfig {
			game_type: 9,
			expected_impl: H160::from(hex!("19AF533Cc2A2A55786DCB8672aA5717e64213208")),
			kind: DisputeGameImpl::SuperFaultDisputeGame,
		}],
	)
	.await?;

	let optimism_initial_consensus_state_message_for_other_chains =
		optimism_chain.query_initial_consensus_state().await?.unwrap();

	log::info!(target: crate::LOG_TARGET, "🧊 Setting consensus states");
	hyperbridge_chain
		.provider()
		.set_initial_consensus_state(
			sync_committee_initial_consensus_state_message_for_other_chains,
		)
		.await?;
	hyperbridge_chain
		.provider()
		.set_initial_consensus_state(arbirtum_initial_consensus_state_message_for_other_chains)
		.await?;
	hyperbridge_chain
		.provider()
		.set_initial_consensus_state(optimism_initial_consensus_state_message_for_other_chains)
		.await?;

	Ok((hyperbridge_chain, sync_committee_chain, arbitrum_chain, optimism_chain))
}

pub async fn set_arbitrum_config_on_hyperbridge(
	hyperbridge_chain: GrandpaHost<Blake2SubstrateChain, Hyperbridge>,
	state_machine_id: StateMachineId,
	rollup_core_address: H160,
) -> Result<(), anyhow::Error> {
	let client = hyperbridge_chain.substrate_client;

	let binding = client.signer.public();
	let public_key_slice: &[u8] = binding.as_ref();

	let public_key_array: [u8; 32] =
		public_key_slice.try_into().expect("sr25519 public key should be 32 bytes");

	let account_id = AccountId32::from(public_key_array);

	let signer = InMemorySigner { account_id: account_id.into(), signer: client.signer.clone() };

	let state_machine_id_value = state_machine_id_to_value(&state_machine_id);

	let rollup_core_address_value = Value::from_bytes(rollup_core_address.0.to_vec());

	let inner_tx_args = vec![state_machine_id_value, rollup_core_address_value];

	let call = subxt::dynamic::tx("IsmpArbitrum", "set_rollup_core_address", inner_tx_args);

	let tx = subxt::dynamic::tx("Sudo", "sudo", vec![call.into_value()]);
	send_extrinsic(&client.client, &signer, &tx, None, true).await?;

	Ok(())
}

pub async fn set_optimism_config_on_hyperbridge(
	hyperbridge_chain: GrandpaHost<Blake2SubstrateChain, Hyperbridge>,
	state_machine_id: StateMachineId,
	dispute_game_factory: H160,
	game_type_configs: Vec<GameTypeConfig>,
) -> Result<(), anyhow::Error> {
	println!("trying to set optimism config");

	let client = hyperbridge_chain.substrate_client;

	let binding = client.signer.public();
	let public_key_slice: &[u8] = binding.as_ref();

	let public_key_array: [u8; 32] =
		public_key_slice.try_into().expect("sr25519 public key should be 32 bytes");

	let account_id = AccountId32::from(public_key_array);

	let signer = InMemorySigner { account_id: account_id.into(), signer: client.signer.clone() };

	let state_machine_id_value = state_machine_id_to_value(&state_machine_id);

	let dispute_game_factory_value = Value::from_bytes(dispute_game_factory.0.to_vec());
	let game_type_configs_value = Value::unnamed_composite(
		game_type_configs.iter().map(game_type_config_to_value).collect::<Vec<_>>(),
	);

	let inner_tx_args =
		vec![state_machine_id_value, dispute_game_factory_value, game_type_configs_value];

	let call = subxt::dynamic::tx("IsmpOptimism", "set_dispute_game_factories", inner_tx_args);
	println!("constructing sudo call");
	let tx = subxt::dynamic::tx("Sudo", "sudo", vec![call.into_value()]);
	send_extrinsic(&client.client, &signer, &tx, None, true).await?;

	Ok(())
}

fn game_type_config_to_value(config: &GameTypeConfig) -> Value {
	let kind = match config.kind {
		DisputeGameImpl::OPSuccinct => Value::unnamed_variant("OPSuccinct", Vec::<Value>::new()),
		DisputeGameImpl::FaultDisputeGame =>
			Value::unnamed_variant("FaultDisputeGame", Vec::<Value>::new()),
		DisputeGameImpl::AggregateVerifier =>
			Value::unnamed_variant("AggregateVerifier", Vec::<Value>::new()),
		DisputeGameImpl::SuperFaultDisputeGame =>
			Value::unnamed_variant("SuperFaultDisputeGame", Vec::<Value>::new()),
	};
	Value::named_composite(vec![
		("game_type", value!(config.game_type)),
		("expected_impl", Value::from_bytes(config.expected_impl.0.to_vec())),
		("kind", kind),
	])
}

#[tokio::test]
#[ignore]
async fn test_consensus_messaging_relay() -> Result<(), anyhow::Error> {
	setup_logging();

	log::info!(target: crate::LOG_TARGET, "🧊 Initializing tesseract consensus");

	let (hyperbridge_chain, sync_committee_chain, arbitrum_chain, optimism_chain) =
		setup_clients().await?;

	let handle_a = tokio::spawn({
		let hyperbridge_chain = hyperbridge_chain.clone();
		let sync_committee_chain = sync_committee_chain.clone();
		async move {
			sync_committee_chain
				.start_consensus(hyperbridge_chain.provider())
				.await
				.unwrap()
		}
	});

	let handle_b = tokio::spawn({
		let hyperbridge_chain = hyperbridge_chain.clone();
		let arbitrum_chain = arbitrum_chain.clone();
		async move { arbitrum_chain.start_consensus(hyperbridge_chain.provider()).await.unwrap() }
	});

	let handle_c = tokio::spawn({
		let hyperbridge_chain = hyperbridge_chain.clone();
		let optimism_chain = optimism_chain.clone();
		async move { optimism_chain.start_consensus(hyperbridge_chain.provider()).await.unwrap() }
	});

	log::info!(target: crate::LOG_TARGET, "🧊 Initialized consensus tasks");

	let _ = tokio::join!(handle_a, handle_b, handle_c);

	Ok(())
}
