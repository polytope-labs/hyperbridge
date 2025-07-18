#![cfg(test)]

use polkadot_sdk::*;
use std::{collections::HashMap, sync::Arc};
use std::ops::Deref;

use alloy_sol_types::SolType;
use anyhow::anyhow;
use codec::Encode;
use futures::StreamExt;
use ismp::host::StateMachine;
use pallet_ismp_rpc::BlockNumberOrHash;
use staging_xcm::{
	v4::{Junction, Junctions, Location, NetworkId, WeightLimit},
	VersionedAssets, VersionedLocation,
};
use subxt::{
	config::Header,
	ext::subxt_rpcs::rpc_params,
	tx::Payload,
	OnlineClient,
	backend::legacy::LegacyRpcMethods,
	ext::subxt_rpcs::RpcClient,
};
use polkadot_sdk::sp_core::{bytes::from_hex, sr25519, Pair, H256};
use polkadot_sdk::sp_runtime::Weight;
use polkadot_sdk::staging_xcm::v4::{Asset, AssetId, Assets, Fungibility};
use subxt::dynamic::Value;
use subxt::ext::scale_value::Composite;
use subxt_utils::{send_extrinsic, BlakeSubstrateChain, Hyperbridge, InMemorySigner};

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
	let signer = InMemorySigner::<BlakeSubstrateChain>::new(pair.clone());
	let url = std::env::var("ROCOCO_LOCAL_URL")
		.ok()
		.unwrap_or("ws://127.0.0.1:9922".to_string());
	let client = OnlineClient::<BlakeSubstrateChain>::from_url(&url).await?;
	let rpc_client = RpcClient::from_url(&url).await?;
	let rpc = LegacyRpcMethods::<BlakeSubstrateChain>::new(rpc_client.clone());

	let para_url = std::env::var("PARA_LOCAL_URL")
		.ok()
		.unwrap_or("ws://127.0.0.1:9990".to_string());
	let para_client = OnlineClient::<Hyperbridge>::from_url(&para_url).await?;
	let para_rpc_client = RpcClient::from_url(&para_url).await?;
	let para_rpc = LegacyRpcMethods::<BlakeSubstrateChain>::new(para_rpc_client.clone());

	// Wait for parachain block production

	let sub = para_rpc.chain_subscribe_finalized_heads().await?;
	let _block = sub
		.take(1)
		.collect::<Vec<_>>()
		.await
		.into_iter()
		.collect::<Result<Vec<_>, _>>()?;
	let beneficiary: Location = Junctions::X3(Arc::new([
		Junction::AccountId32 { network: None, id: pair.public().into() },
		Junction::AccountKey20 {
			network: Some(NetworkId::Ethereum { chain_id: 1 }),
			key: [1u8; 20],
		},
		Junction::GeneralIndex(60 * 60),
	]))
	.into_location();
	let weight_limit = WeightLimit::Unlimited;

	let dest: VersionedLocation = VersionedLocation::V4(Junction::Parachain(2000).into());
	let para_absolute_location: Location = Junction::Parachain(2000).into();

	let call = (
		Box::<VersionedLocation>::new(dest.clone()),
		Box::<VersionedLocation>::new(VersionedLocation::V4(beneficiary.clone().into())),
		Box::<VersionedAssets>::new(VersionedAssets::V4(
			vec![(Junctions::Here, SEND_AMOUNT).into()].into(),
		)),
		0,
		weight_limit,
	);

	let dest_location = Location::new(1, [Junction::Parachain(2000)]);
	let dest = VersionedLocation::V4(dest_location);


	let beneficiary_as_versioned = VersionedLocation::V4(beneficiary);

	let assets = Assets::from(vec![(Junctions::Here, SEND_AMOUNT).into()]);
	let assets_as_versioned = VersionedAssets::V4(assets);

	let assets_vec = vec![(Junctions::Here, SEND_AMOUNT).into()];

	let weight_limit = WeightLimit::Unlimited;

	let dest_value = versioned_location_to_value(&dest);
	let beneficiary_value = versioned_location_to_value(&beneficiary_as_versioned);
	let assets_value = versioned_assets_to_value(&assets_vec);
	let fee_asset_index_value = Value::u128(0);
	let weight_limit_value = weight_limit_to_value(&weight_limit);

	println!("making xcm call");
	{
		let signer = InMemorySigner::<BlakeSubstrateChain>::new(pair.clone());
		let para_absolute_location: Location = Junction::Parachain(2000).into();
		let version = 4u32;

		let location_as_value = location_to_value(&para_absolute_location);
		let version_as_value = Value::u128(version.into());

		// Force set the xcm version to our supported version
		let call = subxt::dynamic::tx("XcmPallet", "force_xcm_version", vec![location_as_value, version_as_value]);
		let tx = subxt::dynamic::tx("Sudo", "sudo", vec![call.into_value()]);
		send_extrinsic(&client, &signer, &tx, None).await?;
	}
	println!("finished making xcm call");

	let ext = subxt::dynamic::tx(
		"XcmPallet",
		"limited_reserve_transfer_assets",
		vec![
			dest_value,
			beneficiary_value,
			assets_value,
			fee_asset_index_value,
			weight_limit_value,
		],
	);

	let init_block = para_rpc
		.chain_get_header(None)
		.await?
		.ok_or_else(|| anyhow!("Failed to fetch latest header"))?
		.number();

	send_extrinsic(&client, &signer, &ext, None).await?;
	println!("finished making limited_reserve_transfer_assets call");

	let mut sub = para_rpc.chain_subscribe_finalized_heads().await?;

	while let Some(res) = sub.next().await {
		match res {
			Ok(header) => {
				// Break if we've waited too long
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
					// Assert that this is the post we sent
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

fn junction_to_value(junction: &Junction) -> Value<()> {
	match junction {
		Junction::Parachain(id) => {
			Value::variant("Parachain", Composite::unnamed(vec![Value::u128((*id).into())]))
		}
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
		}
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
		}
		Junction::GeneralIndex(index) => {
			Value::variant("GeneralIndex", Composite::unnamed(vec![Value::u128(*index)]))
		}
		_ => unimplemented!("This helper only supports a subset of junctions for now"),
	}
}


/// Converts the complex `Junctions` enum into its `Value` representation.
fn junctions_to_value(junctions: &Junctions) -> Value<()> {
	match junctions {
		Junctions::Here => Value::variant("Here", Composite::unnamed(vec![])),
		// The X1..X8 variants are all tuple-structs containing an array of Junctions.
		// We handle them by getting the name of the variant (e.g., "X1") and
		// converting the inner array of junctions into a Value.
		_ => {
			let junctions_slice = junctions.as_slice();
			let variant_name = format!("X{}", junctions_slice.len());
			let junction_values: Vec<Value<()>> =
				junctions_slice.iter().map(junction_to_value).collect();

			// The variant (e.g., X1) contains a tuple, which contains the array of junctions.
			// So we must create a Value::unnamed_composite to represent that inner array.
			let inner_array_value = Value::unnamed_composite(junction_values);

			Value::variant(variant_name, Composite::unnamed(vec![inner_array_value]))
		}
	}
}

/// Precisely builds the `Value` for a `Location` struct by correctly
/// representing its nested structure.
fn location_to_value(location: &Location) -> Value<()> {
	Value::named_composite(vec![
		(
			"parents".to_string(),
			Value::u128(location.parents.into()),
		),
		(
			"interior".to_string(),
			junctions_to_value(&location.interior),
		),
	])
}

fn network_id_to_value(network_id: &NetworkId) -> Value<()> {
	match network_id {
		NetworkId::Ethereum { chain_id } => {
			let composite =
				Composite::named(vec![("chain_id".to_string(), Value::u128((*chain_id).into()))]);
			Value::variant("Ethereum", composite)
		}
		_ => unimplemented!("This helper only supports Ethereum NetworkId for now"),
	}
}



pub fn force_xcm_version_value() -> Value<()> {
	let para_absolute_location: Location = Junction::Parachain(2000).into();
	let some_u32 = 4u32;

	let location_as_value = location_to_value(&para_absolute_location);

	let u32_as_value = Value::u128(some_u32.into());

	let extrinsic_params = Value::unnamed_composite(vec![
		location_as_value,
		u32_as_value,
	]);

	extrinsic_params
}

fn versioned_location_to_value(loc: &VersionedLocation) -> Value<()> {
	match loc {
		VersionedLocation::V4(location) => {
			let location_value = location_to_value(location);
			Value::variant("V4", Composite::unnamed(vec![location_value]))
		}
		_ => unimplemented!("This helper only supports V4 VersionedLocation"),
	}
}

fn fungibility_to_value(fun: &Fungibility) -> Value<()> {
	match fun {
		Fungibility::Fungible(amount) => {
			Value::variant("Fungible", Composite::unnamed(vec![Value::u128(*amount)]))
		}
		_ => unimplemented!("This helper only supports Fungible variant"),
	}
}

fn asset_id_to_value(id: &AssetId) -> Value<()> {
	location_to_value(&id.0)
}

fn asset_to_value(asset: &Asset) -> Value<()> {
	Value::named_composite(vec![
		("id".to_string(), asset_id_to_value(&asset.id)),
		("fun".to_string(), fungibility_to_value(&asset.fun)),
	])
}

fn versioned_assets_to_value(assets_vec: &Vec<Asset>) -> Value<()> {
	let asset_values: Vec<Value<()>> = assets_vec.iter().map(asset_to_value).collect();
	let inner_assets_value = Value::unnamed_composite(asset_values);

	let assets_struct_value = Value::unnamed_composite(vec![inner_assets_value]);
	Value::variant("V4", Composite::unnamed(vec![assets_struct_value]))
}


fn weight_to_value(weight: &Weight) -> Value<()> {
	Value::named_composite(vec![
		("ref_time".to_string(), Value::u128(weight.ref_time().into())),
		("proof_size".to_string(), Value::u128(weight.proof_size().into())),
	])
}

fn weight_limit_to_value(limit: &WeightLimit) -> Value<()> {
	match limit {
		WeightLimit::Unlimited => Value::variant("Unlimited", Composite::unnamed(vec![])),
		WeightLimit::Limited(weight) => {
			let weight_value = weight_to_value(weight);
			Value::variant("Limited", Composite::unnamed(vec![weight_value]))
		}
	}
}




pub fn create_full_xcm_transfer_value() -> Value<()> {
	const FEE_ASSET_INDEX: u32 = 0;

	let dest_location = Location::new(1, [Junction::Parachain(2000)]);
	let dest = VersionedLocation::V4(dest_location);

	let public_key: [u8; 32] = [42; 32];
	let beneficiary = Location::new(
		0,
		[
			Junction::AccountId32 { network: None, id: public_key },
			Junction::AccountKey20 {
				network: Some(NetworkId::Ethereum { chain_id: 1 }),
				key: [1; 20],
			},
			Junction::GeneralIndex(60 * 60),
		],
	);
	let beneficiary_as_versioned = VersionedLocation::V4(beneficiary);


	/*let assets_as_versioned = VersionedAssets::V4(
		vec![(Junctions::Here, SEND_AMOUNT).into()].into(),
	);*/

	let assets_vec = vec![Asset {
		id: AssetId(Location::here()),
		fun: Fungibility::Fungible(SEND_AMOUNT),
	}];

	let weight_limit = WeightLimit::Unlimited;

	let extrinsic_params = Value::unnamed_composite(vec![
		versioned_location_to_value(&dest),
		versioned_location_to_value(&beneficiary_as_versioned),
		versioned_assets_to_value(&assets_vec),
		Value::u128(FEE_ASSET_INDEX.into()),
		weight_limit_to_value(&weight_limit),
	]);

	extrinsic_params
}
