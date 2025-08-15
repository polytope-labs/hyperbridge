#![cfg(test)]
use codec::Decode;
use gargantua_runtime::AuraId;
use sp_core::{H256, Pair, Public, bytes::from_hex, crypto::AccountId32, sr25519};
use std::str::FromStr;
use subxt::{
	OnlineClient,
	dynamic::Value,
	ext::subxt_rpcs::{RpcClient, rpc_params},
	tx::Signer,
};
use subxt_utils::{Hyperbridge, InMemorySigner as PairSigner};

type Api = OnlineClient<Hyperbridge>;

pub fn setup_logging() {
	use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};
	let filter =
		tracing_subscriber::EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());
	tracing_subscriber::fmt().with_env_filter(filter).finish().try_init().unwrap();
}

async fn insert_session_keys(rpc: &RpcClient, suri: &str) -> Result<(), anyhow::Error> {
	let key_type = "aura";
	let public_key = sr25519::Pair::from_string(suri, None).unwrap().public();
	let public_key_hex = format!("0x{}", hex::encode(public_key));
	let params = rpc_params![key_type, suri.to_string(), public_key_hex];
	rpc.request::<()>("author_insertKey", params).await?;
	Ok(())
}

#[ignore]
#[tokio::test]
async fn collator_manager_integration_test() -> Result<(), Box<dyn std::error::Error>> {
	setup_logging();
	dotenv::dotenv().ok();
	let private_key = std::env::var("SUBSTRATE_SIGNING_KEY").ok().unwrap_or(
		"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
	);
	let seed = from_hex(&private_key)?;
	let pair = sr25519::Pair::from_seed_slice(&seed)?;
	let signer = PairSigner::<Hyperbridge>::new(pair.clone());

	println!("collator manager integration test");
	let url1 = "ws://127.0.0.1:9990".to_string();
	let url2 = "ws://127.0.0.1:9991".to_string();
	let url3 = "ws://127.0.0.1:9992".to_string();
	let url4 = "ws://127.0.0.1:9993".to_string();
	println!("Connecting to parachain collator 1 at {}...", url1);
	println!("Connecting to parachain collator 2 at {}...", url2);

	let (api1, rpc1) = subxt_utils::client::ws_client::<Hyperbridge>(&url1, u32::MAX).await?;
	let (api2, rpc2) = subxt_utils::client::ws_client::<Hyperbridge>(&url2, u32::MAX).await?;
	let (_api3, rpc3) = subxt_utils::client::ws_client::<Hyperbridge>(&url3, u32::MAX).await?;
	let (_api4, rpc4) = subxt_utils::client::ws_client::<Hyperbridge>(&url4, u32::MAX).await?;

	let _alice_pair = pair;
	let bob_pair = sr25519::Pair::from_string("//Bob", None).unwrap();
	let charlie_pair = sr25519::Pair::from_string("//Charlie", None).unwrap();
	let dave_pair = sr25519::Pair::from_string("//Dave", None).unwrap();

	let alice = signer;
	let bob = PairSigner::<Hyperbridge>::new(bob_pair.clone());
	let charlie = PairSigner::<Hyperbridge>::new(charlie_pair.clone());
	let dave = PairSigner::<Hyperbridge>::new(dave_pair.clone());

	println!("Setup complete. Connected to nodes and created signers.");

	insert_session_keys(&rpc1, "//Alice").await?;
	insert_session_keys(&rpc2, "//Bob").await?;
	insert_session_keys(&rpc3, "//Charlie").await?;
	insert_session_keys(&rpc4, "//Dave").await?;
	println!("Session keys injected into Charlie, Dave collator nodes.");

	let validators_addr = subxt::dynamic::storage("Session", "Validators", vec![]);
	let initial_validators = api1.storage().at_latest().await?.fetch(&validators_addr).await?;
	let mut initial_validators: Vec<AccountId32> =
		Decode::decode(&mut &initial_validators.unwrap().encoded()[..])?;
	initial_validators.sort();
	println!("initial validators are {:?}", initial_validators);

	let alice_addr = alice.account_id().clone();
	let charlie_addr = charlie.account_id().clone();
	let dave_addr = dave.account_id().clone();

	let reputation_asset_id: H256 = H256([
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 1,
	]);

	let create_asset_call = subxt::dynamic::tx(
		"Assets",
		"force_create",
		vec![
			Value::from_bytes(reputation_asset_id),
			Value::unnamed_variant("Id", [Value::from_bytes(&alice_addr)]),
			Value::bool(true),
			Value::u128(1),
		],
	);
	let sudo_create_asset =
		subxt::dynamic::tx("Sudo", "sudo", vec![create_asset_call.into_value()]);

	api1.tx()
		.sign_and_submit_then_watch_default(&sudo_create_asset, &alice)
		.await?
		.wait_for_finalized_success()
		.await?;
	println!("Reputation asset created.");

	set_keys(&api1, "Alice", &alice).await?;
	set_keys(&api1, "Bob", &bob).await?;
	set_keys(&api1, "Charlie", &charlie).await?;
	set_keys(&api1, "Dave", &dave).await?;

	register(&api1, &alice, &charlie, 10_000, reputation_asset_id).await?;
	register(&api1, &alice, &dave, 20_000, reputation_asset_id).await?;
	println!("New candidates (Charlie, Dave) have registered and set keys.");

	let session_index_addr = subxt::dynamic::storage("Session", "CurrentIndex", vec![]);
	let initial_session_value = api1
		.storage()
		.at_latest()
		.await?
		.fetch(&session_index_addr)
		.await?
		.unwrap()
		.to_value()?;
	let initial_session: u128 = initial_session_value.as_u128().unwrap();
	println!("Waiting for session to change from {}...", initial_session);

	let mut session_changed = false;
	while !session_changed {
		let current_session_value = api1
			.storage()
			.at_latest()
			.await?
			.fetch(&session_index_addr)
			.await?
			.unwrap()
			.to_value()?;
		let current_session: u128 = current_session_value.as_u128().unwrap();
		if current_session > initial_session {
			println!("session changed to {}.", current_session);
			session_changed = true;
		}
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;
	}

	let second_session_value = api1
		.storage()
		.at_latest()
		.await?
		.fetch(&session_index_addr)
		.await?
		.unwrap()
		.to_value()?;
	let second_session: u128 = second_session_value.as_u128().unwrap();
	session_changed = false;
	while !session_changed {
		let current_session_value = api1
			.storage()
			.at_latest()
			.await?
			.fetch(&session_index_addr)
			.await?
			.unwrap()
			.to_value()?;
		let current_session: u128 = current_session_value.as_u128().unwrap();
		if current_session > second_session {
			println!("Session changed to {}.", current_session);
			session_changed = true;
		}
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;
	}

	println!("Verifying the new collator set on both nodes...");
	let validators_node1_val = api1.storage().at_latest().await?.fetch(&validators_addr).await?;
	let validators_node1_val: Vec<AccountId32> =
		Decode::decode(&mut &validators_node1_val.unwrap().encoded()[..])?;
	println!("{:?}", validators_node1_val);

	let validators_node2_val = api2.storage().at_latest().await?.fetch(&validators_addr).await?;
	let validators_node2_val: Vec<AccountId32> =
		Decode::decode(&mut &validators_node2_val.unwrap().encoded()[..])?;
	println!("{:?}", validators_node2_val);

	let mut collators_node1 = validators_node1_val.clone();
	collators_node1.sort();

	let mut collators_node2 = validators_node2_val.clone();
	collators_node2.sort();

	let mut expected_collators = vec![
		AccountId32::from_str(&charlie_addr.to_string())?,
		AccountId32::from_str(&dave_addr.to_string())?,
	];
	expected_collators.sort();

	assert_ne!(initial_validators, collators_node1);
	assert_eq!(collators_node1, expected_collators, "Node 1 has an incorrect collator set!");
	assert_eq!(collators_node2, expected_collators, "Node 2 has an incorrect collator set!");

	println!("Waiting for new collators to produce blocks...");
	let block_number_addr = subxt::dynamic::storage("System", "Number", vec![]);
	let last_known_block_val =
		api1.storage().at_latest().await?.fetch(&block_number_addr).await?.unwrap();
	let last_known_block: u32 = Decode::decode(&mut &last_known_block_val.encoded()[..])?;
	let target_block = last_known_block + 5;

	let mut block_produced = false;
	while !block_produced {
		let current_block_val =
			api1.storage().at_latest().await?.fetch(&block_number_addr).await?.unwrap();
		let current_block: u32 = Decode::decode(&mut &current_block_val.encoded()[..])?;
		if current_block >= target_block {
			println!("New blocks produced. Current block: {}.", current_block);
			block_produced = true;
		}
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;
	}

	println!("Test passed!");

	Ok(())
}

async fn set_keys(
	api: &Api,
	seed: &str,
	signer: &PairSigner<Hyperbridge>,
) -> Result<(), Box<dyn std::error::Error>> {
	let key = get_collator_keys_from_seed(seed);
	let aura_key = Value::from_bytes(key);
	let keys_value = Value::named_composite([("aura", aura_key)]);
	let set_keys_call = subxt::dynamic::tx(
		"Session",
		"set_keys",
		vec![keys_value, Value::from_bytes(vec![0u8; 0])],
	);
	api.tx()
		.sign_and_submit_then_watch_default(&set_keys_call, signer)
		.await?
		.wait_for_finalized_success()
		.await?;
	Ok(())
}

async fn register(
	api: &Api,
	sudo: &PairSigner<Hyperbridge>,
	candidate: &PairSigner<Hyperbridge>,
	bond: u128,
	asset_id: H256,
) -> Result<(), Box<dyn std::error::Error>> {
	let mint_call = subxt::dynamic::tx(
		"Assets",
		"mint",
		vec![
			Value::from_bytes(asset_id),
			Value::unnamed_variant("Id", [Value::from_bytes(&candidate.account_id())]),
			Value::u128(bond),
		],
	);
	api.tx()
		.sign_and_submit_then_watch_default(&mint_call, sudo)
		.await?
		.wait_for_finalized_success()
		.await?;

	let register_call =
		subxt::dynamic::tx("CollatorSelection", "register_as_candidate", Vec::<Value>::new());
	api.tx()
		.sign_and_submit_then_watch_default(&register_call, candidate)
		.await?
		.wait_for_finalized_success()
		.await?;
	Ok(())
}

pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
	get_from_seed::<AuraId>(seed)
}
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}
