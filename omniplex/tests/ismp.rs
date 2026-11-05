//! The ISMP handlers against the real `app` schema, with mocked RPCs. Set
//! `INDEXER_TEST_DATABASE_URL` to run.

mod common;

use std::{collections::BTreeMap, sync::Arc};

use alloy::{
	primitives::{keccak256, Address, Bytes, B256, U256},
	providers::{Provider, ProviderBuilder},
	sol_types::{SolCall, SolValue},
	transports::mock::Asserter,
};
use common::{database, text};
use omniplex::{
	chains::Chain,
	config::{ChainConfig, ChainKind, Config, Contracts, Environment},
	handlers::{
		abi::{pallets::Ismp, Erc6160Ext20, EthereumHost},
		ismp::{
			calldata::HandlerV2,
			commitment::{get_request_commitment, post_request_commitment},
			evm, substrate, Shared,
		},
	},
	registry::{EventContext, EventHandler, LogMeta},
	services::prices::Orderbook,
	Indexer,
};
use serde_json::json;
use tokio_postgres::Transaction;

const HOST: &str = "0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3";
const GATEWAY: &str = "0xae041f7b0cb581876832830baeb6a2aa2a3c9716";
const TOKEN: &str = "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359";
const BENEFICIARY: &str = "0x3333333333333333333333333333333333333333";
const TRANSFER_TOPIC: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

fn address(value: &str) -> Address {
	value.parse().unwrap()
}

fn chain_config(kind: ChainKind, state_machine_id: &str, host: Option<&str>) -> ChainConfig {
	ChainConfig {
		kind,
		chain_id: 1,
		state_machine_id: state_machine_id.into(),
		start_block: 0,
		reorg_safe_distance: 0,
		max_block_range: None,
		op_stack: false,
		contracts: Contracts { ethereum_host: host.map(address), ..Default::default() },
	}
}

/** An indexer whose chains answer RPCs from their asserters, in order. */
async fn indexer(environment: Environment, chains: &[(&str, ChainConfig, &Asserter)]) -> Indexer {
	let _ = rustls::crypto::ring::default_provider().install_default();
	if std::env::var("DATABASE_URL").is_err() {
		std::env::set_var("DATABASE_URL", std::env::var("INDEXER_TEST_DATABASE_URL").unwrap());
	}
	let config = Config {
		environment,
		chains: chains
			.iter()
			.map(|(name, config, _)| (name.to_string(), config.clone()))
			.collect(),
	};
	let chains: BTreeMap<String, Chain> = chains
		.iter()
		.map(|(name, config, asserter)| {
			let provider =
				ProviderBuilder::new().connect_mocked_client((*asserter).clone()).erased();
			let chain = Chain {
				name: name.to_string(),
				config: config.clone(),
				rpc_urls: vec![],
				provider,
				substrate: None,
			};
			(name.to_string(), chain)
		})
		.collect();
	let http = reqwest::Client::new();
	Indexer {
		config,
		db: Arc::new(rindexer::PostgresClient::new().await.unwrap()),
		chains,
		orderbook: Orderbook::new(None, http.clone()),
		http,
	}
}

fn log(block_number: u64, block_timestamp: u64, transaction_hash: B256) -> LogMeta {
	LogMeta {
		address: Address::ZERO,
		block_number,
		block_hash: B256::repeat_byte(0x33),
		block_timestamp,
		transaction_hash,
		transaction_index: 2,
		log_index: 0,
	}
}

async fn handle<H: EventHandler>(
	indexer: &Indexer,
	network: &str,
	db: &Transaction<'_>,
	log: &LogMeta,
	handler: &H,
	event: H::Event,
) {
	let ctx = EventContext { indexer, chain: &indexer.chains[network], db, log };
	handler.handle(&ctx, event).await.unwrap();
}

fn pad32(address: &str) -> String {
	format!("0x{:0>64}", address.trim_start_matches("0x"))
}

fn transfer_receipt(transaction_hash: B256) -> serde_json::Value {
	json!({
		"logs": [{
			"address": TOKEN,
			"topics": [TRANSFER_TOPIC, pad32(GATEWAY), pad32(BENEFICIARY)],
			"data": format!("0x{:064x}", 25_000_000u64),
			"transactionHash": format!("{transaction_hash:#x}"),
		}],
		"gasUsed": "0x5208",
		"effectiveGasPrice": "0x3b9aca00",
	})
}

fn empty_receipt() -> serde_json::Value {
	json!({ "logs": [], "gasUsed": "0x5208", "effectiveGasPrice": "0x1" })
}

/** What a relayer sends: a consensus update and the requests it proves, in one `batchCall`. */
fn batched_delivery() -> Bytes {
	use HandlerV2::*;
	let host = address(HOST);
	let request = PostRequest {
		source: Bytes::from(b"POLKADOT-3367".to_vec()),
		dest: Bytes::from(b"EVM-137".to_vec()),
		nonce: 7,
		from: GATEWAY.parse().unwrap(),
		to: GATEWAY.parse().unwrap(),
		timeoutTimestamp: 0,
		body: "0x1234".parse().unwrap(),
	};
	let delivery = handlePostRequestsCall {
		host,
		request: PostRequestMessage {
			proof: Proof {
				height: StateMachineHeight {
					stateMachineId: U256::from(3367),
					height: U256::from(100),
				},
				multiproof: vec![],
				leafCount: U256::from(1),
			},
			requests: vec![PostRequestLeaf { request, index: U256::ZERO }],
		},
	};
	let consensus = handleConsensusCall { host, proof: "0xdeadbeef".parse().unwrap() };
	batchCallCall { calls: vec![consensus.abi_encode().into(), delivery.abi_encode().into()] }
		.abi_encode()
		.into()
}

/** The token's `symbol()` and `decimals()`, in the order the sweep asks. */
fn push_usdc(asserter: &Asserter) {
	asserter.push_success(&Bytes::from("USDC".to_string().abi_encode()));
	asserter.push_success(&Bytes::from(U256::from(6).abi_encode()));
}

/** `postRequestHandled.handler.test.ts`, plus what the TypeScript test mocked away. */
#[tokio::test]
async fn a_batched_delivery_records_its_transfers_and_attributes_them_to_the_recipient() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let asserter = Asserter::new();
	let config = chain_config(ChainKind::Evm, "EVM-137", Some(HOST));
	let indexer = indexer(Environment::Mainnet, &[("polygon", config, &asserter)]).await;
	let tx = B256::repeat_byte(0x22);
	asserter.push_success(&transfer_receipt(tx));
	asserter.push_success(&json!({ "input": batched_delivery() }));
	push_usdc(&asserter);

	let handler = evm::PostRequestHandled(Arc::new(Shared::default()));
	let event = EthereumHost::PostRequestHandled {
		commitment: B256::repeat_byte(0x11),
		relayer: address(BENEFICIARY),
	};
	handle(&indexer, "polygon", &db, &log(99, 1_700, tx), &handler, event).await;

	let commitment = format!("{:#x}", B256::repeat_byte(0x11));
	let pending = text(
		&db,
		&format!(
			"SELECT status || ' ' || chain || ' ' || timestamp FROM app.pending_status_metadata
			 WHERE id = '{commitment}.RequestV2.DESTINATION'"
		),
	)
	.await;
	assert_eq!(pending.as_deref(), Some("DESTINATION EVM-137 1700"), "no request yet: pending");

	let transfer = text(
		&db,
		&format!(
			"SELECT chain || ' ' || amount || ' ' || \"from\" || ' ' || \"to\" FROM app.transfers
			 WHERE id = '{tx:#x}-index-0'"
		),
	)
	.await;
	assert_eq!(transfer, Some(format!("EVM-137 25000000 {GATEWAY} {BENEFICIARY}")));
	let volume = |id: String| {
		let db = &db;
		async move {
			text(
				db,
				&format!(
					"SELECT volume_u_s_d::text FROM app.cumulative_volume_u_s_ds WHERE id = '{id}'"
				),
			)
			.await
		}
	};
	assert_eq!(
		volume("Transfer.USDC.EVM-137".into()).await.as_deref(),
		Some("25000000000000000000")
	);
	assert_eq!(
		volume(format!("Contract.{GATEWAY}.EVM-137")).await.as_deref(),
		Some("25000000000000000000")
	);

	let stats = text(
		&db,
		&format!(
			"SELECT number_of_successful_messages_delivered || ' ' || gas_used_for_successful_messages
			   || ' ' || gas_fee_for_successful_messages || ' ' || usd_gas_fee_for_successful_messages
			 FROM app.relayer_stats_per_chain_v2s WHERE id = '{BENEFICIARY}-EVM-137'"
		),
	)
	.await;
	assert_eq!(stats.as_deref(), Some("1 21000 21000000000000 0"));
	let delivered = text(
		&db,
		"SELECT number_of_delivered_messages::text FROM app.hyper_bridge_stats WHERE id = 'EVM-137'",
	)
	.await;
	assert_eq!(delivered.as_deref(), Some("1"));
	let points = text(
		&db,
		&format!("SELECT points::text FROM app.rewards WHERE id = '{BENEFICIARY}-EVM-137-RELAYER'"),
	)
	.await;
	assert_eq!(points.as_deref(), Some("50"));
	let activity = text(
		&db,
		&format!(
			"SELECT last_updated_at::text FROM app.relayer_activities WHERE id = '{BENEFICIARY}'"
		),
	)
	.await;
	assert_eq!(activity.as_deref(), Some("1700"));
}

#[tokio::test]
async fn unknown_calldata_still_records_the_transfers_without_a_contract() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let asserter = Asserter::new();
	let config = chain_config(ChainKind::Evm, "EVM-137", Some(HOST));
	let indexer = indexer(Environment::Mainnet, &[("polygon", config, &asserter)]).await;
	let tx = B256::repeat_byte(0x22);
	asserter.push_success(&transfer_receipt(tx));
	let mut erc20_transfer = vec![0xa9, 0x05, 0x9c, 0xbb];
	erc20_transfer.extend([0u8; 64]);
	asserter.push_success(&json!({ "input": Bytes::from(erc20_transfer) }));
	push_usdc(&asserter);

	let handler = evm::PostRequestHandled(Arc::new(Shared::default()));
	let event = EthereumHost::PostRequestHandled {
		commitment: B256::repeat_byte(0x11),
		relayer: address(BENEFICIARY),
	};
	handle(&indexer, "polygon", &db, &log(99, 1_700, tx), &handler, event).await;

	let transfers =
		text(&db, "SELECT count(*)::text FROM app.transfers WHERE chain = 'EVM-137'").await;
	assert_eq!(transfers.as_deref(), Some("1"));
	let volumes = text(
		&db,
		"SELECT string_agg(id, ',' ORDER BY id) FROM app.cumulative_volume_u_s_ds WHERE id LIKE '%.EVM-137'",
	)
	.await;
	assert_eq!(volumes.as_deref(), Some("Transfer.USDC.EVM-137"));
}

/** A status indexed before its request waits, and the request's insert flushes it. */
#[tokio::test]
async fn a_post_request_flushes_the_statuses_that_waited_for_it() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let (ethereum, polygon) = (Asserter::new(), Asserter::new());
	let indexer = indexer(
		Environment::Mainnet,
		&[
			("ethereum", chain_config(ChainKind::Evm, "EVM-1", Some(HOST)), &ethereum),
			("polygon", chain_config(ChainKind::Evm, "EVM-137", Some(HOST)), &polygon),
		],
	)
	.await;
	let to: Bytes = BENEFICIARY.parse().unwrap();
	let body: Bytes = "0x1234".parse().unwrap();
	let commitment = post_request_commitment(
		"EVM-1",
		"EVM-137",
		7,
		1_800_000_000,
		address(GATEWAY).as_slice(),
		&to,
		&body,
	);

	polygon.push_success(&empty_receipt());
	polygon.push_success(&json!({ "input": "0x" }));
	let handled = evm::PostRequestHandled(Arc::new(Shared::default()));
	let event = EthereumHost::PostRequestHandled { commitment, relayer: address(BENEFICIARY) };
	handle(
		&indexer,
		"polygon",
		&db,
		&log(50, 1_700_000_100, B256::repeat_byte(2)),
		&handled,
		event,
	)
	.await;

	ethereum.push_success(&empty_receipt());
	let dispatched = evm::PostRequestEvent(Arc::new(Shared::default()));
	let event = EthereumHost::PostRequestEvent {
		source: "EVM-1".into(),
		dest: "EVM-137".into(),
		from: address(GATEWAY),
		to: to.clone(),
		nonce: U256::from(7),
		timeoutTimestamp: U256::from(1_800_000_000u64),
		body,
		fee: U256::from(5_000_000u64),
	};
	handle(
		&indexer,
		"ethereum",
		&db,
		&log(10, 1_700_000_000, B256::repeat_byte(1)),
		&dispatched,
		event,
	)
	.await;

	let request = text(
		&db,
		&format!(
			"SELECT chain || ' ' || source || ' ' || dest || ' ' || \"from\" || ' ' || \"to\" || ' ' || nonce
			   || ' ' || timeout_timestamp || ' ' || body || ' ' || fee || ' ' || created_at
			 FROM app.request_v2s WHERE id = '{commitment:#x}'"
		),
	)
	.await;
	let from = address(GATEWAY).to_checksum(None);
	assert_eq!(
		request,
		Some(format!(
			"EVM-1 EVM-1 EVM-137 {from} {BENEFICIARY} 7 1800000000 0x1234 5000000 2023-11-14 22:13:20"
		))
	);
	let statuses = text(
		&db,
		&format!(
			"SELECT string_agg(status || '@' || chain, ',' ORDER BY timestamp) FROM app.request_status_metadata
			 WHERE request_id = '{commitment:#x}'"
		),
	)
	.await;
	assert_eq!(statuses.as_deref(), Some("SOURCE@EVM-1,DESTINATION@EVM-137"));
	let pending = text(&db, "SELECT count(*)::text FROM app.pending_status_metadata").await;
	assert_eq!(pending.as_deref(), Some("0"));
	let sent = text(
		&db,
		"SELECT number_of_messages_sent::text FROM app.hyper_bridge_stats WHERE id = 'EVM-1'",
	)
	.await;
	assert_eq!(sent.as_deref(), Some("1"));
}

/** `transfer.event.handler.test.ts`: the transfer keeps the bare transaction hash and checksums. */
#[tokio::test]
async fn fee_token_transfers_out_of_a_host_pay_relayers() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let host = "0x92F217a5e965EAa2aD356678D537A0A9ccC0AF41";
	let relayer = "0xbC50b90751bfCccbFa4c7220261909d0f528b00f";
	let asserter = Asserter::new();
	let config = chain_config(ChainKind::Evm, "EVM-11155111", Some(host));
	let indexer = indexer(Environment::Testnet, &[("sepolia", config, &asserter)]).await;
	let tx: B256 = "0xbf94293f2dbd11e71510d9add4338d5f362a67c69df6055c1c4c9bc965ac1f31"
		.parse()
		.unwrap();
	let amount: U256 = "24000000000000000000000".parse().unwrap();
	let event = Erc6160Ext20::Transfer { from: address(host), to: address(relayer), value: amount };
	handle(
		&indexer,
		"sepolia",
		&db,
		&log(5_854_079, 1_712_000_000, tx),
		&evm::FeeTokenTransfer,
		event,
	)
	.await;
	let incoming = Erc6160Ext20::Transfer {
		from: address(BENEFICIARY),
		to: address(host),
		value: U256::from(3),
	};
	let other = B256::repeat_byte(9);
	handle(
		&indexer,
		"sepolia",
		&db,
		&log(5_854_080, 1_712_000_012, other),
		&evm::FeeTokenTransfer,
		incoming,
	)
	.await;
	let unrelated = Erc6160Ext20::Transfer {
		from: address(BENEFICIARY),
		to: address(GATEWAY),
		value: U256::from(1),
	};
	handle(
		&indexer,
		"sepolia",
		&db,
		&log(5_854_081, 1_712_000_024, B256::repeat_byte(8)),
		&evm::FeeTokenTransfer,
		unrelated,
	)
	.await;

	let transfer = text(
		&db,
		&format!(
			"SELECT chain || ' ' || amount || ' ' || \"from\" || ' ' || \"to\" FROM app.transfers WHERE id = '{tx:#x}'"
		),
	)
	.await;
	assert_eq!(transfer, Some(format!("EVM-11155111 24000000000000000000000 {host} {relayer}")));
	let transfers = text(&db, "SELECT count(*)::text FROM app.transfers").await;
	assert_eq!(transfers.as_deref(), Some("2"));
	let earned = text(
		&db,
		&format!("SELECT fees_earned::text FROM app.relayer_stats_per_chain_v2s WHERE id = '{relayer}-EVM-11155111'"),
	)
	.await;
	assert_eq!(earned.as_deref(), Some("24000000000000000000000"));
	let stats = text(
		&db,
		"SELECT fees_payed_out_to_relayers || ' ' || total_transfers_in FROM app.hyper_bridge_stats WHERE id = 'EVM-11155111'",
	)
	.await;
	assert_eq!(stats.as_deref(), Some("24000000000000000000000 3"));
}

/** `abi.encode(bytes)` of a SCALE value, as substrate-eth-rpc answers a storage read. */
fn storage(value: &[u8]) -> Bytes {
	Bytes::from(Bytes::copy_from_slice(value).abi_encode())
}

fn hyperbridge(asserter: &Asserter) -> (&'static str, ChainConfig, &Asserter) {
	("nexus", chain_config(ChainKind::Substrate, "POLKADOT-3367", None), asserter)
}

/**
 * Hyperbridge's `GetRequestHandled` creates the GET request it hasn't seen, dated by the block;
 * the source chain's `GetRequestEvent` later fills it in and keeps that date.
 */
#[tokio::test]
async fn get_requests_handled_before_they_are_indexed_get_a_skeleton() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let (nexus, bsc) = (Asserter::new(), Asserter::new());
	let indexer = indexer(
		Environment::Mainnet,
		&[hyperbridge(&nexus), ("bsc", chain_config(ChainKind::Evm, "EVM-56", Some(HOST)), &bsc)],
	)
	.await;
	let keys = vec![Bytes::from(vec![1u8, 2])];
	let context: Bytes = "0x".parse().unwrap();
	let from: Bytes = GATEWAY.parse().unwrap();
	let commitment =
		get_request_commitment("EVM-56", "EVM-1", 4, 100, 1_800_000_000, &from, &keys, &context);

	let ms = 1_700_000_000_123u64;
	nexus.push_success(&storage(&ms.to_le_bytes()));
	let handled = substrate::GetRequestHandled(Arc::new(Shared::default()));
	let event = Ismp::GetRequestHandled { commitment, relayer: vec![0xab, 0xcd].into() };
	handle(&indexer, "nexus", &db, &log(9, 1_700_000_000, B256::repeat_byte(5)), &handled, event)
		.await;

	let skeleton = text(
		&db,
		&format!(
			"SELECT source || '|' || chain || '|' || keys::text || '|' || created_at FROM app.get_request_v2s
			 WHERE id = '{commitment:#x}'"
		),
	)
	.await;
	assert_eq!(skeleton.as_deref(), Some("||[\"\"]|2023-11-14 22:13:20.123"));
	let status = text(
		&db,
		&format!(
			"SELECT status || ' ' || chain || ' ' || timestamp FROM app.get_request_status_metadata
			 WHERE id = '{commitment:#x}.HYPERBRIDGE_DELIVERED'"
		),
	)
	.await;
	assert_eq!(
		status.as_deref(),
		Some(format!("HYPERBRIDGE_DELIVERED POLKADOT-3367 {ms}").as_str())
	);
	let relayer = text(
		&db,
		"SELECT number_of_successful_messages_delivered::text FROM app.relayer_stats_per_chain_v2s
		 WHERE id = '0xabcd-POLKADOT-3367'",
	)
	.await;
	assert_eq!(relayer.as_deref(), Some("1"), "S6 names relayers by their raw bytes");

	bsc.push_success(&empty_receipt());
	let dispatched = evm::GetRequestEvent(Arc::new(Shared::default()));
	let event = EthereumHost::GetRequestEvent {
		source: "EVM-56".into(),
		dest: "EVM-1".into(),
		from,
		keys,
		height: U256::from(100),
		nonce: U256::from(4),
		timeoutTimestamp: U256::from(1_800_000_000u64),
		context,
		fee: U256::ZERO,
	};
	handle(&indexer, "bsc", &db, &log(77, 1_800_000_000, B256::repeat_byte(6)), &dispatched, event)
		.await;
	let filled = text(
		&db,
		&format!(
			"SELECT source || '|' || chain || '|' || keys::text || '|' || context || '|' || block_timestamp
			   || '|' || created_at FROM app.get_request_v2s WHERE id = '{commitment:#x}'"
		),
	)
	.await;
	assert_eq!(
		filled.as_deref(),
		Some("EVM-56|EVM-56|[\"0x0102\"]|0x|1800000000|2023-11-14 22:13:20.123")
	);
}

/**
 * S4, S5: a Hyperbridge status for a request no chain has indexed yet waits for the request.
 * Once the request is indexed, a delivery to Hyperbridge itself is its `DESTINATION`. The
 * delivery counts toward the relayer's stats right away. SubQuery dropped both statuses.
 */
#[tokio::test]
async fn hyperbridge_statuses_wait_for_their_request() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let (nexus, ethereum) = (Asserter::new(), Asserter::new());
	let indexer = indexer(
		Environment::Mainnet,
		&[
			hyperbridge(&nexus),
			("ethereum", chain_config(ChainKind::Evm, "EVM-1", Some(HOST)), &ethereum),
		],
	)
	.await;
	let key = [0x58u8; 32];
	let ms = 1_700_000_000_000u64;
	let to: Bytes = BENEFICIARY.parse().unwrap();
	let body: Bytes = "0x1234".parse().unwrap();
	let commitment = |dest: &str, nonce: u64| {
		post_request_commitment(
			"EVM-1",
			dest,
			nonce,
			1_800_000_000,
			address(GATEWAY).as_slice(),
			&to,
			&body,
		)
	};
	let for_hyperbridge = commitment("POLKADOT-3367", 1);
	let routed = commitment("EVM-137", 2);

	nexus.push_success(&storage(&ms.to_le_bytes()));
	let handled = substrate::PostRequestHandled(Arc::new(Shared::default()));
	let event =
		Ismp::PostRequestHandled { commitment: for_hyperbridge, relayer: key.to_vec().into() };
	handle(&indexer, "nexus", &db, &log(9, 1_700_000_000, B256::repeat_byte(5)), &handled, event)
		.await;
	nexus.push_success(&storage(&ms.to_le_bytes()));
	let timed_out = substrate::PostRequestTimeoutHandled(Arc::new(Shared::default()));
	let event = Ismp::PostRequestTimeoutHandled {
		commitment: routed,
		source: "EVM-1".into(),
		dest: "EVM-137".into(),
	};
	let meta = log(9, 1_700_000_000, B256::repeat_byte(6));
	handle(&indexer, "nexus", &db, &meta, &timed_out, event).await;
	let waiting = text(
		&db,
		"SELECT string_agg(status, ',' ORDER BY status) FROM app.pending_status_metadata",
	)
	.await;
	assert_eq!(waiting.as_deref(), Some("HYPERBRIDGE_DELIVERED,HYPERBRIDGE_TIMED_OUT"));
	let delivered = text(
		&db,
		"SELECT number_of_delivered_messages::text FROM app.hyper_bridge_stats WHERE id = 'POLKADOT-3367'",
	)
	.await;
	assert_eq!(delivered.as_deref(), Some("1"));
	let relayer = omniplex::substrate::ss58(&key, 42);
	let activity = text(
		&db,
		&format!("SELECT last_updated_at::text FROM app.relayer_activities WHERE id = '{relayer}'"),
	)
	.await;
	assert_eq!(activity, Some(ms.to_string()), "Substrate timestamps are milliseconds");

	let dispatched = evm::PostRequestEvent(Arc::new(Shared::default()));
	for (dest, nonce) in [("POLKADOT-3367", 1u8), ("EVM-137", 2)] {
		ethereum.push_success(&empty_receipt());
		let event = EthereumHost::PostRequestEvent {
			source: "EVM-1".into(),
			dest: dest.into(),
			from: address(GATEWAY),
			to: to.clone(),
			nonce: U256::from(nonce),
			timeoutTimestamp: U256::from(1_800_000_000u64),
			body: body.clone(),
			fee: U256::from(5_000_000u64),
		};
		let meta = log(10, 1_700_000_000, B256::repeat_byte(nonce));
		handle(&indexer, "ethereum", &db, &meta, &dispatched, event).await;
	}
	for (commitment, expected) in [
		(for_hyperbridge, "SOURCE@EVM-1,DESTINATION@POLKADOT-3367"),
		(routed, "SOURCE@EVM-1,HYPERBRIDGE_TIMED_OUT@POLKADOT-3367"),
	] {
		let statuses = text(
			&db,
			&format!(
				"SELECT string_agg(status || '@' || chain, ',' ORDER BY timestamp)
				 FROM app.request_status_metadata WHERE request_id = '{commitment:#x}'"
			),
		)
		.await;
		assert_eq!(statuses.as_deref(), Some(expected));
	}
	let waiting = text(&db, "SELECT count(*)::text FROM app.pending_status_metadata").await;
	assert_eq!(waiting.as_deref(), Some("0"));
}

/**
 * Off Hyperbridge, a delivery is the request's `DESTINATION` before the request is indexed too.
 * So Hyperbridge's routing status and the destination's wait under different ids, and both
 * land.
 */
#[tokio::test]
async fn deliveries_on_two_substrate_chains_both_wait() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let (nexus, polkadex, ethereum) = (Asserter::new(), Asserter::new(), Asserter::new());
	let indexer = indexer(
		Environment::Mainnet,
		&[
			hyperbridge(&nexus),
			("polkadex", chain_config(ChainKind::Substrate, "SUBSTRATE-PDEX", None), &polkadex),
			("ethereum", chain_config(ChainKind::Evm, "EVM-1", Some(HOST)), &ethereum),
		],
	)
	.await;
	let key = [0x58u8; 32];
	let to: Bytes = BENEFICIARY.parse().unwrap();
	let body: Bytes = "0x1234".parse().unwrap();
	let commitment = post_request_commitment(
		"EVM-1",
		"SUBSTRATE-PDEX",
		3,
		1_800_000_000,
		address(GATEWAY).as_slice(),
		&to,
		&body,
	);
	let ms = 1_700_000_000_000u64;
	for (network, node, at) in [("nexus", &nexus, ms), ("polkadex", &polkadex, ms + 6000)] {
		node.push_success(&storage(&at.to_le_bytes()));
		let handled = substrate::PostRequestHandled(Arc::new(Shared::default()));
		let event = Ismp::PostRequestHandled { commitment, relayer: key.to_vec().into() };
		let meta = log(9, 1_700_000_000, B256::repeat_byte(5));
		handle(&indexer, network, &db, &meta, &handled, event).await;
	}
	let waiting = text(
		&db,
		"SELECT string_agg(status || '@' || chain, ',' ORDER BY status) FROM app.pending_status_metadata",
	)
	.await;
	assert_eq!(
		waiting.as_deref(),
		Some("DESTINATION@SUBSTRATE-PDEX,HYPERBRIDGE_DELIVERED@POLKADOT-3367")
	);

	ethereum.push_success(&empty_receipt());
	let dispatched = evm::PostRequestEvent(Arc::new(Shared::default()));
	let event = EthereumHost::PostRequestEvent {
		source: "EVM-1".into(),
		dest: "SUBSTRATE-PDEX".into(),
		from: address(GATEWAY),
		to: to.clone(),
		nonce: U256::from(3),
		timeoutTimestamp: U256::from(1_800_000_000u64),
		body: body.clone(),
		fee: U256::from(5_000_000u64),
	};
	let meta = log(10, 1_700_000_000, B256::repeat_byte(1));
	handle(&indexer, "ethereum", &db, &meta, &dispatched, event).await;
	let statuses = text(
		&db,
		&format!(
			"SELECT string_agg(status || '@' || chain, ',' ORDER BY timestamp)
			 FROM app.request_status_metadata WHERE request_id = '{commitment:#x}'"
		),
	)
	.await;
	assert_eq!(
		statuses.as_deref(),
		Some("SOURCE@EVM-1,HYPERBRIDGE_DELIVERED@POLKADOT-3367,DESTINATION@SUBSTRATE-PDEX")
	);
}

/** `relayer.AccumulateFees` keeps the fees of a relayer named by an EVM address, which SubQuery
 * dropped. It is named as `Withdraw` names it, in lowercase hex. */
#[tokio::test]
async fn accumulated_fees_keep_evm_relayers() {
	use omniplex::handlers::{abi::pallets::Relayer, pallets::relayer};
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let nexus = Asserter::new();
	let indexer = indexer(Environment::Mainnet, &[hyperbridge(&nexus)]).await;
	// `HostExecutive.FeeTokenDecimals(EVM-1)`: 6.
	nexus.push_success(&storage(&[6]));
	let event = Relayer::AccumulateFees {
		relayer: "0xBC50B907000000000000000000000000000000AA".parse().unwrap(),
		stateMachine: "EVM-1".into(),
		amount: U256::from(5_000_000_000_000_000u64),
	};
	let meta = log(9, 1_700_000_000, B256::repeat_byte(5));
	handle(&indexer, "nexus", &db, &meta, &relayer::AccumulateFees, event).await;
	let fees = text(
		&db,
		"SELECT lifetime_fees || ' ' || relayer FROM app.accumulated_fees
		 WHERE id = '0xbc50b907000000000000000000000000000000aa-EVM-1'",
	)
	.await;
	assert_eq!(fees.as_deref(), Some("5000 0xbc50b907000000000000000000000000000000aa"));
}

/** S1: the commitment timestamp comes from `BoundedStateCommitments` at the event's block. */
#[tokio::test]
async fn substrate_state_machine_updates_read_the_commitment() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let nexus = Asserter::new();
	let indexer = indexer(Environment::Mainnet, &[hyperbridge(&nexus)]).await;
	let ms = 1_700_000_000_500u64;
	nexus.push_success(&storage(&ms.to_le_bytes()));
	let mut commitment = 1_699_999_000u64.to_le_bytes().to_vec();
	commitment.push(0);
	commitment.extend([7u8; 32]);
	nexus.push_success(&storage(&commitment));

	let handler = substrate::StateMachineUpdated(Arc::new(Shared::default()));
	let event = Ismp::StateMachineUpdated {
		stateMachineId: Ismp::StateMachineId {
			stateId: "EVM-56".into(),
			consensusStateId: (*b"BSC0").into(),
		},
		latestHeight: 1234,
	};
	let block_hash = B256::repeat_byte(0x33);
	let init = keccak256([block_hash.as_slice(), b"init"].concat());
	handle(&indexer, "nexus", &db, &log(9, 1_700_000_000, init), &handler, event).await;
	let row = text(
		&db,
		"SELECT chain || ' ' || height || ' ' || transaction_index || ' ' || commitment_timestamp || ' ' || created_at
		 FROM app.state_machine_update_events WHERE id = 'EVM-56--1234'",
	)
	.await;
	assert_eq!(row.as_deref(), Some("POLKADOT-3367 1234 0 1699999000 2023-11-14 22:13:20.5"));
}

/** E7: only Polkadot and Kusama state machines, with the timestamp of the host's commitment. */
#[tokio::test]
async fn evm_state_machine_updates_read_the_hosts_commitment() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let bsc = Asserter::new();
	let indexer = indexer(
		Environment::Mainnet,
		&[("bsc", chain_config(ChainKind::Evm, "EVM-56", Some(HOST)), &bsc)],
	)
	.await;
	let tx = B256::repeat_byte(3);
	let ignored =
		EthereumHost::StateMachineUpdated { stateMachineId: "EVM-1".into(), height: U256::from(5) };
	handle(&indexer, "bsc", &db, &log(40, 1_700_000_000, tx), &evm::StateMachineUpdated, ignored)
		.await;

	// `stateMachineCommitment` returns (timestamp, overlayRoot, stateRoot).
	let commitment = (U256::from(1_793_708_886u64), B256::repeat_byte(1), B256::repeat_byte(2));
	bsc.push_success(&Bytes::from(commitment.abi_encode()));
	let event = EthereumHost::StateMachineUpdated {
		stateMachineId: "POLKADOT-3367".into(),
		height: U256::from(12_597_213),
	};
	handle(&indexer, "bsc", &db, &log(40, 1_700_000_000, tx), &evm::StateMachineUpdated, event)
		.await;
	let rows = text(
		&db,
		"SELECT string_agg(id || ' ' || commitment_timestamp || ' ' || block_number || ' ' || transaction_index, ',')
		 FROM app.state_machine_update_events",
	)
	.await;
	assert_eq!(rows, Some(format!("EVM-56_{tx:#x}_POLKADOT-3367_12597213 1793708886 40 2")));
}

/** On Hyperbridge, `Response` comes before `GetRequestHandled` in the same block, so the
 * response is written before the GET request it points at. */
#[tokio::test]
async fn a_get_response_keeps_its_request_before_the_request_is_indexed() {
	use omniplex::{
		handlers::ismp::requests::{insert_get_response, GetResponseRow},
		store::numeric,
	};
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let response = "0x61567ec5fa5699d1ec48bed7a456ade633572f3217451d0913318d2383e286f2";
	let request = "0xba51a2830d33dcb9c41c533150a97d94b47d30033837ee79815e03292dbc8162";
	insert_get_response(
		&db,
		&GetResponseRow {
			commitment: response,
			chain: "KUSAMA-4009",
			request,
			values: vec![Some(String::new())],
			timeout_timestamp: numeric(0),
			block_timestamp: 1_791_463_710,
			block_number: 10_805_856,
			block_hash: "0xblock",
			transaction_hash: "0xfdce",
		},
	)
	.await
	.unwrap();
	let row = text(
		&db,
		&format!(
			"SELECT request_id || ' ' || created_at FROM app.get_responses WHERE id = '{response}'"
		),
	)
	.await;
	assert_eq!(row, Some(format!("{request} 2026-10-08 12:48:30")));
}
