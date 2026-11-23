//! The governance handlers against the real `governance` schema, on a database of their own,
//! reading storage from a stub of substrate-eth-rpc. Set `INDEXER_TEST_DATABASE_URL` to run.

use std::{
	collections::BTreeMap,
	sync::{Arc, Mutex},
};

use alloy::{
	primitives::{Address, Bytes, B256},
	providers::{Provider, ProviderBuilder},
	sol,
	sol_types::{SolCall, SolValue},
};
use axum::{extract::State, routing::post, Json, Router};
use codec::Encode;
use omniplex::{
	chains::Chain,
	config::{ChainConfig, ChainKind, Config, Contracts, Environment, Mode},
	handlers::{abi::pallets::Governance, governance},
	registry::{EventContext, EventHandler, LogMeta},
	services::prices::Orderbook,
	substrate::{account_id, ss58},
	Indexer,
};
use serde_json::{json, Value};
use sp_crypto_hashing::{blake2_128, twox_128, twox_64};
use tokio::sync::OnceCell;
use tokio_postgres::{NoTls, Transaction};

sol! {
	interface SubstrateStorage {
		function getStorage(bytes key) external view returns (bytes value);
		function getStorageJson(bytes key) external view returns (string value);
		function getConstantJson(string pallet, string name) external view returns (string value);
		function decodeCallJson(bytes call) external view returns (string value);
		function getExtrinsicJson(uint32 index) external view returns (string value);
	}
}

const NETWORK: &str = "nexus-governance";

fn alice() -> String {
	ss58(&[1; 32], 0)
}

fn bob() -> String {
	ss58(&[2; 32], 0)
}

/**
 * substrate-eth-rpc's storage `eth_call`s, answered from canned values. Each answer holds from
 * its block until a later one replaces it, and anything without one reads as absent.
 */
#[derive(Clone, Default)]
struct Node(Arc<Mutex<BTreeMap<Vec<u8>, BTreeMap<u64, Bytes>>>>);

impl Node {
	async fn start(self) -> String {
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let url = format!("http://{}", listener.local_addr().unwrap());
		let app = Router::new().route("/", post(answer)).with_state(self);
		tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
		url
	}

	fn set(&self, call: impl SolCall, from: u64, answer: Vec<u8>) {
		let mut answers = self.0.lock().unwrap();
		answers.entry(call.abi_encode()).or_default().insert(from, answer.into());
	}

	fn storage(&self, key: Vec<u8>, from: u64, value: Vec<u8>) {
		let call = SubstrateStorage::getStorageCall { key: key.into() };
		self.set(call, from, Bytes::from(value).abi_encode());
	}

	/** A JSON storage value from `from` on, or its removal for `None`. */
	fn json(&self, key: Vec<u8>, from: u64, value: Option<Value>) {
		let call = SubstrateStorage::getStorageJsonCall { key: key.into() };
		self.set(call, from, value.map(|v| v.to_string()).unwrap_or_default().abi_encode());
	}

	fn tracks(&self, from: u64, tracks: Value) {
		let call = SubstrateStorage::getConstantJsonCall {
			pallet: "Referenda".into(),
			name: "Tracks".into(),
		};
		self.set(call, from, tracks.to_string().abi_encode());
	}

	fn constant(&self, pallet: &str, name: &str, from: u64, value: Value) {
		let call =
			SubstrateStorage::getConstantJsonCall { pallet: pallet.into(), name: name.into() };
		self.set(call, from, value.to_string().abi_encode());
	}

	fn extrinsic(&self, index: u32, from: u64, call: Value) {
		let call_json = SubstrateStorage::getExtrinsicJsonCall { index };
		self.set(call_json, from, call.to_string().abi_encode());
	}

	fn issuance(&self, from: u64, total: u128, inactive: u128) {
		self.storage(key("Balances", "TotalIssuance"), from, total.encode());
		self.storage(key("Balances", "InactiveIssuance"), from, inactive.encode());
	}
}

async fn answer(State(node): State<Node>, Json(request): Json<Value>) -> Json<Value> {
	let params = &request["params"];
	let input = params[0]["input"].as_str().or(params[0]["data"].as_str()).unwrap_or_default();
	let input = hex::decode(input.trim_start_matches("0x")).unwrap();
	let block = u64::from_str_radix(params[1].as_str().unwrap().trim_start_matches("0x"), 16);
	let answers = node.0.lock().unwrap();
	let result = answers
		.get(&input)
		.and_then(|by_block| by_block.range(..=block.unwrap()).next_back())
		.map(|(_, answer)| answer.clone())
		.unwrap_or_else(|| Bytes::new().abi_encode().into());
	Json(json!({ "jsonrpc": "2.0", "id": request["id"], "result": result }))
}

fn key(pallet: &str, entry: &str) -> Vec<u8> {
	[twox_128(pallet.as_bytes()), twox_128(entry.as_bytes())].concat()
}

fn referendum_key(index: u32) -> Vec<u8> {
	let index = index.encode();
	[key("Referenda", "ReferendumInfoFor"), blake2_128(&index).to_vec(), index].concat()
}

fn voting_key(account: &str, track: u16) -> Vec<u8> {
	let (who, track) = (account_id(account).unwrap(), track.encode());
	[
		key("ConvictionVoting", "VotingFor"),
		twox_64(&who).to_vec(),
		who.to_vec(),
		twox_64(&track).to_vec(),
		track,
	]
	.concat()
}

fn track(id: u16, name: &str) -> Value {
	json!([id, { "name": name, "maxDeciding": 10, "decisionDeposit": "1000" }])
}

fn ongoing(track: u16, ayes: u64) -> Value {
	json!({ "ongoing": {
		"track": track,
		"submissionDeposit": { "who": alice(), "amount": 1 },
		"tally": { "ayes": ayes, "nays": 0, "support": ayes },
	} })
}

fn casting(votes: Value, delegated: u64) -> Value {
	json!({ "casting": {
		"votes": votes,
		"delegations": { "votes": delegated, "capital": delegated },
		"prior": [0, 0],
	} })
}

fn standard(aye: bool, conviction: u8, balance: u64) -> Value {
	json!({ "standard": { "vote": (aye as u8) << 7 | conviction, "balance": balance } })
}

fn delegating(target: &str, conviction: &str, balance: u64) -> Value {
	json!({ "delegating": { "balance": balance, "target": target, "conviction": conviction } })
}

/** The `governance_handlers_test` database, created once for every test here. */
async fn database_url() -> Option<String> {
	static URL: OnceCell<String> = OnceCell::const_new();
	let admin_url = std::env::var("INDEXER_TEST_DATABASE_URL").ok()?;
	let url = URL
		.get_or_init(|| async {
			let name = "governance_handlers_test";
			let (admin, connection) = tokio_postgres::connect(&admin_url, NoTls).await.unwrap();
			let connection = tokio::spawn(connection);
			admin
				.batch_execute(&format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)"))
				.await
				.unwrap();
			admin.batch_execute(&format!("CREATE DATABASE {name}")).await.unwrap();
			drop(admin);
			connection.await.unwrap().unwrap();
			let url = format!("{}/{name}", admin_url.rsplit_once('/').unwrap().0);
			let (mut client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
			let connection = tokio::spawn(connection);
			omniplex::db::apply_migrations(&mut client).await.unwrap();
			drop(client);
			connection.await.unwrap().unwrap();
			url
		})
		.await;
	Some(url.clone())
}

/** A governance indexer of one chain, whose storage reads `node` answers. */
async fn indexer(url: &str, node: &Node) -> Indexer {
	let _ = rustls::crypto::ring::default_provider().install_default();
	std::env::set_var("DATABASE_URL", url);
	let config = ChainConfig {
		kind: ChainKind::Substrate,
		chain_id: 1000003367,
		state_machine_id: "POLKADOT-3367".into(),
		start_block: 0,
		reorg_safe_distance: 0,
		max_block_range: None,
		op_stack: false,
		contracts: Contracts::default(),
		end_block: None,
	};
	let rpc = node.clone().start().await;
	let chain = Chain {
		name: NETWORK.into(),
		config: config.clone(),
		rpc_urls: vec![rpc.clone()],
		provider: ProviderBuilder::new().connect_http(rpc.parse().unwrap()).erased(),
		substrate: None,
	};
	let http = reqwest::Client::new();
	Indexer {
		config: Config {
			environment: Environment::Mainnet,
			mode: Mode::Governance,
			chains: BTreeMap::from([(NETWORK.to_string(), config)]),
		},
		db: Arc::new(rindexer::PostgresClient::new().await.unwrap()),
		chains: BTreeMap::from([(NETWORK.to_string(), chain)]),
		orderbook: Orderbook::new(None, http.clone()),
		http,
	}
}

/** Runs the governance handler on one event at `block`, as substrate-eth-rpc logs it. */
async fn event(
	indexer: &Indexer,
	db: &Transaction<'_>,
	block: u64,
	index: u32,
	section: &str,
	method: &str,
	args: &[(&str, Value)],
) {
	let args: Vec<Value> = args
		.iter()
		.map(|(name, value)| json!({ "name": name, "value": value }))
		.collect();
	let event = Governance::Event {
		index,
		isExtrinsic: true,
		extrinsicIndex: 2,
		section: section.into(),
		method: method.into(),
		args: Value::Array(args).to_string(),
	};
	let log = LogMeta {
		address: Address::ZERO,
		block_number: block,
		block_hash: B256::repeat_byte(0x33),
		block_timestamp: 1_764_924_258,
		transaction_hash: B256::ZERO,
		transaction_index: 0,
		log_index: index as u64,
	};
	let ctx = EventContext {
		indexer,
		chain: &indexer.chains[NETWORK],
		db,
		log: &log,
		skip_non_critical: false,
	};
	governance::Event.handle(&ctx, event).await.unwrap();
}

/** Submits referendum `index` on `track` at block 100, with an electorate of 60. */
async fn submit(indexer: &Indexer, node: &Node, db: &Transaction<'_>, index: u32, track: u16) {
	node.issuance(0, 100, 40);
	node.json(referendum_key(index), 100, Some(ongoing(track, 0)));
	let args = [
		("index", json!(index)),
		("track", json!(track)),
		("proposal", json!({ "inline": "0x00" })),
	];
	event(indexer, db, 100, index, "referenda", "Submitted", &args).await;
}

async fn vote(indexer: &Indexer, db: &Transaction<'_>, block: u64, method: &str, who: &str) {
	let args = [("who", json!(who)), ("vote", json!({}))];
	event(indexer, db, block, 3, "convictionVoting", method, &args).await;
}

async fn text(db: &Transaction<'_>, sql: &str) -> Option<String> {
	db.query_opt(sql, &[]).await.unwrap().and_then(|row| row.get(0))
}

#[tokio::test]
async fn a_split_vote_is_stored_with_both_sides_and_a_tally_snapshot() {
	let Some(url) = database_url().await else { return };
	let node = Node::default();
	node.tracks(0, json!([track(1, "root")]));
	let indexer = indexer(&url, &node).await;
	let (mut client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	let db = client.transaction().await.unwrap();

	submit(&indexer, &node, &db, 1, 1).await;
	let split = json!({ "split": { "aye": 7, "nay": 3 } });
	node.json(voting_key(&alice(), 1), 101, Some(casting(json!([[1, split]]), 0)));
	node.json(referendum_key(1), 101, Some(ongoing(1, 5)));
	vote(&indexer, &db, 101, "Voted", &alice()).await;

	let vote = text(
		&db,
		&format!(
			"SELECT kind || ' ' || balance || ' ' || aye_balance || ' ' || nay_balance
			 FROM governance.governance_votes WHERE id = '1-{}'",
			alice()
		),
	)
	.await;
	assert_eq!(vote.as_deref(), Some("Split 10 7 3"));
	let call = text(
		&db,
		&format!(
			"SELECT account || ' ' || removed FROM governance.governance_vote_calls
			 WHERE id = '101-3-1-{}'",
			alice()
		),
	)
	.await;
	assert_eq!(call, Some(format!("{} false", alice())));
	let snapshot = text(
		&db,
		"SELECT ayes || ' ' || electorate FROM governance.governance_tally_snapshots
		 WHERE id = '1-101'",
	)
	.await;
	assert_eq!(snapshot.as_deref(), Some("5 60"));
	let info = text(&db, "SELECT info FROM governance.governance_referendums WHERE id = '1'").await;
	let info: Value = serde_json::from_str(&info.unwrap()).unwrap();
	assert_eq!(info, ongoing(1, 5)["ongoing"]);
}

#[tokio::test]
async fn a_vote_taken_back_goes_from_an_ongoing_referendum_but_stays_on_a_finished_one() {
	let Some(url) = database_url().await else { return };
	let node = Node::default();
	node.tracks(0, json!([track(1, "root")]));
	let indexer = indexer(&url, &node).await;
	let (mut client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	let db = client.transaction().await.unwrap();

	submit(&indexer, &node, &db, 2, 1).await;
	submit(&indexer, &node, &db, 3, 1).await;
	let votes = json!([[2, standard(true, 1, 4)], [3, standard(true, 1, 4)]]);
	node.json(voting_key(&alice(), 1), 101, Some(casting(votes, 0)));
	vote(&indexer, &db, 101, "Voted", &alice()).await;

	node.json(referendum_key(3), 102, Some(json!({ "approved": [102, null, null] })));
	event(&indexer, &db, 102, 1, "referenda", "Confirmed", &[("index", json!(3))]).await;
	node.json(voting_key(&alice(), 1), 103, Some(casting(json!([]), 0)));
	vote(&indexer, &db, 103, "VoteRemoved", &alice()).await;

	let votes = text(
		&db,
		"SELECT string_agg(referendum_index::text, ',') FROM governance.governance_votes
		 WHERE referendum_index IN (2, 3)",
	)
	.await;
	assert_eq!(votes.as_deref(), Some("3"));
	let removed = text(
		&db,
		&format!(
			"SELECT removed::text FROM governance.governance_vote_calls WHERE id = '103-3-2-{}'",
			alice()
		),
	)
	.await;
	assert_eq!(removed.as_deref(), Some("true"));
	let state = text(
		&db,
		"SELECT state || ' ' || is_final FROM governance.governance_referendums WHERE id = '3'",
	)
	.await;
	assert_eq!(state.as_deref(), Some("Confirmed true"));
}

/** A late deposit refund keeps the electorate the tally was measured against, and neither it
 * nor the metadata events change a referendum's state or its track. */
#[tokio::test]
async fn refunds_and_metadata_keep_a_referendums_state_electorate_and_track() {
	let Some(url) = database_url().await else { return };
	let node = Node::default();
	node.tracks(0, json!([track(1, "root")]));
	let indexer = indexer(&url, &node).await;
	let (mut client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	let db = client.transaction().await.unwrap();
	let state = || async {
		text(
			&db,
			"SELECT state || ' ' || electorate || ' ' || updated_block || ' ' ||
			   (track_info::jsonb->>'name')
			 FROM governance.governance_referendums WHERE id = '4'",
		)
		.await
	};

	submit(&indexer, &node, &db, 4, 1).await;
	let metadata = [("index", json!(4)), ("hash", json!(format!("0x{}", "ab".repeat(32))))];
	event(&indexer, &db, 101, 1, "referenda", "MetadataSet", &metadata).await;
	assert_eq!(state().await.as_deref(), Some("Submitted 60 101 root"));

	node.json(referendum_key(4), 110, Some(json!({ "rejected": [110, null, null] })));
	let tally = json!({ "ayes": 1, "nays": 9, "support": 1 });
	event(&indexer, &db, 110, 1, "referenda", "Rejected", &[("index", json!(4)), ("tally", tally)])
		.await;
	assert_eq!(state().await.as_deref(), Some("Rejected 60 110 root"));

	// The issuance and the tracks change before the deposit comes back and anyone clears the
	// metadata
	node.issuance(150, 1000, 40);
	node.tracks(150, json!([track(1, "renamed")]));
	let refund = [("index", json!(4)), ("who", json!(alice())), ("amount", json!(1))];
	event(&indexer, &db, 200, 1, "referenda", "DecisionDepositRefunded", &refund).await;
	assert_eq!(state().await.as_deref(), Some("Rejected 60 200 root"));
	event(&indexer, &db, 210, 1, "referenda", "MetadataCleared", &metadata).await;
	assert_eq!(state().await.as_deref(), Some("Rejected 60 210 root"));

	let events = text(
		&db,
		"SELECT string_agg(name || ' ' || changes_state, ',' ORDER BY block_number)
		 FROM governance.governance_referendum_events WHERE referendum_id = '4'",
	)
	.await;
	assert_eq!(
		events.as_deref(),
		Some(
			"Submitted true,MetadataSet false,Rejected true,DecisionDepositRefunded false,\
			 MetadataCleared false"
		)
	);
}

#[tokio::test]
async fn delegations_open_are_replaced_and_close_with_the_delegatee_refreshed() {
	let Some(url) = database_url().await else { return };
	let node = Node::default();
	node.tracks(0, json!([track(1, "root")]));
	let indexer = indexer(&url, &node).await;
	let (mut client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	let db = client.transaction().await.unwrap();
	let (alice, bob) = (alice(), bob());
	let delegation = |id: String| {
		let db = &db;
		async move {
			text(
				db,
				&format!(
					"SELECT target || ' ' || conviction || ' ' || balance || ' ' || active || ' ' ||
					   coalesce(to_block::text, '-')
					 FROM governance.governance_delegations WHERE id = '{id}'"
				),
			)
			.await
		}
	};
	let delegated_votes = || async {
		text(
			&db,
			&format!("SELECT delegated_votes::text FROM governance.governance_votes WHERE id = '5-{bob}'"),
		)
		.await
	};

	submit(&indexer, &node, &db, 5, 1).await;
	node.json(voting_key(&bob, 1), 101, Some(casting(json!([[5, standard(true, 1, 4)]]), 30)));
	node.json(voting_key(&alice, 1), 101, Some(delegating(&bob, "Locked3x", 10)));
	let delegated = [("who", json!(alice)), ("target", json!(bob))];
	event(&indexer, &db, 101, 3, "convictionVoting", "Delegated", &delegated).await;
	assert_eq!(delegation(format!("{alice}-1-101")).await, Some(format!("{bob} 3 10 true -")));
	assert_eq!(delegated_votes().await.as_deref(), Some("30"));

	node.json(voting_key(&alice, 1), 110, Some(delegating(&bob, "Locked6x", 10)));
	event(&indexer, &db, 110, 3, "convictionVoting", "Delegated", &delegated).await;
	assert_eq!(delegation(format!("{alice}-1-101")).await, Some(format!("{bob} 3 10 false 110")));
	assert_eq!(delegation(format!("{alice}-1-110")).await, Some(format!("{bob} 6 10 true -")));

	node.json(voting_key(&alice, 1), 120, None);
	node.json(voting_key(&bob, 1), 120, Some(casting(json!([[5, standard(true, 1, 4)]]), 0)));
	vote(&indexer, &db, 120, "Undelegated", &alice).await;
	assert_eq!(delegation(format!("{alice}-1-110")).await, Some(format!("{bob} 6 10 false 120")));
	assert_eq!(delegated_votes().await.as_deref(), Some("0"));
}

/** A delegation syncs the delegator and the delegatee, and each gets its own vote call when both
 * voted on the same referendum in that block. */
#[tokio::test]
async fn two_accounts_synced_by_one_event_both_get_their_vote_calls() {
	let Some(url) = database_url().await else { return };
	let node = Node::default();
	node.tracks(0, json!([track(0, "root"), track(1, "whitelisted_caller")]));
	let indexer = indexer(&url, &node).await;
	let (mut client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	let db = client.transaction().await.unwrap();
	let (alice, bob) = (alice(), bob());

	submit(&indexer, &node, &db, 6, 1).await;
	node.json(voting_key(&alice, 0), 101, Some(delegating(&bob, "Locked1x", 10)));
	node.json(voting_key(&alice, 1), 101, Some(casting(json!([[6, standard(true, 1, 4)]]), 0)));
	node.json(voting_key(&bob, 1), 101, Some(casting(json!([[6, standard(false, 2, 8)]]), 0)));
	let delegated = [("who", json!(alice)), ("target", json!(bob))];
	event(&indexer, &db, 101, 3, "convictionVoting", "Delegated", &delegated).await;

	let calls = text(
		&db,
		"SELECT string_agg(id, ',' ORDER BY id) FROM governance.governance_vote_calls
		 WHERE referendum_index = 6",
	)
	.await;
	let mut expected = [format!("101-3-6-{alice}"), format!("101-3-6-{bob}")];
	expected.sort();
	assert_eq!(calls, Some(expected.join(",")));
}

#[tokio::test]
async fn a_treasury_spend_is_followed_from_approval_to_payment() {
	let Some(url) = database_url().await else { return };
	let node = Node::default();
	let indexer = indexer(&url, &node).await;
	let (mut client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	let db = client.transaction().await.unwrap();

	let approved = [
		("index", json!(0)),
		("assetKind", json!({ "native": null })),
		("amount", json!("1000")),
		("beneficiary", json!({ "accountId32": alice() })),
		("validFrom", json!(10)),
		("expireAt", json!(20)),
	];
	event(&indexer, &db, 100, 3, "treasury", "AssetSpendApproved", &approved).await;
	let paid = [("index", json!(0)), ("paymentId", json!(7))];
	event(&indexer, &db, 105, 3, "treasury", "Paid", &paid).await;

	let spend = text(
		&db,
		"SELECT amount || ' ' || state || ' ' || payment_id || ' ' || approved_block || ' ' ||
		   updated_block FROM governance.governance_treasury_spends WHERE id = '0'",
	)
	.await;
	assert_eq!(spend.as_deref(), Some("1000 Paid 7 100 105"));
}

#[tokio::test]
async fn a_motion_proposed_voted_and_closed_in_one_block_keeps_its_votes_and_call() {
	let Some(url) = database_url().await else { return };
	let node = Node::default();
	let indexer = indexer(&url, &node).await;
	let (mut client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	let db = client.transaction().await.unwrap();

	// Closing in the same block leaves neither `ProposalOf` nor `Voting` in storage, so the call
	// comes from the extrinsic and the votes from the events
	node.constant("TechnicalCollective", "MotionDuration", 200, json!(100));
	let propose = json!({ "section": "technicalCollective", "method": "propose", "args": [
		{ "name": "threshold", "value": 2 },
		{ "name": "proposal", "value": { "section": "whitelist", "method": "whitelistCall", "args": [] } },
	]});
	node.extrinsic(
		2,
		200,
		json!({ "section": "utility", "method": "batchAll", "args": [
			{ "name": "calls", "value": [propose] },
		]}),
	);
	let hash = json!(format!("0x{}", "77".repeat(32)));
	let voted = |who: String, aye: bool| {
		[("account", json!(who)), ("proposalHash", hash.clone()), ("voted", json!(aye))]
	};
	let proposed = [
		("account", json!(alice())),
		("proposalIndex", json!(7)),
		("proposalHash", hash.clone()),
		("threshold", json!(2)),
	];
	event(&indexer, &db, 200, 1, "technicalCollective", "Proposed", &proposed).await;
	event(&indexer, &db, 200, 2, "technicalCollective", "Voted", &voted(alice(), true)).await;
	event(&indexer, &db, 200, 3, "technicalCollective", "Voted", &voted(bob(), false)).await;
	event(&indexer, &db, 200, 4, "technicalCollective", "Voted", &voted(bob(), true)).await;
	let closed = [("proposalHash", hash.clone())];
	event(&indexer, &db, 200, 5, "technicalCollective", "Closed", &closed).await;
	event(&indexer, &db, 200, 6, "technicalCollective", "Approved", &closed).await;
	let executed = [("proposalHash", hash.clone()), ("result", json!({ "ok": null }))];
	event(&indexer, &db, 200, 7, "technicalCollective", "Executed", &executed).await;

	let motion = text(
		&db,
		"SELECT ayes::text || ' ' || nays::text || ' ' || \"end\" || ' ' || state || ' ' || is_final
		   || ' ' || (call::jsonb ->> 'method') FROM governance.governance_motions WHERE id = '7'",
	)
	.await;
	let expected = format!("[\"{}\", \"{}\"] [] 300 Executed true whitelistCall", alice(), bob());
	assert_eq!(motion.as_deref(), Some(expected.as_str()));
}

#[tokio::test]
async fn a_preimage_noted_again_and_gone_by_the_end_of_its_block_keeps_its_bytes() {
	let Some(url) = database_url().await else { return };
	let node = Node::default();
	let indexer = indexer(&url, &node).await;
	let (mut client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	let db = client.transaction().await.unwrap();

	let hash = [0x55u8; 32];
	let status = [key("Preimage", "RequestStatusFor"), hash.to_vec()].concat();
	node.json(status.clone(), 300, Some(json!({ "unrequested": { "len": 3 } })));
	let bytes = [key("Preimage", "PreimageFor"), (hash, 3u32).encode()].concat();
	node.storage(bytes.clone(), 300, vec![1u8, 2, 3].encode());
	let noted = [("hash", json!(format!("0x{}", hex::encode(hash))))];
	event(&indexer, &db, 300, 1, "preimage", "Noted", &noted).await;

	// Noted and cleared again within block 310, so its storage is empty at the end of it
	node.json(status, 310, None);
	node.storage(bytes, 310, Vec::new());
	event(&indexer, &db, 310, 1, "preimage", "Noted", &noted).await;

	let preimage = text(
		&db,
		"SELECT length || ' ' || data || ' ' || noted_block FROM governance.governance_preimages",
	)
	.await;
	assert_eq!(preimage.as_deref(), Some("3 0x010203 310"));
}
