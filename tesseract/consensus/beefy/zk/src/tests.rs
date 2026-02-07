use anyhow::anyhow;
use codec::Decode;
use ethers::abi::{AbiEncode, Token, Tokenizable};
use futures::stream::StreamExt;
use hex_literal::hex;
use ismp::messaging::Message;
use ismp_solidity_abi::beefy::{BeefyConsensusProof, BeefyConsensusState};
use serde::Deserialize;
use sp_consensus_beefy::{ecdsa_crypto::Signature, VersionedFinalityProof};
use subxt::{
	backend::legacy::LegacyRpcMethods,
	config::{Hasher, Header},
	ext::subxt_rpcs::{client::RpcSubscription, rpc_params},
	PolkadotConfig,
};
use subxt_utils::Hyperbridge;
use tesseract_evm::transport::RpcTransport;
use tesseract_primitives::IsmpProvider;
use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};

fn default_para_id() -> u32 {
	3367
}

fn default_relay_ws_url() -> String {
	"wss://rpc.stakeworld.io:443".to_string()
}

fn default_para_ws_url() -> String {
	"wss://nexus.ibp.network:443".to_string()
}

#[derive(Deserialize, Debug)]
struct Config {
	#[serde(default = "default_relay_ws_url")]
	relay_ws_url: String,
	#[serde(default = "default_para_ws_url")]
	para_ws_url: String,
	#[serde(default = "default_para_id")]
	para_id: u32,
}

pub fn setup() -> Result<(), anyhow::Error> {
	let filter =
		tracing_subscriber::EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());
	tracing_subscriber::fmt().with_env_filter(filter).finish().try_init()?;

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_sp1_beefy() -> Result<(), anyhow::Error> {
	setup()?;

	// first compile the project.
	let config = envy::from_env::<Config>()?;

	dbg!(&config);

	let Config { relay_ws_url, para_ws_url, para_id } = config;
	let (relay, relay_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, u32::MAX).await?;
	let (para, para_rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&para_ws_url, u32::MAX).await?;

	let para_rpc = LegacyRpcMethods::<Hyperbridge>::new(para_rpc_client.clone());

	let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());

	let metadata = relay.metadata();
	let hasher = <PolkadotConfig as subxt::Config>::Hasher::new(&metadata);

	let header = relay_rpc
		.chain_get_header(None)
		.await?
		.ok_or_else(|| anyhow!("No blocks on the relay chain?"))?;

	let header_hash = header.hash_with(hasher);

	let leaves = relay_rpc
		.state_get_storage(
			hex!("a8c65209d47ee80f56b0011e8fd91f508156209906244f2341137c136774c91d").as_slice(),
			Some(header_hash),
		)
		.await?
		.map(|data| u64::decode(&mut data.as_ref()))
		.transpose()?
		.ok_or_else(|| anyhow!("Couldn't fetch latest beefy authority set"))?;

	let activation_block = header.number - leaves as u32;

	para.blocks()
		.subscribe_best()
		.await
		.unwrap()
		.skip_while(|result| {
			futures::future::ready({
				match result {
					Ok(block) => block.number() < 5,
					Err(_) => false,
				}
			})
		})
		.take(1)
		.collect::<Vec<_>>()
		.await;

	println!("Parachains Onboarded");

	// ============================================================================
	// ZK Prover Setup (Commented out - using naive prover instead)
	// ============================================================================
	// let sp1_prover = sp1_beefy::cluster::ClusterProver::new(
	// 	"http://127.0.0.1:50051".to_string(),
	// 	"redis://:redispassword@127.0.0.1:6379".to_string(),
	// )
	// .await?;

	// let sp1_prover = sp1_beefy::local::LocalProver::new(true);

	// let prover = Prover::new(
	// 	beefy_prover::Prover {
	// 		beefy_activation_block: activation_block,
	// 		relay: relay.clone(),
	// 		relay_rpc: relay_rpc.clone(),
	// 		relay_rpc_client: relay_rpc_client.clone(),
	// 		para,
	// 		para_rpc,
	// 		para_rpc_client,
	// 		para_ids: vec![para_id],
	// 		query_batch_size: None,
	// 	},
	// 	sp1_prover,
	// );

	// ============================================================================
	// Naive Prover Setup (Active)
	// ============================================================================
	let prover = beefy_prover::Prover {
		beefy_activation_block: activation_block,
		relay: relay.clone(),
		relay_rpc: relay_rpc.clone(),
		relay_rpc_client: relay_rpc_client.clone(),
		para,
		para_rpc,
		para_rpc_client,
		para_ids: vec![para_id],
		query_batch_size: None,
	};

	// Get initial consensus state
	let consensus_state = prover.get_initial_consensus_state(None).await?;

	// Log the ABI-encoded BeefyConsensusState
	let encoded_consensus_state = BeefyConsensusState::from(consensus_state.clone()).encode_hex();
	println!("\n=== Initial Consensus State (ABI-encoded) ===");
	println!("0x{}", encoded_consensus_state);
	println!("\n=== Consensus State Details ===");
	println!("{:#?}", consensus_state);
	println!("==============================================\n");

	let mut subscription: RpcSubscription<String> = prover
		.relay_rpc_client
		.subscribe(
			"beefy_subscribeJustifications",
			rpc_params![],
			"beefy_unsubscribeJustifications",
		)
		.await?;

	while let Some(Ok(commitment)) = subscription.next().await {
		let commitment = hex::decode(&commitment[2..])?;
		let VersionedFinalityProof::V1(signed_commitment) =
			VersionedFinalityProof::<u32, Signature>::decode(&mut &*commitment)?;

		match signed_commitment.commitment.validator_set_id {
			id if id < consensus_state.current_authorities.id => {
				// If validator set id of signed commitment is less than current validator set id we
				// have Then commitment is outdated and we skip it.
				println!(
					"Skipping outdated commitment \n Received signed commitmment with
	validator_set_id: {:?}\n Current authority set id: {:#?}\n Next authority set id: {:?}\n",
					signed_commitment.commitment.validator_set_id,
					consensus_state.current_authorities.id,
					consensus_state.current_authorities.id
				);
				continue;
			},
			_ => {},
		};

		// Naive prover consensus proof
		let proof: BeefyConsensusProof =
			prover.consensus_proof(signed_commitment.clone()).await?.into();

		println!("\n=== Consensus proof (ABI-encoded) ===");
		println!("0x{}", hex::encode([&[0u8], AbiEncode::encode(proof).as_slice()].concat()));
		println!("==============================================\n");

		// ============================================================================
		// ZK Prover Call (Commented out)
		// ============================================================================
		// prover
		// 	.consensus_proof(signed_commitment.clone(), consensus_state.clone())
		// 	.await?;
	}

	Ok(())
}

#[derive(Deserialize, Debug)]
struct TronTestConfig {
	#[serde(default = "default_relay_ws_url")]
	relay_ws_url: String,
	#[serde(default = "default_para_ws_url")]
	para_ws_url: String,
	#[serde(default = "default_para_id")]
	para_id: u32,
	/// Deployed TronHost contract address (hex, 41-prefixed or 0x-prefixed).
	/// If absent, the test only prints the consensus state for deployment.
	tron_host_address: Option<String>,
	/// TRE account private key (hex, 32 bytes).
	#[serde(alias = "PRIVATE_KEY")]
	private_key: String,
	/// TRON native HTTP API URL (for tx submission).
	/// API keys can be included in the URL if needed.
	#[serde(alias = "TRON_HOST")]
	tron_api_url: String,
	/// TRON native API key (for tx submission).
	#[serde(alias = "TRON_API_KEY")]
	tron_api_key: Option<String>,
}

/// Encode a FiatShamir consensus proof in the format expected by
/// `ConsensusRouter` on-chain: `[0x02] ++ abi.encode(relay, parachain, bitmap)`.
fn encode_fiat_shamir_proof(
	consensus_message: beefy_verifier_primitives::ConsensusMessage,
	bitmap: beefy_prover::fiat_shamir::SignersBitmap,
) -> Vec<u8> {
	let proof: BeefyConsensusProof = consensus_message.into();
	let bitmap_token = Token::FixedArray(
		bitmap
			.words
			.iter()
			.map(|w| {
				let buf = w.to_big_endian();
				Token::Uint(ethers::types::U256::from_big_endian(&buf))
			})
			.collect(),
	);
	let encoded = ethers::abi::encode(&[
		proof.relay.into_token(),
		proof.parachain.into_token(),
		bitmap_token,
	]);
	[&[0x02u8], encoded.as_slice()].concat()
}

/// Integration test: deploy to TRE, submit FiatShamir BEEFY consensus updates.
///
/// # Two-phase workflow
///
/// **Phase 1** — Run without `TRON_HOST_ADDRESS`:
///   Connects to live Polkadot relay + Hyperbridge parachain, fetches the
///   initial consensus state and prints it as a hex string.  Use this value
///   as `CONSENSUS_STATE` when deploying contracts via TronBox.
///
/// **Phase 2** — Run with `TRON_HOST_ADDRESS=<deployed TronHost>`:
///   Creates a [`TronClient`] pointing at TRE, subscribes to BEEFY
///   justifications, generates FiatShamir proofs, and submits them to the
///   deployed `TronHost` contract.
///
/// # Environment variables
///
/// | Variable             | Default                           | Description                        |
/// |----------------------|-----------------------------------|------------------------------------|
/// | `RELAY_WS_URL`       | `wss://rpc.stakeworld.io:443`     | Polkadot relay chain WS RPC        |
/// | `PARA_WS_URL`        | `wss://nexus.ibp.network:443`     | Hyperbridge parachain WS RPC       |
/// | `PARA_ID`            | `3367`                            | Hyperbridge parachain ID           |
/// | `TRON_HOST_ADDRESS`  | *(none)*                          | Deployed TronHost hex address      |
/// | `TRON_PRIVATE_KEY`   | TRE deterministic key             | Relayer private key (hex)          |
/// | `TRON_API_URL`       | `http://127.0.0.1:9090`           | TRON native API (tx submission)    |
/// | `TRON_RPC_URL`       | `http://127.0.0.1:9090/jsonrpc`   | TRON JSON-RPC (reads via EvmClient)|
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_tron_fiat_shamir() -> Result<(), anyhow::Error> {
	setup()?;

	let config = envy::from_env::<TronTestConfig>()?;
	dbg!(&config);

	let TronTestConfig {
		relay_ws_url,
		para_ws_url,
		para_id,
		tron_host_address,
		private_key,
		tron_api_url,
		tron_api_key,
	} = config;

	// Derive the JSON-RPC URL from the API URL
	let tron_rpc_url = format!("{}/jsonrpc", tron_api_url);

	// Connect to relay chain and parachain
	let (relay, relay_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, u32::MAX).await?;
	let (para, para_rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&para_ws_url, u32::MAX).await?;

	let para_rpc = LegacyRpcMethods::<Hyperbridge>::new(para_rpc_client.clone());
	let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());
	let metadata = relay.metadata();
	let hasher = <PolkadotConfig as subxt::Config>::Hasher::new(&metadata);

	let header = relay_rpc
		.chain_get_header(None)
		.await?
		.ok_or_else(|| anyhow!("No blocks on the relay chain?"))?;

	let header_hash = header.hash_with(hasher);

	let leaves = relay_rpc
		.state_get_storage(
			hex!("a8c65209d47ee80f56b0011e8fd91f508156209906244f2341137c136774c91d").as_slice(),
			Some(header_hash),
		)
		.await?
		.map(|data| u64::decode(&mut data.as_ref()))
		.transpose()?
		.ok_or_else(|| anyhow!("Couldn't fetch latest beefy authority set"))?;

	let activation_block = header.number - leaves as u32;

	// Wait for parachain to produce blocks
	para.blocks()
		.subscribe_best()
		.await
		.unwrap()
		.skip_while(|result| {
			futures::future::ready(match result {
				Ok(block) => block.number() < 5,
				Err(_) => false,
			})
		})
		.take(1)
		.collect::<Vec<_>>()
		.await;

	println!("Parachains onboarded");

	// Set up the BEEFY prover
	let prover = beefy_prover::Prover {
		beefy_activation_block: activation_block,
		relay: relay.clone(),
		relay_rpc: relay_rpc.clone(),
		relay_rpc_client: relay_rpc_client.clone(),
		para,
		para_rpc,
		para_rpc_client,
		para_ids: vec![para_id],
		query_batch_size: None,
	};

	// Phase 1: Get initial consensus state
	let consensus_state = prover.get_initial_consensus_state(None).await?;

	let encoded_consensus_state = BeefyConsensusState::from(consensus_state.clone()).encode_hex();
	println!("\n=== Initial Consensus State (ABI-encoded) ===");
	println!("CONSENSUS_STATE={encoded_consensus_state}");
	println!("\n=== Consensus State Details ===");
	println!("  latest_beefy_height: {}", consensus_state.latest_beefy_height);
	println!("  current_authorities.id: {}", consensus_state.current_authorities.id);
	println!("  current_authorities.len: {}", consensus_state.current_authorities.len);
	println!("  next_authorities.id: {}", consensus_state.next_authorities.id);
	println!("  next_authorities.len: {}", consensus_state.next_authorities.len);
	println!("================================================\n");

	// Phase 2: If TRON_HOST_ADDRESS is set, submit FiatShamir proofs
	let host_address = match tron_host_address {
		Some(addr) => addr,
		None => {
			println!("TRON_HOST_ADDRESS not set.");
			println!("Deploy contracts to TRE with the CONSENSUS_STATE above, then re-run:");
			println!("  TRON_HOST_ADDRESS=<TronHost hex address> cargo test -p zk-beefy test_tron_fiat_shamir -- --ignored --nocapture");
			return Ok(());
		},
	};

	println!("Connecting to TRON at {} (API) / {} (RPC) ...", tron_api_url, tron_rpc_url);
	println!("TronHost address: {host_address}");

	// Parse the host address into H160 for EvmConfig
	let host_hex = host_address
		.strip_prefix("0x")
		.or_else(|| host_address.strip_prefix("41"))
		.unwrap_or(&host_address);
	let host_bytes = hex::decode(host_hex)?;
	let ismp_host = sp_core::H160::from_slice(&host_bytes);

	// Build the TronConfig
	let tron_config = tesseract_tron::TronConfig {
		evm: tesseract_evm::EvmConfig {
			rpc_urls: vec![tron_rpc_url],
			state_machine: ismp::host::StateMachine::Evm(tesseract_tron::TRON_MAINNET_CHAIN_ID),
			consensus_state_id: "BEEF".into(),
			ismp_host,
			signer: private_key,
			tracing_batch_size: None,
			query_batch_size: None,
			poll_interval: None,
			gas_price_buffer: None,
			client_type: None,
			initial_height: Some(1),
			transport: RpcTransport::Tron,
		},
		tron_api_key,
		tron_api_url,
		fee_limit: 1_000_000_000, // 1000 TRX
		tron_api_timeout_secs: 180,
	};

	let tron_client = tesseract_tron::TronClient::new(tron_config).await?;
	println!("TronClient initialized: {}", tron_client.name());

	// Subscribe to BEEFY justifications
	let mut subscription: RpcSubscription<String> = prover
		.relay_rpc_client
		.subscribe(
			"beefy_subscribeJustifications",
			rpc_params![],
			"beefy_unsubscribeJustifications",
		)
		.await?;

	println!("Subscribed to BEEFY justifications, waiting for a finality proof...\n");

	while let Some(Ok(commitment)) = subscription.next().await {
		let consensus_state_bytes =
			tron_client.evm.query_consensus_state(None, Default::default()).await?;

		let consensus_state =
			beefy_verifier_primitives::ConsensusState::decode(&mut &*consensus_state_bytes)?;

		log::info!("Consensus state: {:#?}", consensus_state);

		let commitment_bytes = hex::decode(&commitment[2..])?;
		let VersionedFinalityProof::V1(signed_commitment) =
			VersionedFinalityProof::<u32, Signature>::decode(&mut &*commitment_bytes)?;

		// Skip outdated commitments
		if signed_commitment.commitment.validator_set_id < consensus_state.current_authorities.id {
			println!(
				"Skipping outdated commitment (set_id={}, current={})",
				signed_commitment.commitment.validator_set_id,
				consensus_state.current_authorities.id,
			);
			continue;
		}

		println!(
			"Got BEEFY justification at block {} (set_id={})",
			signed_commitment.commitment.block_number,
			signed_commitment.commitment.validator_set_id,
		);

		// Generate FiatShamir consensus proof
		let (consensus_message, bitmap) = prover
			.consensus_proof_fiat_shamir(signed_commitment.clone(), &consensus_state)
			.await?;

		let proof_bytes = encode_fiat_shamir_proof(consensus_message, bitmap);
		println!("FiatShamir proof encoded: {} bytes", proof_bytes.len());

		// Wrap in ISMP Message and submit via TronClient
		let ismp_message = Message::Consensus(ismp::messaging::ConsensusMessage {
			consensus_proof: proof_bytes,
			consensus_state_id: *b"BEEF",
			signer: tron_client.inner().address.clone(),
		});

		println!("Submitting FiatShamir proof to TronHost via TronClient...");
		let result = tron_client
			.submit(vec![ismp_message], ismp::host::StateMachine::Polkadot(para_id))
			.await;

		match result {
			Ok(tx_result) => {
				println!("\n=== Submission Result ===");
				println!("  receipts: {}", tx_result.receipts.len());
				println!("  unsuccessful: {}", tx_result.unsuccessful.len());
				println!("=========================\n");
			},
			Err(err) => {
				println!("\n=== Submission Failed ===");
				println!("  error: {err:#}");
				println!("=========================\n");
				return Err(err);
			},
		}

		// One successful submission is enough for the test
		println!("FiatShamir consensus update submitted successfully to TRON!");
	}

	Ok(())
}
