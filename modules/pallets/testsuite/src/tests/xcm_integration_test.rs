#![cfg(test)]

use std::{collections::HashMap, sync::Arc};

use alloy_sol_types::SolType;
use anyhow::anyhow;
use futures::StreamExt;
use polkadot_sdk::{
	sp_core::{bytes::from_hex, sr25519, Pair, H256},
	sp_runtime::Weight,
	staging_xcm::v5::{Asset, AssetId, Fungibility, Parent},
	*,
};
use polkadot_sdk::cumulus_primitives_core::{All, AllCounted, AssetFilter, BuyExecution, DepositAsset, Parachain, SetFeesMode, TransferReserveAsset, Wild, Xcm};
use polkadot_sdk::staging_xcm::{VersionedAssets, VersionedXcm};
use polkadot_sdk::staging_xcm_executor::traits::TransferType;
use staging_xcm::{
	v5::{Junction, Junctions, Location, NetworkId, WeightLimit},
	VersionedLocation,
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

fn junction_to_value(junction: &Junction) -> Value {
	match junction {
		Junction::Parachain(id) =>
			Value::variant("Parachain", Composite::unnamed(vec![Value::u128((*id).into())])),
		Junction::AccountId32 { network, id } => {
			let network_value = match network {
				Some(n) => Value::variant("Some", Composite::unnamed(vec![network_id_to_value(n)])),
				None => Value::variant("None", Composite::unnamed(vec![])),
			};
			let composite = Composite::named(vec![
				("network".to_string(), network_value),
				("id".to_string(), Value::from_bytes(id.to_vec())),
			]);
			Value::variant("AccountId32", composite)
		},
		Junction::AccountKey20 { network, key } => {
			let network_value = match network {
				Some(n) => Value::variant("Some", Composite::unnamed(vec![network_id_to_value(n)])),
				None => Value::variant("None", Composite::unnamed(vec![])),
			};
			let composite = Composite::named(vec![
				("network".to_string(), network_value),
				("key".to_string(), Value::from_bytes(key.to_vec())),
			]);
			Value::variant("AccountKey20", composite)
		},
		Junction::GeneralIndex(index) =>
			Value::variant("GeneralIndex", Composite::unnamed(vec![Value::u128(*index)])),
		_ => unimplemented!("This helper only supports a subset of junctions for now"),
	}
}

fn junctions_to_value(junctions: &Junctions) -> Value<()> {
	match junctions {
		Junctions::Here => Value::variant("Here", Composite::unnamed(vec![])),
		_ => {
			let junctions_slice = junctions.as_slice();
			let variant_name = format!("X{}", junctions_slice.len());
			let junction_values: Vec<Value<()>> =
				junctions_slice.iter().map(junction_to_value).collect();

			let inner_array_value = Value::unnamed_composite(junction_values);

			Value::variant(variant_name, Composite::unnamed(vec![inner_array_value]))
		},
	}
}

fn location_to_value(location: &Location) -> Value<()> {
	Value::named_composite(vec![
		("parents".to_string(), Value::u128(location.parents.into())),
		("interior".to_string(), junctions_to_value(&location.interior)),
	])
}

fn network_id_to_value(network_id: &NetworkId) -> Value<()> {
	match network_id {
		NetworkId::Ethereum { chain_id } => {
			let composite =
				Composite::named(vec![("chain_id".to_string(), Value::u128((*chain_id).into()))]);
			Value::variant("Ethereum", composite)
		},
		_ => unimplemented!("This helper only supports Ethereum NetworkId for now"),
	}
}

#[allow(dead_code)]
pub fn force_xcm_version_value() -> Value<()> {
	let para_absolute_location: Location = Junction::Parachain(2000).into();
	let some_u32 = 4u32;

	let location_as_value = location_to_value(&para_absolute_location);

	let u32_as_value = Value::u128(some_u32.into());

	let extrinsic_params = Value::unnamed_composite(vec![location_as_value, u32_as_value]);

	extrinsic_params
}

fn versioned_location_to_value(loc: &VersionedLocation) -> Value<()> {
	match loc {
		VersionedLocation::V5(location) => {
			let location_value = location_to_value(location);
			Value::variant("V5", Composite::unnamed(vec![location_value]))
		},
		_ => unimplemented!("This helper only supports V5 VersionedLocation"),
	}
}

fn fungibility_to_value(fun: &Fungibility) -> Value {
	match fun {
		Fungibility::Fungible(amount) =>
			Value::variant("Fungible", Composite::unnamed(vec![Value::u128(*amount)])),
		_ => unimplemented!("This helper only supports Fungible variant"),
	}
}

fn asset_id_to_value(id: &AssetId) -> Value {
	location_to_value(&id.0)
}

fn asset_to_value(asset: &Asset) -> Value {
	Value::named_composite(vec![
		("id".to_string(), asset_id_to_value(&asset.id)),
		("fun".to_string(), fungibility_to_value(&asset.fun)),
	])
}

fn transfer_type_to_value(transfer_type: &TransferType) -> Value {
	match transfer_type {
		TransferType::DestinationReserve =>
			Value::variant("DestinationReserve", Composite::unnamed(vec![])),
		_ => unimplemented!("This helper only supports DestinationReserve"),
	}
}

fn versioned_asset_id_to_value(id: &AssetId) -> Value {
	Value::variant("V5", Composite::unnamed(vec![
		location_to_value(&id.0)
	]))
}

fn versioned_assets_to_value(assets: &VersionedAssets) -> Value {
	match assets {
		VersionedAssets::V5(assets_vec) => {
			let asset_values: Vec<Value> = vec![asset_to_value(assets_vec.get(0).unwrap())];
			let vec_multi_asset_value = Value::unnamed_composite(asset_values);

			Value::variant("V5", Composite::unnamed(vec![vec_multi_asset_value]))
		},
		_ => unimplemented!("This helper only supports V5 VersionedAssets"),
	}
}

fn weight_to_value(weight: &Weight) -> Value {
	Value::named_composite(vec![
		("ref_time".to_string(), Value::u128(weight.ref_time().into())),
		("proof_size".to_string(), Value::u128(weight.proof_size().into())),
	])
}

fn versioned_xcm_to_value(xcm: &VersionedXcm<()>) -> Value {
	match xcm {
		VersionedXcm::V5(instruction) => {
			let instruction_value = xcm_instruction_to_value(instruction);
			Value::variant("V5", Composite::unnamed(vec![instruction_value]))
		},
		_ => unimplemented!("This helper only supports V5 VersionedXcm"),
	}
}

fn wild_to_value(wild: &AssetFilter) -> Value {
	match wild {
		Wild(AllCounted(count)) =>
			Value::variant("Wild", Composite::unnamed(vec![
				Value::variant("AllCounted", Composite::unnamed(vec![Value::u128(*count as u128)]))
			])),
		_ => unimplemented!("This helper only supports Wild(AllCounted)"),
	}
}
fn xcm_instruction_to_value(instruction: &Xcm<()>) -> Value {
	let instructions: Vec<Value> = instruction
		.0
		.iter()
		.map(|inst| match inst {
			DepositAsset { assets, beneficiary } => {
				let assets_value = wild_to_value(assets);
				let beneficiary_value = location_to_value(beneficiary);
				Value::variant(
					"DepositAsset",
					Composite::named(vec![
						("assets".to_string(), assets_value),
						("beneficiary".to_string(), beneficiary_value),
					]),
				)
			},
			_ => unimplemented!("This helper only supports DepositAsset instruction"),
		})
		.collect();

	Value::unnamed_composite(instructions)
}

fn weight_limit_to_value(limit: &WeightLimit) -> Value {
	match limit {
		WeightLimit::Unlimited => Value::variant("Unlimited", Composite::unnamed(vec![])),
		WeightLimit::Limited(weight) => {
			let weight_value = weight_to_value(weight);
			Value::variant("Limited", Composite::unnamed(vec![weight_value]))
		},
	}
}



