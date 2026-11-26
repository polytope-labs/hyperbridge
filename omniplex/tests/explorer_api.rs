//! The explorer API over HTTP, as the Statescan site calls it, on a database of its own. Set
//! `INDEXER_TEST_DATABASE_URL` to run.

use std::time::Duration;

use alloy::primitives::{Address, Bytes, FixedBytes, B256};
use codec::Encode;
use omniplex::{
	handlers::{
		abi::pallets::{Balances, Explorer},
		explorer::{write_account, write_block, write_event, write_extrinsic, write_transfer},
	},
	registry::LogMeta,
	substrate::encode_address,
};
use serde_json::{json, Value};
use tokio_postgres::NoTls;

const HEIGHT: u64 = 9_709_558;
const ALICE: [u8; 32] = [7; 32];
const BOB: [u8; 32] = [9; 32];

fn log(transaction_index: u64, log_index: u64) -> LogMeta {
	LogMeta {
		address: Address::ZERO,
		block_number: HEIGHT,
		block_hash: B256::repeat_byte(0xbb),
		block_timestamp: 1_784_624_058,
		transaction_hash: B256::repeat_byte(0xee),
		transaction_index,
		log_index,
	}
}

/** A fresh `explorer_api_test` database with one block in it: a timestamp inherent, then Alice's
 * signed transfer to Bob with its two events. */
async fn seed(admin_url: &str) -> String {
	let name = "explorer_api_test";
	let (admin, connection) = tokio_postgres::connect(admin_url, NoTls).await.unwrap();
	tokio::spawn(connection);
	admin.batch_execute(&format!("DROP DATABASE IF EXISTS {name}")).await.unwrap();
	admin.batch_execute(&format!("CREATE DATABASE {name}")).await.unwrap();
	let url = format!("{}/{name}", admin_url.rsplit_once('/').unwrap().0);

	let (mut client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	omniplex::db::apply_migrations(&mut client).await.unwrap();
	let db = client.transaction().await.unwrap();

	let block = Explorer::Block {
		specVersion: 7_900,
		parentHash: B256::repeat_byte(1),
		stateRoot: B256::repeat_byte(2),
		extrinsicsRoot: B256::repeat_byte(3),
		extrinsicsCount: 2,
		eventsCount: 3,
	};
	write_block(&db, &log(2, 3), &block).await.unwrap();
	let inherent = Explorer::Extrinsic {
		index: 0,
		signed: false,
		success: true,
		signer: Bytes::new(),
		section: "timestamp".into(),
		method: "set".into(),
		args: r#"[{"name":"now","type":"Moment","value":1784624058000}]"#.into(),
		eventsCount: 1,
	};
	write_extrinsic(&db, &log(0, 4), &inherent).await.unwrap();
	let transfer_call = Explorer::Extrinsic {
		index: 1,
		signed: true,
		success: true,
		signer: Bytes::copy_from_slice(&ALICE),
		section: "balances".into(),
		method: "transferKeepAlive".into(),
		args: r#"[{"name":"value","type":"Compact<Balance>","value":"5000000000000"}]"#.into(),
		eventsCount: 2,
	};
	let mut transfer_log = log(1, 5);
	transfer_log.transaction_hash = B256::repeat_byte(0xcc);
	write_extrinsic(&db, &transfer_log, &transfer_call).await.unwrap();
	for (index, extrinsic, section, method) in [
		(0, 0, "system", "ExtrinsicSuccess"),
		(1, 1, "balances", "Transfer"),
		(2, 1, "system", "ExtrinsicSuccess"),
	] {
		let event = Explorer::Event {
			index,
			isExtrinsic: true,
			extrinsicIndex: extrinsic,
			section: section.into(),
			method: method.into(),
			args: r#"[{"name":"amount","typeName":"Balance","value":"5000000000000"}]"#.into(),
		};
		write_event(&db, &log(extrinsic as u64, 6 + index as u64), &event)
			.await
			.unwrap();
	}
	let transfer = Balances::Transfer {
		from: FixedBytes(ALICE),
		to: FixedBytes(BOB),
		amount: 5_000_000_000_000,
	};
	write_transfer(&db, &log(1, 1), &transfer).await.unwrap();
	let info = (3u32, 0u32, 1u32, 0u32, 900u128, 100u128, 0u128, 0u128).encode();
	write_account(&db, &encode_address(&ALICE, 0).unwrap(), &info, HEIGHT)
		.await
		.unwrap();
	db.commit().await.unwrap();
	url
}

async fn get(base: &str, path: &str) -> Value {
	reqwest::get(format!("{base}{path}")).await.unwrap().json().await.unwrap()
}

async fn graphql(base: &str, query: &str, variables: Value) -> Value {
	let body = json!({ "query": query, "variables": variables });
	let client = reqwest::Client::new();
	let response = client.post(format!("{base}/graphql")).json(&body).send().await.unwrap();
	response.json::<Value>().await.unwrap()["data"].clone()
}

#[tokio::test]
async fn the_site_reads_every_page_it_shows() {
	let Ok(admin_url) = std::env::var("INDEXER_TEST_DATABASE_URL") else { return };
	let _ = rustls::crypto::ring::default_provider().install_default();
	let url = seed(&admin_url).await;
	std::env::set_var("DATABASE_URL", &url);
	let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
	tokio::spawn(omniplex::explorer_api::serve(([127, 0, 0, 1], port).into()));
	let base = format!("http://127.0.0.1:{port}");
	// The server is ready once it answers the overview from the seeded block
	for _ in 0..100 {
		let answer = reqwest::get(format!("{base}/overview")).await;
		let ready = match answer {
			Ok(answer) =>
				answer.json::<Value>().await.is_ok_and(|o| o["finalizedHeight"] == HEIGHT),
			Err(_) => false,
		};
		if ready {
			break;
		}
		tokio::time::sleep(Duration::from_millis(100)).await;
	}

	let alice = encode_address(&ALICE, 0).unwrap();
	let alice_42 = encode_address(&ALICE, 42).unwrap();
	let bob = encode_address(&BOB, 0).unwrap();
	let transfer_hash = B256::repeat_byte(0xcc).to_string();

	let overview = get(&base, "/overview").await;
	assert_eq!(overview["finalizedHeight"], HEIGHT);
	assert_eq!(overview["signedExtrinsics"], 1);
	assert_eq!(overview["accounts"], 1);
	assert_eq!(overview["transfers"], 1, "a table Postgres hasn't analyzed yet is counted");

	let blocks = get(&base, "/latest-blocks").await;
	assert_eq!(blocks[0]["height"], HEIGHT);
	assert_eq!(blocks[0]["extrinsicsCount"], 2);
	assert_eq!(blocks[0]["time"], 1_784_624_058_000u64);

	let extrinsics = get(&base, "/extrinsics?page=0&page_size=25").await;
	let items = extrinsics["items"].as_array().unwrap();
	assert_eq!(items.len(), 1, "the timestamp inherent is not listed");
	assert_eq!(items[0]["indexer"]["extrinsicIndex"], 1);
	assert_eq!(items[0]["signer"], alice);
	assert_eq!(items[0]["args"][0]["value"], "5000000000000");
	assert_eq!(extrinsics["pageSize"], 25);
	let filtered = get(&base, "/extrinsics?section=balances&method=transferKeepAlive").await;
	assert_eq!(filtered["items"].as_array().unwrap().len(), 1);
	let none = get(&base, &format!("/extrinsics?block_start={}", HEIGHT + 1)).await;
	assert!(none["items"].as_array().unwrap().is_empty());

	let by_hash = get(&base, &format!("/extrinsics/{transfer_hash}")).await;
	assert_eq!(by_hash["indexer"]["blockHeight"], HEIGHT);
	let by_index = get(&base, &format!("/extrinsics/{HEIGHT}-1")).await;
	assert_eq!(by_index["hash"], transfer_hash);
	let missing = reqwest::get(format!("{base}/extrinsics/{HEIGHT}-9")).await.unwrap();
	assert_eq!(missing.status(), 404);

	let events = get(&base, "/events").await;
	let events = events["items"].as_array().unwrap();
	assert_eq!(events.len(), 1, "extrinsic results are not listed");
	assert_eq!(events[0]["method"], "Transfer");

	let transfers = get(&base, "/transfers?signed_only=true").await;
	assert_eq!(transfers["items"][0]["from"], alice);
	assert_eq!(transfers["items"][0]["to"], bob);
	assert_eq!(transfers["items"][0]["balance"], "5000000000000");
	assert_eq!(get(&base, "/latest-transfers").await[0]["isSigned"], true);

	let accounts = get(&base, "/accounts").await;
	assert_eq!(accounts["total"], 1);
	assert_eq!(accounts["items"][0]["data"]["total"], "1000");
	let summary = get(&base, &format!("/accounts/{alice_42}/summary")).await;
	assert_eq!(summary, json!({ "transfersCount": 1, "extrinsicsCount": 1 }));
	let theirs = get(&base, &format!("/accounts/{alice_42}/extrinsics")).await;
	assert_eq!(theirs["total"], 1);
	assert_eq!(get(&base, &format!("/accounts/{bob}/transfers")).await["total"], 1);

	assert_eq!(
		get(&base, &format!("/search/hints?term={HEIGHT}")).await,
		json!({ "block": HEIGHT })
	);
	assert_eq!(
		get(&base, &format!("/search/hints?term={transfer_hash}")).await,
		json!({ "extrinsic": transfer_hash })
	);
	assert_eq!(
		get(&base, &format!("/search/hints?term={alice_42}")).await,
		json!({ "account": alice })
	);

	let specs = get(&base, "/specs/filter").await;
	assert_eq!(specs[0]["specVersion"], 7_900);
	let balances = specs[0]["pallets"]
		.as_array()
		.unwrap()
		.iter()
		.find(|p| p["name"] == "balances")
		.unwrap();
	assert_eq!(balances["calls"], json!(["transferKeepAlive"]));
	assert_eq!(balances["events"], json!(["Transfer"]));

	let block = graphql(
		&base,
		"query GetBlockInfo($blockHeightOrHash: BlockHeightOrHash!) { chainBlock(blockHeightOrHash: $blockHeightOrHash) { hash } }",
		json!({ "blockHeightOrHash": HEIGHT }),
	)
	.await;
	assert_eq!(block["chainBlock"]["extrinsics"].as_array().unwrap().len(), 2);
	assert_eq!(block["chainBlock"]["events"].as_array().unwrap().len(), 3);
	assert_eq!(block["chainBlock"]["__typename"], "Block");

	let extrinsic = graphql(
		&base,
		"query GetExtrinsicInfo($blockHeight: Int!, $extrinsicIndex: Int!) { chainExtrinsic(blockHeight: $blockHeight, extrinsicIndex: $extrinsicIndex) { hash } }",
		json!({ "blockHeight": HEIGHT, "extrinsicIndex": 1 }),
	)
	.await;
	assert_eq!(extrinsic["chainExtrinsic"]["call"]["method"], "transferKeepAlive");
	assert_eq!(extrinsic["chainExtrinsic"]["events"].as_array().unwrap().len(), 2);

	let event = graphql(
		&base,
		"query GetEventInfo($blockHeight: Int!, $eventIndex: Int!) { chainEvent(blockHeight: $blockHeight, eventIndex: $eventIndex) { method } }",
		json!({ "blockHeight": HEIGHT, "eventIndex": 2 }),
	)
	.await;
	assert_eq!(event["chainEvent"]["isExtrinsicResult"], true);
	assert_eq!(event["chainEvent"]["docs"], Value::Null);

	let account = graphql(
		&base,
		"query GetAccountInfo($address: String!) { chainAccount(address: $address) { data { total } } }",
		json!({ "address": alice_42 }),
	)
	.await;
	assert_eq!(account["chainAccount"]["data"]["transferrable"], "900");
	assert_eq!(account["chainAccount"]["detail"]["nonce"], 3);
}
