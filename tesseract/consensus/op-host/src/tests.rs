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
use op_verifier::{
	verify_optimism_dispute_game_proof, DisputeGameImpl, GameTypeConfig,
};
use tesseract_evm::EvmConfig;
use tesseract_primitives::Hasher;

const MESSAGE_PARSER: [u8; 20] = hex!("4200000000000000000000000000000000000016");

/// Placeholder secp256k1 key for read-only tests. `EvmClient::new` requires a valid signer to
/// construct, but none of the consensus-verification flow this file exercises actually signs or
/// sends transactions.
const DUMMY_SIGNING_KEY: &str =
	"0000000000000000000000000000000000000000000000000000000000000001";
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
	};
	let evm_config = EvmConfig {
		rpc_urls: vec![l2_url],
		consensus_state_id: "ETH0".to_string(),
		signer: DUMMY_SIGNING_KEY.to_string(),
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
		disputeProxy: Address::from_slice(&hex!(
			"2c9ece6a5856ab7f5f2c49072e9f7a4f00d4c1e6"
		)),
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

/// End-to-end verification of a Cannon (gameType 0) dispute game created on Ethereum mainnet:
///
/// ```text
/// DisputeGameCreated(
///     proxy:     0xE6512d19E2bac97A2Ed17e2cC1C5Df96E29d3EA8,
///     gameType:  0,
///     rootClaim: 0x76B0808A7D3244F52677F7FEC036A25FAAD2FB80EE4D9504C5458775A7024FFA,
/// )
/// ```
///
/// Factory: `0xe5965Ab5962eDc7477C8520243A95517CD252fA9` on L1 mainnet. Requires
/// `MAINNET_RPC_URL` (L1) and `OP_MAINNET_RPC_URL` (L2 Optimism mainnet) in the environment.
#[tokio::test]
#[ignore]
async fn test_cannon_dispute_game_verification() {
	dotenv::dotenv().ok();
	let l1_url = std::env::var("MAINNET_RPC_URL")
		.expect("MAINNET_RPC_URL must be set to an Ethereum mainnet RPC endpoint");
	let l2_url = std::env::var("OP_MAINNET_RPC_URL")
		.expect("OP_MAINNET_RPC_URL must be set to an Optimism mainnet RPC endpoint");

	let event = DisputeGameCreated {
		disputeProxy: Address::from_slice(&hex!(
			"e6512d19e2bac97a2ed17e2cc1c5df96e29d3ea8"
		)),
		gameType: 0,
		rootClaim: B256::from(hex!(
			"76b0808a7d3244f52677f7fec036a25faad2fb80ee4d9504c5458775a7024ffa"
		)),
	};
	let factory_addr = H160::from(hex!("e5965ab5962edc7477c8520243a95517cd252fa9"));
	let game_type_configs = vec![GameTypeConfig {
		game_type: 0,
		// Cannon implementation pinned in the migration module.
		expected_impl: H160::from(hex!("6dDBa09bc4cCB0D6Ca9Fc5350580f74165707499")),
		kind: DisputeGameImpl::FaultDisputeGame,
	}];

	// Ethereum mainnet chain id = 1.
	run_dispute_game_verification(l1_url, l2_url, 1, factory_addr, event, game_type_configs)
		.await;
}
