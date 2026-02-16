use anyhow::anyhow;
use codec::Decode;
use ethers::abi::AbiEncode;
use futures::stream::StreamExt;
use hex_literal::hex;
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
