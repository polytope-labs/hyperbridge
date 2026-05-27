use alloy_sol_types::SolValue;
use anyhow::anyhow;
use codec::Decode;
use futures::stream::StreamExt;
use hex_literal::hex;
use ismp_abi::{ecdsa_beefy::BeefyConsensusState, sp1_beefy::SP1BeefyProof};
use pallet_ismp::{ConsensusDigest, TimestampDigest, ISMP_ID, ISMP_TIMESTAMP_ID};
use polkadot_sdk::*;
use serde::Deserialize;
use sp_consensus_beefy::{ecdsa_crypto::Signature, VersionedFinalityProof};
use sp_runtime::{generic::Header as SubstrateHeader, traits::BlakeTwo256, DigestItem};

/// Decode the (timestamp, overlayRoot, stateRoot) the on-chain SP1Beefy will
/// produce for a parachain header, by parsing its ISMP digests the same way
/// `HeaderImpl.stateCommitment` does: the overlay/state roots come from the
/// `ISMP` consensus digest, and the timestamp (seconds) from the `ISTM`
/// `TimestampDigest` consensus digest deposited by pallet-ismp.
fn expected_commitment(header_bytes: &[u8]) -> anyhow::Result<(u64, u32, [u8; 32], [u8; 32])> {
	let header = SubstrateHeader::<u32, BlakeTwo256>::decode(&mut &*header_bytes)
		.map_err(|e| anyhow!("decode parachain header: {e}"))?;
	let mut overlay = None::<[u8; 32]>;
	let mut state = None::<[u8; 32]>;
	let mut timestamp = None::<u64>;
	for log in header.digest.logs.iter() {
		match log {
			DigestItem::Consensus(id, value) if id == &ISMP_ID => {
				let d = ConsensusDigest::decode(&mut &value[..])
					.map_err(|e| anyhow!("decode ISMP ConsensusDigest: {e}"))?;
				overlay = Some(d.mmr_root.0);
				state = Some(d.child_trie_root.0);
			},
			DigestItem::Consensus(id, value) if id == &ISMP_TIMESTAMP_ID => {
				let d = TimestampDigest::decode(&mut &value[..])
					.map_err(|e| anyhow!("decode ISMP TimestampDigest: {e}"))?;
				timestamp = Some(d.timestamp);
			},
			_ => {},
		}
	}
	Ok((
		timestamp.ok_or_else(|| anyhow!("no ISMP timestamp digest"))?,
		header.number,
		overlay.ok_or_else(|| anyhow!("no ISMP consensus digest"))?,
		state.ok_or_else(|| anyhow!("no ISMP consensus digest"))?,
	))
}
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

	// Local SP1 prover (single GPU). ClusterProver requires moongate + redis.
	let sp1_prover = sp1_beefy::local::LocalProver::new().await.unwrap();

	let prover = crate::Prover::new(
		beefy_prover::Prover {
			beefy_activation_block: activation_block,
			relay: relay.clone(),
			relay_rpc: relay_rpc.clone(),
			relay_rpc_client: relay_rpc_client.clone(),
			para,
			para_rpc,
			para_rpc_client,
			para_ids: vec![para_id],
			query_batch_size: None,
		},
		sp1_prover,
		// Commit Bob's well-known sr25519 dev account as the nonce so the generated fixture is
		// usable by the pallet tests, which require `committed nonce == submit_proof signer`
		// (the simtest signs as Bob).
		primitive_types::H256(hex!("8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48")),
	);

	// Get initial consensus state — also the "previousState" the on-chain verify() consumes.
	let consensus_state = prover.inner.get_initial_consensus_state(None).await?;
	let previous_state =
		hex::encode(BeefyConsensusState::from(consensus_state.clone()).abi_encode());
	println!("\n=== Initial Consensus State (ABI-encoded) ===");
	println!("0x{}", previous_state);
	println!("\n=== Consensus State Details ===");
	println!("{:#?}", consensus_state);
	println!("==============================================\n");

	let mut subscription: RpcSubscription<String> = prover
		.inner
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

		if signed_commitment.commitment.validator_set_id < consensus_state.current_authorities.id {
			println!(
				"Skipping outdated commitment validator_set_id={} current={}",
				signed_commitment.commitment.validator_set_id,
				consensus_state.current_authorities.id,
			);
			continue;
		}

		println!(
			"\nGenerating SP1 proof for block={} validator_set_id={} ...",
			signed_commitment.commitment.block_number,
			signed_commitment.commitment.validator_set_id,
		);

		let sp1_proof: SP1BeefyProof = prover
			.consensus_proof(signed_commitment.clone(), consensus_state.clone())
			.await?;

		let encoded_proof = hex::encode(sp1_proof.abi_encode_params());
		println!("\n=== SP1Beefy Proof (ABI-encoded SP1BeefyProof) ===");
		println!("0x{}", encoded_proof);
		println!("==============================================\n");

		// Cross-check expectations the contract should emit, derived from the same
		// header digests HeaderImpl.stateCommitment parses on-chain.
		let mut expected = Vec::with_capacity(sp1_proof.headers.len());
		for header in &sp1_proof.headers {
			let (ts, height, overlay, state) = expected_commitment(&header.header)?;
			expected.push(serde_json::json!({
				"state_machine_id": header.id.to::<u64>(),
				"height": height,
				"timestamp": ts,
				"overlay_root": format!("0x{}", hex::encode(overlay)),
				"state_root": format!("0x{}", hex::encode(state)),
			}));
		}

		let fixture = serde_json::json!({
			"block_number": signed_commitment.commitment.block_number,
			"validator_set_id": signed_commitment.commitment.validator_set_id,
			"para_id": para_id,
			"previous_state": format!("0x{}", previous_state),
			"proof": format!("0x{}", encoded_proof),
			"intermediates_expected": expected,
		});
		let out_path =
			std::env::var("FIXTURE_OUT").unwrap_or_else(|_| "/tmp/sp1_beefy_fixture.json".into());
		tokio::fs::write(&out_path, serde_json::to_string_pretty(&fixture)?).await?;
		println!("Fixture written to {}", out_path);
		break;
	}

	Ok(())
}
