#![cfg(test)]

use std::{collections::HashMap, sync::Arc};

use alloy_sol_types::SolType;
use anyhow::anyhow;
use futures::StreamExt;
use polkadot_sdk::{
	cumulus_primitives_core::{AllCounted, AssetFilter, DepositAsset, Wild, Xcm},
	sp_core::{bytes::from_hex, sr25519, Pair, H256},
	sp_runtime::Weight,
	staging_xcm::{
		v5::{Asset, AssetId, Fungibility, Parent},
		VersionedAssets, VersionedXcm,
	},
	staging_xcm_executor::traits::TransferType,
	*,
};
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
	let beneficiary: Location = Junctions::X4(Arc::new([
		Junction::AccountId32 { network: None, id: pair.public().into() },
		Junction::AccountKey20 {
			network: Some(NetworkId::Ethereum { chain_id: 1 }),
			key: [1u8; 20],
		},
		Junction::GeneralIndex(60 * 60),
		Junction::GeneralIndex(1),
	]))
	.into_location();
	let weight_limit = WeightLimit::Unlimited;

	let dest: VersionedLocation =
		VersionedLocation::V5(Location::new(1, [Junction::Parachain(2000)]));

	let dot_location: Location = Parent.into();
	let fee_asset_id = AssetId(dot_location.clone());

	let asset = Asset { id: dot_location.clone().into(), fun: Fungibility::Fungible(SEND_AMOUNT) };
	let assets = VersionedAssets::V5(vec![asset].into());

	let xcm_beneficiary = beneficiary.clone();
	let custom_xcm_on_dest = VersionedXcm::V5(Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(1)),
		beneficiary: xcm_beneficiary,
	}]));

	println!("performing limited reserve asset transfer");
	let dest_value = versioned_location_to_value(&dest);
	let assets_value = versioned_assets_to_value(&assets);
	let assets_transfer_type_value = Value::variant("LocalReserve", Composite::unnamed(vec![]));
	let fee_transfer_type_value = Value::variant("LocalReserve", Composite::unnamed(vec![]));
	let fee_asset_id_value = versioned_asset_id_to_value(&fee_asset_id);
	let custom_xcm_on_dest_value = versioned_xcm_to_value(&custom_xcm_on_dest);
	let weight_limit_value = weight_limit_to_value(&weight_limit);
	let ext = subxt::dynamic::tx(
		"PolkadotXcm",
		"transfer_assets_using_type_and_then",
		vec![
			dest_value,
			assets_value,
			assets_transfer_type_value,
			fee_asset_id_value,
			fee_transfer_type_value,
			custom_xcm_on_dest_value,
			weight_limit_value,
		],
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
	Value::variant("V5", Composite::unnamed(vec![location_to_value(&id.0)]))
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
		Wild(AllCounted(count)) => Value::variant(
			"Wild",
			Composite::unnamed(vec![Value::variant(
				"AllCounted",
				Composite::unnamed(vec![Value::u128(*count as u128)]),
			)]),
		),
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
