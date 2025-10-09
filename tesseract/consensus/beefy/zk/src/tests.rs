use crate::Prover;
use anyhow::anyhow;
use codec::Decode;
use futures::stream::StreamExt;
use hex_literal::hex;
use ismp_solidity_abi::beefy::BeefyConsensusState;
use serde::Deserialize;
use sp_consensus_beefy::{ecdsa_crypto::Signature, VersionedFinalityProof};
use subxt::{
	backend::legacy::LegacyRpcMethods,
	config::{Hasher, Header},
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

	dbg!(header_hash);
	dbg!(header.number);
	dbg!(leaves);

	let activation_block = header.number - leaves as u32;

	dbg!(activation_block);

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

	let prover = Prover::new(beefy_prover::Prover {
		beefy_activation_block: activation_block,
		relay: relay.clone(),
		relay_rpc: relay_rpc.clone(),
		relay_rpc_client: relay_rpc_client.clone(),
		para,
		para_rpc,
		para_rpc_client,
		para_ids: vec![para_id],
		query_batch_size: None,
	});

	let consensus_state_bytes = hex!("000000000000000000000000000000000000000000000000000000000183db1000000000000000000000000000000000000000000000000000000000012a531800000000000000000000000000000000000000000000000000000000000009980000000000000000000000000000000000000000000000000000000000000258bea1ea741f3a85e9d200e3dc6fd7d929a82c313566c2e3ed8e75cd141b58498300000000000000000000000000000000000000000000000000000000000009990000000000000000000000000000000000000000000000000000000000000258bea1ea741f3a85e9d200e3dc6fd7d929a82c313566c2e3ed8e75cd141b584983");
	let consensus_state =
		<BeefyConsensusState as ethers::core::abi::AbiDecode>::decode(&consensus_state_bytes)
			.unwrap();

	let block_hash =
		relay_rpc.chain_get_block_hash(Some(25420895u64.into())).await.unwrap().unwrap();
	let (_, proof) = relay_rpc
		.chain_get_block(Some(block_hash))
		.await
		.unwrap()
		.unwrap()
		.justifications
		.expect("No justifications found")
		.into_iter()
		.find(|justfication| justfication.0 == sp_consensus_beefy::BEEFY_ENGINE_ID)
		.expect("No beefy justification found");

	let VersionedFinalityProof::V1(signed_commitment) =
		VersionedFinalityProof::<u32, Signature>::decode(&mut &*proof)?;

	prover
		.consensus_proof(signed_commitment.clone(), consensus_state.into())
		.await?;

	// let consensus_state = prover.inner.get_initial_consensus_state().await?;
	// let mut subscription: Subscription<String> = prover
	// 	.inner
	// 	.relay
	// 	.rpc()
	// 	.subscribe(
	// 		"beefy_subscribeJustifications",
	// 		rpc_params![],
	// 		"beefy_unsubscribeJustifications",
	// 	)
	// 	.await?;

	// while let Some(Ok(commitment)) = subscription.next().await {
	// 	let commitment = hex::decode(&commitment[2..])?;
	// 	let VersionedFinalityProof::V1(signed_commitment) =
	// 		VersionedFinalityProof::<u32, Signature>::decode(&mut &*commitment)?;

	// 	match signed_commitment.commitment.validator_set_id {
	// 		id if id < consensus_state.current_authorities.id => {
	// 			// If validator set id of signed commitment is less than current validator set id we
	// 			// have Then commitment is outdated and we skip it.
	// 			println!(
	//                    "Skipping outdated commitment \n Received signed commitmment with
	// validator_set_id: {:?}\n Current authority set id: {:#?}\n Next authority set id: {:?}\n",
	//                    signed_commitment.commitment.validator_set_id,
	// consensus_state.current_authorities.id, consensus_state.current_authorities.id
	// ); 			continue;
	// 		},
	// 		_ => {},
	// 	};

	// 	prover
	// 		.consensus_proof(signed_commitment.clone(), consensus_state.clone())
	// 		.await?;
	// }

	Ok(())
}

// mmr.nodes:11739778
// 0xa291c8362a0ea21efc57eae6e87a2c2d829fb2f319c102c1255e3110bcd51e60

// mmr.nodes:11739777
// 0x3a1754334582d9352eb0d02ad61d7f163bd52169286ba7c440ab16d253bf9884

// mmr.nodes:11739774
// 0xb65c73f960dbb4bdc809fc85fb5a6613fdb3a0bed42aaa224529d0bee4ff4d35

// mmr.nodes:11739767
// 0x795fe3802cf9bdc0817662f45a87a422ce78f6e0f6539299a57c03f38af3f502

// mmr.nodes:11739640
// 0x6417f0a6e5b1b1dc430e5e36149a86cee6139118e7fd908e9017e4f257cd524f

// mmr.nodes:11739129
// 0x6e422f8438eac054afb93c0413a39b12b310af32036ffc19110b6b2fe1c1d99e

// mmr.nodes:11730938
// 0x5d3bd19213d6748570584221ab11e62d2d4ba608de0abf9291a229647a718f2a

// mmr.nodes:11665403
// 0x4bb51a313a2f26255ccbbd216f76321f288db10f17c56dccdf8d9374f2c131d7

// mmr.nodes: 11534332
// 0x3c28e5329be1d4c45923caad77473fee6a37d1e70427e14a3c20982a7c62c6ed

// mmr.nodes: 10485757
// 0x53754b7d6def97becdf7dd0f1c8de010a7c4b39160c44e25bd0a54df36535cb4

// mmr.nodes: 8388606
// 0xd3aae8775e63f0c88c5fb3d55cfe96478a407bda10b33d782ba2e500a122e316
