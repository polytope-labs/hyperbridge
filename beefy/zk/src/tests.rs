use crate::Prover;
use anyhow::anyhow;
use codec::Decode;
use futures::stream::StreamExt;
use hex_literal::hex;
use serde::Deserialize;
use sp_consensus_beefy::{ecdsa_crypto::Signature, VersionedFinalityProof};
use subxt::{config::Header, rpc::Subscription, rpc_params, PolkadotConfig};
use subxt_utils::Hyperbridge;
use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};

fn default_para_id() -> u32 {
	3367
}

fn default_relay_ws_url() -> String {
	"wss://rpc.ibp.network/polkadot:443".to_string()
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
	let relay = subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, u32::MAX).await?;
	let para = subxt_utils::client::ws_client::<Hyperbridge>(&para_ws_url, u32::MAX).await?;

	let header = relay
		.rpc()
		.header(None)
		.await?
		.ok_or_else(|| anyhow!("No blocks on the relay chain?"))?;

	let leaves = relay
		.rpc()
		.storage(
			hex!("a8c65209d47ee80f56b0011e8fd91f508156209906244f2341137c136774c91d").as_slice(),
			Some(header.hash()),
		)
		.await?
		.map(|data| u64::decode(&mut data.as_ref()))
		.transpose()?
		.ok_or_else(|| anyhow!("Couldn't fetch latest beefy authority set"))?;

	let activation_block = header.number() - leaves as u32;

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
		relay,
		para,
		para_ids: vec![para_id],
	});
	let consensus_state = prover.inner.get_initial_consensus_state().await?;
	let mut subscription: Subscription<String> = prover
		.inner
		.relay
		.rpc()
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
                    "Skipping outdated commitment \n Received signed commitmment with validator_set_id: {:?}\n Current authority set id: {:#?}\n Next authority set id: {:?}\n",
                    signed_commitment.commitment.validator_set_id, consensus_state.current_authorities.id, consensus_state.current_authorities.id
                );
				continue;
			},
			_ => {},
		};

		prover
			.consensus_proof(signed_commitment.clone(), consensus_state.clone())
			.await?;
	}

	Ok(())
}
