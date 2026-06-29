use crate::{HostConfig, OpHost};
use alloy::{
	eips::BlockId,
	primitives::{Address, B256},
	providers::Provider,
};
use hex_literal::hex;
use primitive_types::{H160, H256};
// use ismp_testsuite::mocks::Host;
// use op_verifier::{verify_optimism_dispute_game_proof, verify_optimism_payload};
use crate::abi::DisputeGameFactory::DisputeGameCreated;
use ismp::host::StateMachine;
use op_verifier::{verify_optimism_dispute_game_proof, DisputeGameImpl, GameTypeConfig};
use tesseract_evm::EvmConfig;
use tesseract_primitives::Hasher;

const MESSAGE_PARSER: [u8; 20] = hex!("4200000000000000000000000000000000000016");

/// Placeholder secp256k1 key for read-only tests. `EvmClient::new` requires a valid signer to
/// construct, but none of the consensus-verification flow this file exercises actually signs or
/// sends transactions.
const DUMMY_SIGNING_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000001";
/// Verify a real `DisputeGameCreated` event against the factory on L1 at the latest block head
/// by round-tripping through the host's own proof-construction path.
///
/// Requires live RPC access to both the L1 (where the factory lives) and the L2 (where the
/// block backing the root claim lives) the game was proposed for.
async fn run_dispute_game_verification(
	l1_url: String,
	l2_url: String,
	l1_chain_id: u32,
	factory_addr: H160,
	event: DisputeGameCreated,
	game_type_configs: Vec<GameTypeConfig>,
) {
	let host = HostConfig {
		ethereum_rpc_url: vec![l1_url],
		l2_oracle: None,
		message_parser: H160::from(MESSAGE_PARSER),
		dispute_game_factory: Some(factory_addr),
		proposer_config: None,
		l1_state_machine: StateMachine::Evm(l1_chain_id),
		l1_consensus_state_id: "ETH0".to_string(),
		consensus_update_frequency: None,
		consensus_state_id: "OPT0".to_string(),
	};
	let evm_config = EvmConfig {
		rpc_urls: vec![l2_url],
		// Placeholders: `EvmClient::new` requires these to be resolved, but the dispute-game
		// verification path exercised here never consults the L2 state machine id or ismp host.
		state_machine: Some(StateMachine::Evm(0)),
		ismp_host: Some(H160::zero()),
		consensus_state_id: Some("ETH0".to_string()),
		signer: Some(DUMMY_SIGNING_KEY.to_string()),
		..Default::default()
	};
	let op_client = OpHost::new(&host, &evm_config).await.expect("Host creation failed");

	// Step back a handful of blocks so we're pinning to a block that's well past any reorg
	// window — the storage proofs returned by load-balanced RPCs need to match the state root
	// we cite, and "latest" can shift between sibling calls.
	let head = op_client
		.beacon_execution_client
		.get_block_number()
		.await
		.expect("L1 block number");
	let l1_block_num = head.saturating_sub(8);
	let l1_header = op_client
		.beacon_execution_client
		.get_block(BlockId::number(l1_block_num))
		.await
		.expect("L1 block")
		.expect("L1 block exists");
	let l1_state_root = H256::from_slice(l1_header.header.state_root.as_slice());

	let payload = op_client
		.fetch_dispute_game_payload(l1_block_num, game_type_configs.clone(), vec![event])
		.await
		.expect("fetch_dispute_game_payload")
		.expect("payload must be produced for a registered game");

	verify_optimism_dispute_game_proof::<Hasher>(
		payload,
		l1_state_root,
		factory_addr,
		game_type_configs,
		Default::default(),
	)
	.expect("dispute-game proof must verify at latest L1 head");
}

/// End-to-end verification of the AggregateVerifier dispute game created on Ethereum Sepolia:
///
/// ```text
/// DisputeGameCreated(
///     proxy:     0x2C9ecE6a5856ab7F5f2C49072e9F7A4F00D4C1E6,
///     gameType:  621,
///     rootClaim: 0xA76FA38FCEF8C7E910FB80064B2B98C275569A84C8DB3C4E04141B9457867B95,
/// )
/// ```
///
/// Factory: `0xd6E6dBf4F7EA0ac412fD8b65ED297e64BB7a06E1` on L1 Sepolia. Requires
/// `SEPOLIA_RPC_URL` (L1) and `BASE_SEPOLIA_RPC_URL` (L2) in the environment.
#[tokio::test]
#[ignore]
async fn test_aggregate_verifier_dispute_game_verification() {
	dotenv::dotenv().ok();
	let l1_url = std::env::var("SEPOLIA_RPC_URL")
		.expect("SEPOLIA_RPC_URL must be set to an Ethereum Sepolia RPC endpoint");
	let l2_url = std::env::var("BASE_SEPOLIA_RPC_URL")
		.expect("BASE_SEPOLIA_RPC_URL must be set to a Base Sepolia RPC endpoint");

	let event = DisputeGameCreated {
		disputeProxy: Address::from_slice(&hex!("2c9ece6a5856ab7f5f2c49072e9f7a4f00d4c1e6")),
		gameType: 621,
		rootClaim: B256::from(hex!(
			"a76fa38fcef8c7e910fb80064b2b98c275569a84c8db3c4e04141b9457867b95"
		)),
	};
	let factory_addr = H160::from(hex!("d6e6dbf4f7ea0ac412fd8b65ed297e64bb7a06e1"));
	let game_type_configs = vec![GameTypeConfig {
		game_type: 621,
		expected_impl: H160::from(hex!("498313fb340cd5055c5568546364008299a47517")),
		kind: DisputeGameImpl::AggregateVerifier,
	}];

	// Sepolia chain id = 11155111.
	run_dispute_game_verification(l1_url, l2_url, 11155111, factory_addr, event, game_type_configs)
		.await;
}

/// End-to-end verification of a Cannon (gameType 8) dispute game created on Ethereum mainnet:
///
/// ```text
/// DisputeGameCreated(
///     proxy:     0x48dDB9bfE0e24828FF39406aEda9cE1a9107b80f,
///     gameType:  8,
///     rootClaim: 0x1cbae15429a91277bfef6ab3578e7e72a9968741853d6b220c02676583436aa5,
/// )
/// ```
///
/// Factory: `0xe5965Ab5962eDc7477C8520243A95517CD252fA9` on L1 mainnet, whose `gameImpls[8]`
/// resolves to `0x2DDA3584b51eF5236f7726Dea5A0FB6B3cA94AeC`. Requires `MAINNET_RPC_URL` (L1) and
/// `OP_MAINNET_RPC_URL` (L2 Optimism mainnet) in the environment.
#[tokio::test]
#[ignore]
async fn test_cannon_dispute_game_verification() {
	dotenv::dotenv().ok();
	let l1_url = std::env::var("MAINNET_RPC_URL")
		.expect("MAINNET_RPC_URL must be set to an Ethereum mainnet RPC endpoint");
	let l2_url = std::env::var("OP_MAINNET_RPC_URL")
		.expect("OP_MAINNET_RPC_URL must be set to an Optimism mainnet RPC endpoint");

	let event = DisputeGameCreated {
		disputeProxy: Address::from_slice(&hex!("48ddb9bfe0e24828ff39406aeda9ce1a9107b80f")),
		gameType: 8,
		rootClaim: B256::from(hex!(
			"1cbae15429a91277bfef6ab3578e7e72a9968741853d6b220c02676583436aa5"
		)),
	};
	let factory_addr = H160::from(hex!("e5965ab5962edc7477c8520243a95517cd252fa9"));
	let game_type_configs = vec![GameTypeConfig {
		game_type: 8,
		// Cannon implementation pinned for gameImpls[8] on the OP mainnet factory.
		expected_impl: H160::from(hex!("2DDA3584b51eF5236f7726Dea5A0FB6B3cA94AeC")),
		kind: DisputeGameImpl::FaultDisputeGame,
	}];

	// Ethereum mainnet chain id = 1.
	run_dispute_game_verification(l1_url, l2_url, 1, factory_addr, event, game_type_configs).await;
}

/// Exercises the full host-side flow — `latest_dispute_games` → `fetch_dispute_game_payload`
/// → `verify_optimism_dispute_game_proof` — against Base Sepolia's factory over an L1 block
/// range that definitely contains a live, unchallenged AggregateVerifier game. Reproduces the
/// "consensus updates stop flowing" symptom by doing exactly what the relayer loop does, so a
/// regression in the host-side challenge filter or the payload builder surfaces here.
#[tokio::test]
#[ignore]
async fn test_base_sepolia_latest_and_verify() {
	dotenv::dotenv().ok();
	let l1_url = std::env::var("SEPOLIA_RPC_URL")
		.expect("SEPOLIA_RPC_URL must be set to an Ethereum Sepolia RPC endpoint");
	let l2_url = std::env::var("BASE_SEPOLIA_RPC_URL")
		.expect("BASE_SEPOLIA_RPC_URL must be set to a Base Sepolia RPC endpoint");

	let factory_addr = H160::from(hex!("d6e6dbf4f7ea0ac412fd8b65ed297e64bb7a06e1"));
	let game_type_configs = vec![
		GameTypeConfig {
			game_type: 0,
			expected_impl: H160::from(hex!("6dDBa09bc4cCB0D6Ca9Fc5350580f74165707499")),
			kind: DisputeGameImpl::FaultDisputeGame,
		},
		GameTypeConfig {
			game_type: 1,
			expected_impl: H160::from(hex!("58bf355C5d4EdFc723eF89d99582ECCfd143266A")),
			kind: DisputeGameImpl::FaultDisputeGame,
		},
		GameTypeConfig {
			game_type: 621,
			expected_impl: H160::from(hex!("c45dC8a279b2fDB7efEF72044e53514eD1bc2c08")),
			kind: DisputeGameImpl::AggregateVerifier,
		},
	];

	let host = HostConfig {
		ethereum_rpc_url: vec![l1_url],
		l2_oracle: None,
		message_parser: H160::from(MESSAGE_PARSER),
		dispute_game_factory: Some(factory_addr),
		proposer_config: None,
		l1_state_machine: StateMachine::Evm(11155111),
		l1_consensus_state_id: "ETH0".to_string(),
		consensus_update_frequency: None,
		consensus_state_id: "OPT0".to_string(),
	};
	let evm_config = EvmConfig {
		rpc_urls: vec![l2_url],
		state_machine: Some(StateMachine::Evm(84532)),
		ismp_host: Some(H160::zero()),
		consensus_state_id: Some("ETH0".to_string()),
		signer: Some(DUMMY_SIGNING_KEY.to_string()),
		..Default::default()
	};
	let op_client = OpHost::new(&host, &evm_config).await.expect("Host creation failed");

	// Widen the range to capture multiple game types so the diagnostic surfaces any Cannon
	// or Permissioned events alongside the AggregateVerifier ones.
	let to_block = op_client.beacon_execution_client.get_block_number().await.expect("L1 head") - 8;
	let from_block = to_block.saturating_sub(2000);

	// Walk the same steps `latest_dispute_games` does, printing each stage, so we can see
	// whether a valid game is being filtered out as "challenged".
	{
		use crate::{abi::DisputeGameFactory, challenge_slot_keys, game_is_challenged};
		use alloy::{rpc::types::Filter, sol_types::SolEvent};

		let rollup_addr = Address::from_slice(&factory_addr.0);
		let filter = Filter::new().address(rollup_addr).from_block(from_block).to_block(to_block);
		let logs = op_client.beacon_execution_client.get_logs(&filter).await.expect("get_logs");
		println!("raw logs in {from_block}..={to_block}: {} total", logs.len());

		let candidates: Vec<_> = logs
			.into_iter()
			.filter_map(|log| DisputeGameFactory::DisputeGameCreated::decode_log(&log.inner).ok())
			.map(|log| log.data)
			.filter(|a| game_type_configs.iter().any(|c| c.game_type == a.gameType))
			.collect();
		println!("candidates after game_type filter: {}", candidates.len());

		for ev in &candidates {
			let config = game_type_configs.iter().find(|c| c.game_type == ev.gameType).unwrap();
			let slot = challenge_slot_keys(&config.kind).into_iter().next();
			let slot_value = match slot {
				None => alloy::primitives::U256::ZERO,
				Some(s) => op_client
					.beacon_execution_client
					.get_storage_at(
						ev.disputeProxy,
						alloy::primitives::U256::from_be_slice(s.as_slice()),
					)
					.block_id(to_block.into())
					.await
					.expect("get_storage_at"),
			};
			let challenged = game_is_challenged(&config.kind, slot_value);
			println!(
				"  proxy={:?} gameType={} rootClaim={:?} kind={:?} slotValue=0x{} => challenged={}",
				ev.disputeProxy,
				ev.gameType,
				ev.rootClaim,
				config.kind,
				hex::encode(slot_value.to_be_bytes::<32>()),
				challenged,
			);
		}
	}

	let events = op_client
		.latest_dispute_games(from_block, to_block, game_type_configs.clone())
		.await
		.expect("latest_dispute_games");
	println!("latest_dispute_games returned {} events", events.len());
	for ev in &events {
		println!(
			"  proxy={:?} gameType={} rootClaim={:?}",
			ev.disputeProxy, ev.gameType, ev.rootClaim
		);
	}
	assert!(
		!events.is_empty(),
		"latest_dispute_games returned no events for {}..{} — the challenge filter is dropping a valid unchallenged game",
		from_block,
		to_block,
	);

	// Log the L2 block number backing the latest (newest) dispute game event, alongside the
	// current L2 head, so the distance to the RPC proof window is visible.
	if let Some(latest) = events.last() {
		use crate::abi::FaultDisputeGame;
		let contract =
			FaultDisputeGame::new(latest.disputeProxy, &*op_client.beacon_execution_client);
		let extra_data =
			contract.extraData().block(BlockId::latest()).call().await.expect("extraData");
		let l2_block_num: u64 = alloy::primitives::U256::from_be_slice(&extra_data[..32])
			.try_into()
			.unwrap_or(u64::MAX);
		let l2_head = op_client.op_execution_client.get_block_number().await.expect("L2 head");
		println!(
			"latest dispute game event: proxy={:?} l2_block_num={} (L2 head={}, distance={})",
			latest.disputeProxy,
			l2_block_num,
			l2_head,
			l2_head.saturating_sub(l2_block_num),
		);
	}

	// Step back a handful of blocks on the L1 head so the state root is past any reorg.
	let head = op_client.beacon_execution_client.get_block_number().await.expect("L1 head");
	let l1_block_num = head.saturating_sub(8);
	let l1_header = op_client
		.beacon_execution_client
		.get_block(BlockId::number(l1_block_num))
		.await
		.expect("L1 block")
		.expect("L1 block exists");
	let l1_state_root = H256::from_slice(l1_header.header.state_root.as_slice());

	let payload = op_client
		.fetch_dispute_game_payload(l1_block_num, game_type_configs.clone(), events)
		.await
		.expect("fetch_dispute_game_payload")
		.expect("payload must be produced");

	let intermediate_state = verify_optimism_dispute_game_proof::<Hasher>(
		payload,
		l1_state_root,
		factory_addr,
		game_type_configs,
		Default::default(),
	)
	.expect("dispute-game proof must verify");

	dbg!(intermediate_state);
}
