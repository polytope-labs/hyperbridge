#![cfg(test)]

use std::collections::HashMap;

use anyhow::anyhow;
use codec::Encode;
use futures::StreamExt;
use ismp::host::StateMachine;
use pallet_ismp_rpc::BlockNumberOrHash;
use staging_xcm::{
	v3::{Junction, Junctions, MultiLocation, NetworkId, WeightLimit},
	VersionedMultiAssets, VersionedMultiLocation,
};
use subxt::{
	config::Header,
	ext::sp_core::{bytes::from_hex, sr25519, Pair, H256},
	rpc_params,
	tx::TxPayload,
	OnlineClient, PolkadotConfig,
};
use subxt_utils::{send_extrinsic, Extrinsic, Hyperbridge, InMemorySigner};

const SEND_AMOUNT: u128 = 2_000_000_000_000;

#[ignore]
#[tokio::test]
async fn should_dispatch_ismp_request_when_xcm_is_received() -> anyhow::Result<()> {
	dotenv::dotenv().ok();
	let private_key = std::env::var("SUBSTRATE_SIGNING_KEY").ok().unwrap_or(
		"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
	);
	let seed = from_hex(&private_key)?;
	let pair = sr25519::Pair::from_seed_slice(&seed)?;
	let signer = InMemorySigner::<PolkadotConfig>::new(pair.clone());
	let url = std::env::var("ROCOCO_LOCAL_URL")
		.ok()
		.unwrap_or("ws://127.0.0.1:9922".to_string());
	let client = OnlineClient::<PolkadotConfig>::from_url(&url).await?;

	let para_url = std::env::var("PARA_LOCAL_URL")
		.ok()
		.unwrap_or("ws://127.0.0.1:9990".to_string());
	let para_client = OnlineClient::<Hyperbridge>::from_url(&para_url).await?;

	// Wait for parachain block production

	let sub = para_client.rpc().subscribe_all_block_headers().await?;
	let _block = sub
		.take(1)
		.collect::<Vec<_>>()
		.await
		.into_iter()
		.collect::<Result<Vec<_>, _>>()?;
	let beneficiary: MultiLocation = Junctions::X3(
		Junction::AccountId32 { network: None, id: pair.public().into() },
		Junction::AccountKey20 {
			network: Some(NetworkId::Ethereum { chain_id: 1 }),
			key: [1u8; 20],
		},
		Junction::GeneralIndex(60 * 60),
	)
	.into_location();
	let weight_limit = WeightLimit::Unlimited;

	let dest: MultiLocation = Junction::Parachain(2000).into();

	let call = (
		Box::<VersionedMultiLocation>::new(dest.clone().into()),
		Box::<VersionedMultiLocation>::new(beneficiary.clone().into()),
		Box::<VersionedMultiAssets>::new((Junctions::Here, SEND_AMOUNT).into()),
		0,
		weight_limit,
	);

	{
		let signer = InMemorySigner::<PolkadotConfig>::new(pair.clone());
		// Force set the xcm version to our supported version
		let encoded_call =
			Extrinsic::new("XcmPallet", "force_xcm_version", (Box::new(dest.clone()), 3).encode())
				.encode_call_data(&client.metadata())?;
		let tx = Extrinsic::new("Sudo", "sudo", encoded_call);
		send_extrinsic(&client, signer, tx).await?;
	}

	let ext = Extrinsic::new(
		"XcmPallet".to_string(),
		"limited_reserve_transfer_assets".to_string(),
		call.encode(),
	);

	let init_block = para_client
		.rpc()
		.header(None)
		.await?
		.ok_or_else(|| anyhow!("Failed to fetch latest header"))?
		.number();

	send_extrinsic(&client, signer, ext).await?;

	let mut sub = para_client.rpc().subscribe_finalized_block_headers().await?;

	let mut prev_block = init_block;
	while let Some(res) = sub.next().await {
		match res {
			Ok(header) => {
				// Break if we've waited too long
				if header.number().saturating_sub(init_block) >= 500 {
					Err(anyhow!("XCM Integration test failed: Post request event was not found"))?
				}

				let params = rpc_params![
					BlockNumberOrHash::<H256>::Number(prev_block),
					BlockNumberOrHash::<H256>::Number(header.number())
				];

				let response: HashMap<String, Vec<ismp::events::Event>> =
					para_client.rpc().request("ismp_queryEvents", params).await?;

				let events = response.values().into_iter().cloned().flatten().collect::<Vec<_>>();
				if let Some(post) = events.into_iter().find_map(|ev| match ev {
					ismp::events::Event::PostRequest(post) => Some(post),
					_ => None,
				}) {
					dbg!(&post);

					// Assert that this is the post we sent
					assert_eq!(post.nonce, 0);
					assert_eq!(
						post.dest,
						StateMachine::Ethereum(ismp::host::Ethereum::ExecutionLayer)
					);
					assert_eq!(post.source, StateMachine::Kusama(2000));
					return Ok(());
				}
				prev_block = header.number() + 1;
			},

			Err(err) => {
				println!("{err:?}")
			},
		}
	}

	Err(anyhow!("XCM Integration test failed"))
}
