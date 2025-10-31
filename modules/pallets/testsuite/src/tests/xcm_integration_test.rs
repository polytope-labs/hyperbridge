#![cfg(test)]

use std::{collections::HashMap};

use alloy_sol_types::SolType;
use anyhow::anyhow;
use futures::StreamExt;
use polkadot_sdk::{
	sp_core::{bytes::from_hex, sr25519, Pair, H256},
	*,
};
use subxt::{
	backend::legacy::LegacyRpcMethods,
	config::Header,
	dynamic::Value,
	ext::{
		scale_value::Composite,
		subxt_rpcs::{rpc_params, RpcClient},
	},
	OnlineClient,
};
use subxt::tx::Payload;

use ismp::host::StateMachine;
use pallet_ismp_rpc::BlockNumberOrHash;
use subxt_utils::{send_extrinsic, BlakeSubstrateChain, Hyperbridge, InMemorySigner};

const SEND_AMOUNT: u128 = 2_000_000_000_000;

#[tokio::test]
#[ignore]
async fn should_dispatch_ismp_request_when_xcm_is_received() -> anyhow::Result<()> {
	println!("inside the test");
	dotenv::dotenv().ok();
	let private_key = std::env::var("SUBSTRATE_SIGNING_KEY").ok().unwrap_or(
		"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
	);
	let seed = from_hex(&private_key)?;
	let pair = sr25519::Pair::from_seed_slice(&seed)?;
	let signer = InMemorySigner::<BlakeSubstrateChain>::new(pair.clone());
	println!("connecting to rococo");
	let url = std::env::var("ROCOCO_LOCAL_URL")
		.ok()
		.unwrap_or("ws://127.0.0.1:9922".to_string());
	let relay_client = OnlineClient::<BlakeSubstrateChain>::from_url(&url).await?;
	let rpc_client = RpcClient::from_url(&url).await?;
	let _rpc = LegacyRpcMethods::<BlakeSubstrateChain>::new(rpc_client.clone());
	println!("connecting to asset hub");
	let assethub_url = std::env::var("ASSET_HUB_LOCAL_URL")
		.ok()
		.unwrap_or("ws://127.0.0.1:9910".to_string());
	let assethub_client = OnlineClient::<BlakeSubstrateChain>::from_url(&assethub_url).await?;
	let _assethub_rpc_client = RpcClient::from_url(&assethub_url).await?;
	let _assethub_rpc = LegacyRpcMethods::<BlakeSubstrateChain>::new(rpc_client.clone());
	println!("connecting to hyperbridge");
	let para_url = std::env::var("PARA_LOCAL_URL")
		.ok()
		.unwrap_or("ws://127.0.0.1:9990".to_string());
	let _para_client = OnlineClient::<Hyperbridge>::from_url(&para_url).await?;
	let para_rpc_client = RpcClient::from_url(&para_url).await?;
	let para_rpc = LegacyRpcMethods::<BlakeSubstrateChain>::new(para_rpc_client.clone());

	println!("opening hrmp channels");

	force_open_hrmp_channel(&relay_client, &signer, 1000, 2000).await?;
	force_open_hrmp_channel(&relay_client, &signer, 2000, 1000).await?;

	let sub = para_rpc.chain_subscribe_finalized_heads().await?;
	let _block = sub
		.take(1)
		.collect::<Vec<_>>()
		.await
		.into_iter()
		.collect::<Result<Vec<_>, _>>()?;

	let beneficiary_value = Value::named_composite(vec![
		("parents".to_string(), Value::u128(0_u8 as u128)),
		("interior".to_string(), Value::variant("X4", Composite::unnamed(vec![
			Value::unnamed_composite(vec![
				Value::variant("AccountId32", Composite::named(vec![
					("network".to_string(), Value::variant("None", Composite::unnamed(vec![]))),
					("id".to_string(), Value::from_bytes(pair.public().to_vec()))
				])),
				Value::variant("AccountKey20", Composite::named(vec![
					("network".to_string(), Value::variant("Some", Composite::unnamed(vec![
						Value::variant("Ethereum", Composite::named(vec![("chain_id".to_string(), Value::u128(1_u64 as u128))]))
					]))),
					("key".to_string(), Value::from_bytes(vec![1u8; 20]))
				])),
				Value::variant("GeneralIndex", Composite::unnamed(vec![Value::u128(3600)])),
				Value::variant("GeneralIndex", Composite::unnamed(vec![Value::u128(1)]))
			])
		])))
	]);

	let dot_location_value = Value::named_composite(vec![
		("parents".to_string(), Value::u128(1_u8 as u128)),
		("interior".to_string(), Value::variant("Here", Composite::unnamed(vec![])))
	]);

	let assets_value = Value::named_composite(vec![
		("id".to_string(), dot_location_value.clone()),
		("fun".to_string(), Value::variant("Fungible", Composite::unnamed(vec![Value::u128(SEND_AMOUNT)])))
	]);

	let weight_limit_value = Value::variant("Unlimited", Composite::unnamed(vec![]));

	let buy_execution_value = Value::variant("BuyExecution", Composite::named(vec![
		("fees".to_string(), assets_value.clone()),
		("weight_limit".to_string(), weight_limit_value.clone())
	]));

	let wild_all_value = Value::variant("Wild", Composite::unnamed(vec![
		Value::variant("All", Composite::unnamed(vec![]))
	]));

	let deposit_asset_value = Value::variant("DepositAsset", Composite::named(vec![
		("assets".to_string(), wild_all_value),
		("beneficiary".to_string(), beneficiary_value)
	]));

	let remote_xcm_value = Value::unnamed_composite(vec![
		buy_execution_value,
		deposit_asset_value
	]);

	let set_fees_mode_value = Value::variant("SetFeesMode", Composite::named(vec![
		("jit_withdraw".to_string(), Value::bool(true))
	]));

	let dest_location_value = Value::named_composite(vec![
		("parents".to_string(), Value::u128(1_u8 as u128)),
		("interior".to_string(), Value::variant("X1", Composite::unnamed(vec![
			Value::unnamed_composite(vec![
				Value::variant("Parachain", Composite::unnamed(vec![Value::u128(2000_u32 as u128)]))
			])
		])))
	]);

	let transfer_reserve_asset_value = Value::variant("TransferReserveAsset", Composite::named(vec![
		("assets".to_string(), Value::unnamed_composite(vec![assets_value.clone()])),
		("dest".to_string(), dest_location_value),
		("xcm".to_string(), remote_xcm_value)
	]));

	let message_xcm_value = Value::unnamed_composite(vec![
		set_fees_mode_value,
		transfer_reserve_asset_value
	]);

	let xcm_struct_value = Value::unnamed_composite(vec![message_xcm_value]);

	let message_value = Value::variant("V5", Composite::unnamed(vec![xcm_struct_value]));

	let max_weight_value = Value::named_composite(vec![
		("ref_time".to_string(), Value::u128(400_000_000_000u128)),
		("proof_size".to_string(), Value::u128(1_000_000u128))
	]);

	let ext = subxt::dynamic::tx(
		"PolkadotXcm",
		"execute",
		vec![
			message_value,
			max_weight_value,
		],
	);

	let metadata = assethub_client.metadata();
	let call_data_bytes = ext.encode_call_data(&metadata)?;

	println!(
		"\n\n>>> Hex-encoded call for Polkadot-JS:\n0x{}\n\n",
		sp_core::hexdisplay::HexDisplay::from(&call_data_bytes)
	);


	let init_block = para_rpc
		.chain_get_header(None)
		.await?
		.ok_or_else(|| anyhow!("Failed to fetch latest header"))?
		.number();

	send_extrinsic(&assethub_client, &signer, &ext, None).await?;

	println!("done performing limited reserve asset transfer");
	let mut sub = para_rpc.chain_subscribe_finalized_heads().await?;

	while let Some(res) = sub.next().await {
		match res {
			Ok(header) => {
				if header.number().saturating_sub(init_block) >= 500 {
					Err(anyhow!("XCM Integration test failed: Post request event was not found"))?
				}

				let params = rpc_params![
                BlockNumberOrHash::<H256>::Number(init_block),
                BlockNumberOrHash::<H256>::Number(header.number())
             ];

				let response: HashMap<String, Vec<ismp::events::Event>> =
					para_rpc_client.request("ismp_queryEvents", params).await?;

				let events = response.values().into_iter().cloned().flatten().collect::<Vec<_>>();
				if let Some(post) = events.into_iter().find_map(|ev| match ev {
					ismp::events::Event::PostRequest(post) => Some(post),
					_ => None,
				}) {
					let body =
						pallet_token_gateway::types::Body::abi_decode(&mut &post.body[1..], true)
							.unwrap();
					let to = alloy_primitives::FixedBytes::<32>::from_slice(
						&vec![vec![0u8; 12], vec![1u8; 20]].concat(),
					);
					assert_eq!(body.to, to);
					assert_eq!(post.dest, StateMachine::Evm(1));
					assert_eq!(post.source, StateMachine::Kusama(2000));
					return Ok(());
				}
			},

			Err(err) => {
				println!("{err:?}")
			},
		}
	}

	Err(anyhow!("XCM Integration test failed"))
}

async fn force_open_hrmp_channel(
	relay_client: &OnlineClient<BlakeSubstrateChain>,
	signer: &InMemorySigner<BlakeSubstrateChain>,
	sender: u128,
	recipient: u128,
) -> anyhow::Result<()> {
	let force_call = subxt::dynamic::tx(
		"Hrmp",
		"force_open_hrmp_channel",
		vec![Value::u128(sender), Value::u128(recipient), Value::u128(8), Value::u128(1024 * 1024)],
	);
	let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![force_call.into_value()]);
	send_extrinsic(relay_client, signer, &sudo_call, None).await?;

	Ok(())
}




