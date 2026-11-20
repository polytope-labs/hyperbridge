//! Runs the API against a real Postgres loaded with indexed Nexus data. It needs a database it may
//! wipe, so it only runs with `TEST_DATABASE_URL` set:
//!
//!   TEST_DATABASE_URL=postgres://user:pass@localhost/test cargo test -p governance-api --
//! --ignored

use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::{Method, StatusCode};
use serde_json::{json, Value};
use sp_core::{
	crypto::{AccountId32, Ss58AddressFormat, Ss58Codec},
	sr25519, Pair,
};
use tokio::sync::Semaphore;
use tokio_postgres::{Client, NoTls};

use super::{router, AppState};
use crate::{crypto::token_hash, db, limits::RateLimiter};

const REFERENDUM: i32 = 80;
const MOTION: i32 = 65;

fn address(pair: &sr25519::Pair) -> String {
	AccountId32::from(pair.public().0).to_ss58check_with_version(Ss58AddressFormat::custom(0))
}

/// The same account in the generic format many wallets show.
fn generic(pair: &sr25519::Pair) -> String {
	AccountId32::from(pair.public().0).to_ss58check_with_version(Ss58AddressFormat::custom(42))
}

/// A signature as a browser wallet makes it, over the message wrapped in `<Bytes>` tags.
fn sign(pair: &sr25519::Pair, message: &str) -> String {
	format!("0x{}", hex::encode(pair.sign(format!("<Bytes>{message}</Bytes>").as_bytes()).0))
}

struct Visitor {
	http: reqwest::Client,
	base: String,
	cookie: Option<String>,
	forwarded_for: &'static str,
}

impl Visitor {
	fn new(base: &str) -> Self {
		Self {
			http: reqwest::Client::new(),
			base: base.into(),
			cookie: None,
			forwarded_for: "198.51.100.1",
		}
	}

	async fn call(
		&self,
		method: Method,
		path: &str,
		body: Option<Value>,
	) -> anyhow::Result<(StatusCode, Value)> {
		let mut request = self
			.http
			.request(method, format!("{}/{path}", self.base))
			.header("x-forwarded-for", self.forwarded_for);
		if let Some(cookie) = &self.cookie {
			request = request.header("cookie", cookie);
		}
		if let Some(body) = body {
			request = request.json(&body);
		}
		let response = request.send().await?;
		let status = response.status();
		Ok((status, response.json().await.unwrap_or(Value::Null)))
	}

	async fn get(&self, path: &str) -> anyhow::Result<Value> {
		let (status, body) = self.call(Method::GET, path, None).await?;
		anyhow::ensure!(status == StatusCode::OK, "GET {path} answered {status}: {body}");
		Ok(body)
	}

	/// Logs in with a wallet signature, the way the frontend does.
	async fn login(base: &str, pair: &sr25519::Pair) -> anyhow::Result<Self> {
		Self::login_as(base, pair, &address(pair)).await
	}

	/// Logs in giving the account in the format `account` is written in.
	async fn login_as(base: &str, pair: &sr25519::Pair, account: &str) -> anyhow::Result<Self> {
		let mut visitor = Self::new(base);
		let challenge = visitor.get(&format!("auth/login/{account}")).await?;
		let response = visitor
			.http
			.post(format!(
				"{base}/auth/login/{}",
				challenge["attemptId"].as_str().unwrap_or_default()
			))
			.json(
				&json!({ "challengeAnswer": sign(pair, challenge["challenge"].as_str().unwrap_or_default()) }),
			)
			.send()
			.await?;
		anyhow::ensure!(
			response.status() == StatusCode::OK,
			"login answered {}",
			response.status()
		);
		let cookie =
			response.headers().get("set-cookie").context("login sets a cookie")?.to_str()?;
		visitor.cookie = cookie.split(';').next().map(str::to_owned);
		Ok(visitor)
	}

	/// Sends a change signed with the wallet instead of a session.
	async fn signed(
		&self,
		path: &str,
		signer: &sr25519::Pair,
		entity: Value,
		address: String,
	) -> anyhow::Result<StatusCode> {
		self.signed_with(Method::POST, path, signer, entity, address).await
	}

	async fn signed_with(
		&self,
		method: Method,
		path: &str,
		signer: &sr25519::Pair,
		entity: Value,
		address: String,
	) -> anyhow::Result<StatusCode> {
		let body = json!({
			"entity": entity,
			"address": address,
			"signature": sign(signer, &entity.to_string()),
			"signerWallet": "polkadot-js",
		});
		Ok(self.call(method, path, Some(body)).await?.0)
	}
}

const METADATA: &str = "app.\"_metadata_0x1867da0954c96ede\"";
/// A voter in the fixture, and the account delegating to it.
const VOTER: &str = "12ve5fWBPUnhqZFvZtzg5QVND5dBkdWQfC5boWywnsob5PMH";
const DELEGATOR: &str = "1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM";
const INDEXED_HEIGHT: i64 = 12337798;

/// Creates the governance tables from omniplex's migration and loads the fixture into them, with
/// the indexer's metadata table set aside so the API
/// starts before omniplex has created it, and serves the API on a free port. The test reaches it
/// from 127.0.0.1, which is trusted as a proxy so each visitor can name its own address. Returns
/// the API's address and a connection to the database.
async fn serve(admins: Vec<String>) -> anyhow::Result<(String, Client)> {
	let url = env::var("TEST_DATABASE_URL").context("TEST_DATABASE_URL is not set")?;
	let (client, connection) = tokio_postgres::connect(&url, NoTls).await?;
	tokio::spawn(connection);
	client
		.batch_execute(
			"DROP SCHEMA IF EXISTS forum CASCADE; DROP SCHEMA IF EXISTS governance CASCADE;
			DROP SCHEMA IF EXISTS app CASCADE;",
		)
		.await?;
	client
		.batch_execute(include_str!("../../../../omniplex/migrations/0004_governance.sql"))
		.await?;
	client.batch_execute(include_str!("../../tests/governance.sql")).await?;
	client
		.batch_execute(&format!("ALTER TABLE {METADATA} RENAME TO not_yet_created"))
		.await?;

	let state = AppState {
		// One stream at a time, so the cap is easy to reach
		streams: Arc::new(Semaphore::new(1)),
		..AppState::new(
			db::pool(&url).await?,
			admins.into_iter().collect(),
			RateLimiter::new(RateLimiter::trusted_proxies("127.0.0.1")?),
			"forum.test",
		)
		.await?
	};
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
	let base = format!("http://{}", listener.local_addr()?);
	let app = router(state).into_make_service_with_connect_info::<SocketAddr>();
	tokio::spawn(async move { axum::serve(listener, app).await });
	Ok((base, client))
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a database it may wipe"]
async fn serves_the_forum() -> anyhow::Result<()> {
	let alice = sr25519::Pair::from_string("//Alice", None)?;
	let bob = sr25519::Pair::from_string("//Bob", None)?;
	let (base, db) = serve(vec![address(&alice)]).await?;
	let anyone = Visitor::new(&base);

	// The API starts before omniplex has created its metadata table, and finds it once it exists
	assert_eq!(anyone.get("inspect/scan-height").await?["value"], 0);
	let mut stream = anyone.http.get(format!("{base}/stream/scan-height")).send().await?;
	let first = stream.chunk().await?.context("the stream sends the height straight away")?;
	assert_eq!(serde_json::from_slice::<Value>(&first)?["value"], 0);
	let (status, _) = anyone.call(Method::GET, "stream/scan-height", None).await?;
	assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "open streams are capped");
	db.batch_execute(&format!("ALTER TABLE app.not_yet_created RENAME TO {}", &METADATA[4..]))
		.await?;
	let changed = tokio::time::timeout(Duration::from_secs(10), stream.chunk()).await??;
	let changed = changed.context("the stream follows the height")?;
	assert_eq!(serde_json::from_slice::<Value>(&changed)?["value"], INDEXED_HEIGHT);
	assert_eq!(anyone.get("inspect/scan-height").await?["value"], INDEXED_HEIGHT);
	drop(stream);

	// Indexed data comes back in the frontend's shapes, and unused filters arrive empty
	let list = anyone
		.get("gov2/referendums?page=1&page_size=25&status=&ongoing=&is_treasury=&sort=")
		.await?;
	assert_eq!(list["total"], 5);
	let referendum = anyone.get(&format!("gov2/referendums/{REFERENDUM}")).await?;
	assert_eq!(referendum["state"]["name"], "Executed");
	assert_eq!(referendum["trackInfo"]["origin"], "WhitelistedCaller");
	assert_eq!(referendum["trackInfo"]["decisionDeposit"], "400000000000000000");
	let timeline = &referendum["onchainData"]["timeline"];
	assert_eq!(timeline[0]["args"]["referendumIndex"], REFERENDUM);
	assert_eq!(timeline[1]["name"], "DecisionDepositPlaced");
	assert_eq!(timeline[1]["args"]["amount"], "400000000000000000", "decimal strings stay decimal");

	// Votes and delegations decode from the shapes omniplex writes. A finished referendum counts
	// the delegations in force when it was decided, though they have ended since
	let votes = anyone.get(&format!("gov2/referenda/{REFERENDUM}/votes")).await?;
	let stats = anyone.get(&format!("gov2/referendums/{REFERENDUM}/vote-stats")).await?;
	assert_eq!(votes.as_array().map(Vec::len), Some(3));
	assert_eq!(votes[0]["balance"], "200000000000000000000");
	assert_eq!(votes[0]["votes"], "200000000000000000000");
	assert_eq!(votes[2]["isDelegating"], true);
	assert_eq!(votes[2]["votes"], "20000000000000000000", "conviction 2 doubles the balance");
	assert_eq!(stats["directAddresses"], 2);
	assert_eq!(stats["delegationAddresses"], 1);
	assert_eq!(stats["delegationCapital"], "10000000000000000000");
	assert_eq!(stats["totalCapital"], "359483333333333343398");
	let calls = anyone.get(&format!("gov2/referendums/{REFERENDUM}/vote-calls")).await?;
	assert_eq!(
		calls[0]["vote"],
		json!({ "balance": "149483333333333343398", "vote": { "isAye": true, "conviction": 1 } })
	);
	let delegated = anyone
		.get(&format!("gov2/referendums/{REFERENDUM}/delegation-votes?target={VOTER}"))
		.await?;
	assert_eq!(delegated.as_array().map(Vec::len), Some(1));
	assert_eq!(delegated[0]["account"], DELEGATOR);

	// An ongoing referendum counts the delegations in force now, even one made after the height the
	// indexer last recorded, and not one that has ended
	let ongoing = anyone.get("gov2/referenda/83/votes").await?;
	assert_eq!(ongoing.as_array().map(Vec::len), Some(2));
	assert_eq!(ongoing[1]["votes"], "500000000000000000", "no conviction is a tenth");
	assert_eq!(ongoing[1]["queryAt"], INDEXED_HEIGHT);

	// The state is the last event that changes it: clearing a killed referendum's metadata leaves
	// it killed, and a confirmed one is shown and found as approved
	let killed = anyone.get("gov2/referendums/81").await?;
	assert_eq!(killed["state"]["name"], "Killed");
	let names = killed["onchainData"]["timeline"]
		.as_array()
		.into_iter()
		.flatten()
		.map(|event| event["name"].clone())
		.collect::<Vec<_>>();
	assert_eq!(names, [json!("Submitted"), json!("Killed")]);
	let status = |status: &'static str| {
		let anyone = &anyone;
		async move {
			let list = anyone.get(&format!("gov2/referendums?status={status}")).await?;
			anyhow::Ok(
				list["items"]
					.as_array()
					.into_iter()
					.flatten()
					.map(|item| item["referendumIndex"].clone())
					.collect::<Vec<_>>(),
			)
		}
	};
	assert_eq!(status("Killed").await?, [json!(81)]);
	assert_eq!(status("Approved").await?, [json!(82)]);
	assert_eq!(status("Deciding").await?, [json!(83)]);

	// A voter's lists come back a page at a time with each referendum's title and state
	let voted = anyone
		.get(&format!("users/{VOTER}/referenda/votes?includes_title=true&page_size=2"))
		.await?;
	assert_eq!(voted["total"], 3);
	assert_eq!(voted["items"][0]["referendumIndex"], 83);
	assert_eq!(voted["items"][0]["proposal"]["state"]["name"], "DecisionStarted");
	assert_eq!(voted["items"][1]["proposal"]["state"]["name"], "Executed");
	let voted = anyone
		.get(&format!("users/{VOTER}/referenda/vote-calls?includes_title=true&page=2&page_size=1"))
		.await?;
	assert_eq!(voted["total"], 2);
	assert_eq!(voted["items"][0]["referendumIndex"], 79);
	assert_eq!(voted["items"][0]["vote"]["vote"]["isAye"], true);
	assert_eq!(voted["items"][0]["proposal"]["state"]["name"], "Executed");
	let rate = anyone.get(&format!("users/{VOTER}/referenda/vote-stats")).await?;
	assert_eq!(rate["participationRate"], 0.4);

	// Statistics read every referendum at once
	assert_eq!(anyone.get("referenda/summary").await?["totalReferendaCount"], 5);
	let turnout = anyone.get("referenda/tracks/1/turnout").await?;
	assert_eq!(turnout.as_array().map(Vec::len), Some(5));
	assert_eq!(turnout[1]["delegationCapital"], "10000000000000000000");
	let drill = anyone.get(&format!("referenda/delegatee/{VOTER}/delegators")).await?;
	assert_eq!(drill["total"], 1);
	assert_eq!(drill["items"][0]["account"], DELEGATOR);
	assert_eq!(drill["items"][0]["trackName"], "whitelisted_caller");
	let on_track = anyone
		.get(&format!(
			"referenda/tracks/1/delegatee/{VOTER}/delegators?sort=%5B%22balance%22%2C%22desc%22%5D"
		))
		.await?;
	assert_eq!(on_track["items"][0]["votes"], "500000000000000000");
	assert_eq!(anyone.get("referenda/tracks/2/delegators").await?["total"], 0);

	// A paid spend is open until its payment is checked, an approved one until it expires
	let open = anyone.get("overview/treasury-spends").await?;
	let open = open["items"]
		.as_array()
		.into_iter()
		.flatten()
		.map(|spend| spend["index"].clone());
	assert_eq!(open.collect::<Vec<_>>(), [json!(2), json!(0)]);
	assert_eq!(anyone.get("treasury/spends/1").await?["isFinal"], true);
	assert_eq!(anyone.get("treasury/spends/0").await?["isFinal"], false);
	assert_eq!(anyone.get("overview/summary").await?["treasurySpends"]["active"], 2);

	// Routes the frontend calls that have nothing to show on Nexus
	assert_eq!(anyone.get(&format!("treasury/addresses/{VOTER}")).await?, json!({}));
	assert_eq!(
		anyone
			.call(Method::POST, "client-errors", Some(json!({ "message": "x" })))
			.await?
			.0,
		StatusCode::OK
	);
	assert!(!anyone
		.get(&format!("gov2/referenda/{REFERENDUM}/tally-history"))
		.await?
		.as_array()
		.unwrap_or(&vec![])
		.is_empty());
	assert_eq!(anyone.get(&format!("tech-comm/motions/{MOTION}")).await?["state"], "Executed");
	let preimage = referendum["onchainData"]["proposalHash"]
		.as_str()
		.unwrap_or_default()
		.to_owned();
	assert!(anyone.get(&format!("preimages/{preimage}")).await?["hex"]
		.as_str()
		.is_some_and(|hex| hex.starts_with("0x5302")));

	// Wallet login: a challenge can be answered once and only by its account
	let as_alice = Visitor::login(&base, &alice).await?;
	assert_eq!(as_alice.get("user/status").await?["isLoggedIn"], true);
	let challenge = anyone.get(&format!("auth/login/{}", address(&alice))).await?;
	let forged = json!({ "challengeAnswer": sign(&bob, challenge["challenge"].as_str().unwrap_or_default()) });
	let path = format!("auth/login/{}", challenge["attemptId"].as_str().unwrap_or_default());
	assert_eq!(anyone.call(Method::POST, &path, Some(forged)).await?.0, StatusCode::BAD_REQUEST);
	assert_eq!(anyone.call(Method::GET, "user/profile", None).await?.0, StatusCode::UNAUTHORIZED);
	let as_bob = Visitor::login(&base, &bob).await?;

	// The challenge is a readable message naming the forum, what it is for and when it was asked
	// for, with the account in the chain's format whatever format the wallet gave
	let challenger = &Visitor { forwarded_for: "198.51.100.2", ..Visitor::new(&base) };
	let message = challenger.get(&format!("auth/login/{}", generic(&alice))).await?["challenge"]
		.as_str()
		.unwrap_or_default()
		.to_owned();
	assert!(message.starts_with("forum.test wants you to sign in with your account:\n"));
	assert!(message.contains(&address(&alice)) && !message.contains(&generic(&alice)));
	assert!(message.contains("Purpose: Sign in") && message.contains("Nonce: "));
	assert!(message.contains("Issued at: "));
	let as_generic = Visitor::login_as(&base, &alice, &generic(&alice)).await?;
	assert_eq!(as_generic.get("user/profile").await?["address"], address(&alice));

	// Asking for challenges never cancels one someone else is answering, and clears expired ones
	db.execute(
		"INSERT INTO forum.login_attempts (id, address, challenge, created_at)
		VALUES ('expired', $1, '', NOW() - INTERVAL '11 minutes')",
		&[&address(&bob)],
	)
	.await?;
	let challenge_bob = format!("auth/login/{}", address(&bob));
	let attempts = stream::iter(0..4)
		.then(|_| challenger.get(&challenge_bob))
		.try_collect::<Vec<_>>()
		.await?;
	let expired = db
		.query_opt("SELECT 1 FROM forum.login_attempts WHERE id = 'expired'", &[])
		.await?;
	assert!(expired.is_none(), "expired challenges are cleared");
	let answers = stream::iter(&attempts)
		.then(|attempt| {
			let answer = json!({ "challengeAnswer": sign(&bob, attempt["challenge"].as_str().unwrap_or_default()) });
			let path = format!("auth/login/{}", attempt["attemptId"].as_str().unwrap_or_default());
			async move { challenger.call(Method::POST, &path, Some(answer)).await.map(|(status, _)| status) }
		})
		.try_collect::<Vec<_>>()
		.await?;
	assert_eq!(answers, [StatusCode::OK; 4], "every challenge can still be answered");

	// Sessions are stored by the hash of their token
	let token = as_bob
		.cookie
		.as_deref()
		.and_then(|cookie| cookie.strip_prefix("auth-token="))
		.context("bob has a session cookie")?;
	let db = &db;
	let stored = |hash: String| async move {
		db.query_one("SELECT count(*) FROM forum.sessions WHERE token_hash = $1", &[&hash])
			.await
			.map(|row| row.get::<_, i64>(0))
	};
	assert_eq!(stored(token.to_owned()).await?, 0);
	assert_eq!(stored(token_hash(token)).await?, 1);

	// Discussions, comments, replies and reactions
	let (_, number) = as_alice
		.call(Method::POST, "posts", Some(json!({ "title": "Hello", "content": "First post" })))
		.await?;
	let post = anyone.get(&format!("posts/{number}")).await?;
	let post_id = post["_id"].as_str().unwrap_or_default().to_owned();
	let (_, comment) = as_bob
		.call(
			Method::POST,
			&format!("posts/{post_id}/comments"),
			Some(json!({ "content": "Nice" })),
		)
		.await?;
	let comment_id = comment["_id"].as_str().unwrap_or_default().to_owned();
	let reply_path = format!("posts/{post_id}/comments/{comment_id}/replies");
	let (status, reply) = as_alice
		.call(Method::POST, &reply_path, Some(json!({ "content": "Thanks" })))
		.await?;
	assert_eq!(status, StatusCode::OK);
	let nested =
		format!("posts/{post_id}/comments/{}/replies", reply["_id"].as_str().unwrap_or_default());
	assert_eq!(
		as_bob.call(Method::POST, &nested, Some(json!({ "content": "x" }))).await?.0,
		StatusCode::NOT_FOUND
	);
	assert_eq!(
		as_bob
			.call(Method::PUT, &format!("posts/{post_id}/reaction"), Some(json!({ "reaction": 1 })))
			.await?
			.0,
		StatusCode::OK
	);
	assert_eq!(
		anyone.get(&format!("posts/{number}")).await?["reactions"]
			.as_array()
			.map(Vec::len),
		Some(1)
	);

	// Only authors edit their comments, and a deleted comment with replies stays as a placeholder
	let edit = format!("comments/{}", reply["_id"].as_str().unwrap_or_default());
	assert_eq!(
		as_bob.call(Method::PATCH, &edit, Some(json!({ "content": "x" }))).await?.0,
		StatusCode::FORBIDDEN
	);
	assert_eq!(
		as_bob.call(Method::DELETE, &format!("comments/{comment_id}"), None).await?.0,
		StatusCode::OK
	);
	let thread = anyone.get(&format!("posts/{post_id}/comments")).await?;
	assert_eq!(thread["items"][0]["content"], "_This comment was deleted._");
	assert_eq!(thread["items"][0]["replies"].as_array().map(Vec::len), Some(1));

	// Titles of on-chain items belong to their proposer, or an admin
	let item = referendum["_id"].as_str().unwrap_or_default().to_owned();
	let title = json!({ "title": "Upgrade the Intent Gateway", "content": "Details" });
	assert_eq!(
		as_bob
			.call(Method::PATCH, &format!("gov2/referendums/{item}"), Some(title.clone()))
			.await?
			.0,
		StatusCode::FORBIDDEN
	);
	assert_eq!(
		as_alice
			.call(Method::PATCH, &format!("gov2/referendums/{item}"), Some(title))
			.await?
			.0,
		StatusCode::OK
	);
	assert_eq!(
		anyone.get(&format!("gov2/referendums/{REFERENDUM}")).await?["title"],
		"Upgrade the Intent Gateway"
	);

	// Wallet signed profile edits, checked for signer, action, age and proxies
	let now = chrono::Utc::now().timestamp_millis();
	let bio = |bio: &str, timestamp: i64| json!({ "action": "set-profile-bio", "bio": bio, "timestamp": timestamp });
	assert_eq!(
		anyone.signed("user/bio", &alice, bio("Hello", now), address(&alice)).await?,
		StatusCode::OK
	);
	assert_eq!(anyone.get(&format!("users/{}", address(&alice))).await?["bio"], "Hello");
	assert_eq!(
		anyone.signed("user/bio", &bob, bio("x", now), address(&alice)).await?,
		StatusCode::BAD_REQUEST
	);
	assert_eq!(
		anyone
			.signed("user/bio", &alice, bio("x", now - 11 * 60 * 1000), address(&alice))
			.await?,
		StatusCode::BAD_REQUEST
	);
	let avatar = json!({ "action": "set-avatar", "CID": "x", "timestamp": now });
	assert_eq!(
		anyone.signed("user/bio", &alice, avatar, address(&alice)).await?,
		StatusCode::BAD_REQUEST
	);
	let proxied =
		json!({ "action": "set-profile-bio", "bio": "x", "timestamp": now, "real": address(&bob) });
	assert_eq!(
		anyone.signed("user/bio", &alice, proxied, address(&alice)).await?,
		StatusCode::BAD_REQUEST
	);

	// A signed request is acted on once, may be a little ahead of the server's clock but not more,
	// and counts for the account whatever address format it names
	let first = bio("Once", now + 1);
	assert_eq!(
		anyone.signed("user/bio", &alice, first.clone(), generic(&alice)).await?,
		StatusCode::OK
	);
	assert_eq!(
		anyone.signed("user/bio", &alice, first, generic(&alice)).await?,
		StatusCode::BAD_REQUEST,
		"replayed"
	);
	assert_eq!(anyone.get(&format!("users/{}", generic(&alice))).await?["bio"], "Once");
	assert_eq!(
		anyone.get(&format!("users/{}", generic(&alice))).await?["address"],
		address(&alice)
	);
	assert_eq!(
		anyone
			.signed("user/bio", &alice, bio("x", now + 30 * 1000), address(&alice))
			.await?,
		StatusCode::OK
	);
	assert_eq!(
		anyone
			.signed("user/bio", &alice, bio("x", now + 5 * 60 * 1000), address(&alice))
			.await?,
		StatusCode::BAD_REQUEST,
		"too far ahead"
	);
	assert_eq!(
		anyone.call(Method::GET, "users/not-an-address", None).await?.0,
		StatusCode::BAD_REQUEST
	);

	// Delegate announcements, only from accounts seen voting or delegated to
	let announcement = |timestamp: i64| {
		json!({
			"action": "set-delegation-announcement",
			"shortDescription": "I vote often",
			"longDescription": "",
			"timestamp": timestamp,
		})
	};
	assert_eq!(
		anyone
			.signed("delegation/announcements", &alice, announcement(now), address(&alice))
			.await?,
		StatusCode::BAD_REQUEST
	);
	db.execute(
		"INSERT INTO governance.governance_delegations VALUES ('to-alice', $1, $2, 0, 1, 1, true, 12337000, NULL)",
		&[&DELEGATOR, &address(&alice)],
	)
	.await?;
	assert_eq!(
		anyone
			.signed("delegation/announcements", &alice, announcement(now + 2), address(&alice))
			.await?,
		StatusCode::OK
	);
	let delegate = anyone
		.get(&format!("delegation/referenda/delegates/{}", address(&alice)))
		.await?;
	assert_eq!(delegate["manifesto"]["shortDescription"], "I vote often");
	assert_eq!(delegate["delegatorsCount"], 1);
	let delegates = anyone.get("delegation/referenda/delegates?page_size=1").await?;
	assert_eq!(delegates["total"], 2);
	let first = delegates["items"][0]["address"].as_str().unwrap_or_default().to_owned();
	assert_eq!(first, address(&alice).min(VOTER.to_owned()), "ties go in address order");
	let voter = anyone.get(&format!("delegation/referenda/delegates/{VOTER}")).await?;
	assert_eq!(
		(voter["participationRate"].clone(), voter["manifesto"].clone()),
		(json!(0.4), Value::Null)
	);
	let announced = format!("delegation/announcements/{}", address(&alice));
	assert_eq!(as_bob.call(Method::DELETE, &announced, None).await?.0, StatusCode::FORBIDDEN);
	assert_eq!(as_alice.call(Method::DELETE, &announced, None).await?.0, StatusCode::OK);
	let delegate = anyone
		.get(&format!("delegation/referenda/delegates/{}", address(&alice)))
		.await?;
	assert_eq!(delegate["manifesto"], Value::Null, "admins remove announcements");

	// Appendants belong to the referendum's proposer, or an admin
	let appendants = format!("gov2/referenda/{REFERENDUM}/appendants");
	assert_eq!(
		as_bob.call(Method::POST, &appendants, Some(json!({ "content": "x" }))).await?.0,
		StatusCode::FORBIDDEN
	);
	assert_eq!(
		as_alice
			.call(Method::POST, &appendants, Some(json!({ "content": "Done" })))
			.await?
			.0,
		StatusCode::OK
	);
	assert_eq!(anyone.get(&appendants).await?[0]["content"], "Done");

	// Admins mark spam, and an account's comments are listed with the post they were made on
	let (_, spam) = as_bob
		.call(Method::POST, &format!("posts/{post_id}/comments"), Some(json!({ "content": "Buy" })))
		.await?;
	let spam_path = format!("comments/{}/spam", spam["_id"].as_str().unwrap_or_default());
	assert_eq!(as_bob.call(Method::POST, &spam_path, None).await?.0, StatusCode::FORBIDDEN);
	assert_eq!(as_alice.call(Method::POST, &spam_path, None).await?.0, StatusCode::OK);
	let mine = anyone
		.get(&format!("users/{}/comments?page=1&page_size=10", address(&bob)))
		.await?;
	assert_eq!(mine["total"], 1, "deleted comments are left out");
	assert_eq!(mine["items"][0]["spam"], true);
	assert_eq!(mine["items"][0]["post"]["title"], "Hello");

	// A referendum's comment filter is chosen by its proposer or an admin
	let filter = format!("gov2/referendums/{REFERENDUM}/comment-filter-setting");
	let hide = json!({ "filter": { "hide_0": true } });
	assert_eq!(
		as_bob.call(Method::PUT, &filter, Some(hide.clone())).await?.0,
		StatusCode::FORBIDDEN
	);
	assert_eq!(as_alice.call(Method::PUT, &filter, Some(hide)).await?.0, StatusCode::OK);
	let detail = anyone.get(&format!("gov2/referendums/{REFERENDUM}")).await?;
	assert_eq!(detail["commentFilterSetting"]["hide_0"], true);
	assert_eq!(detail["title"], "Upgrade the Intent Gateway", "the filter leaves the title alone");

	let report = json!({ "postType": "post", "postId": post_id, "reason": "Spam" });
	assert_eq!(as_bob.call(Method::POST, "report", Some(report.clone())).await?.0, StatusCode::OK);
	assert_eq!(
		anyone.call(Method::POST, "report", Some(report)).await?.0,
		StatusCode::UNAUTHORIZED
	);

	// Saved multisigs, stored under the address their signatories and threshold control
	let multisigs = format!("users/{}/multisigs", address(&alice));
	let signatories = json!([address(&alice), address(&bob)]);
	let add = |threshold: u64| {
		json!({ "action": "add-multisig", "signatories": signatories, "threshold": threshold,
			"name": "Council", "timestamp": now })
	};
	assert_eq!(
		anyone.signed(&multisigs, &alice, add(3), address(&alice)).await?,
		StatusCode::BAD_REQUEST
	);
	assert_eq!(
		anyone.signed(&multisigs, &bob, add(2), address(&bob)).await?,
		StatusCode::FORBIDDEN
	);
	assert_eq!(anyone.signed(&multisigs, &alice, add(2), address(&alice)).await?, StatusCode::OK);
	let saved = anyone.get(&format!("users/{}/multisigs", generic(&alice))).await?;
	let multisig = saved[0]["multisigAddress"].as_str().unwrap_or_default().to_owned();
	assert_eq!(saved[0]["threshold"], 2);
	let rename = json!({ "action": "update-multisig", "multisigAddress": multisig, "name": "Board",
		"timestamp": now });
	let path = format!("{multisigs}/{multisig}");
	let renamed = anyone
		.signed_with(Method::PATCH, &path, &alice, rename, address(&alice))
		.await?;
	assert_eq!(renamed, StatusCode::OK);
	assert_eq!(anyone.get(&multisigs).await?[0]["name"], "Board");
	let remove =
		json!({ "action": "remove-multisig", "multisigAddress": multisig, "timestamp": now });
	let path = format!("{multisigs}/{multisig}/delete");
	assert_eq!(anyone.signed(&path, &alice, remove, address(&alice)).await?, StatusCode::OK);
	assert_eq!(anyone.get(&multisigs).await?.as_array().map(Vec::len), Some(0));

	// The header search finds referenda by index and by their forum title
	let found = anyone.get(&format!("search?text={REFERENDUM}")).await?;
	assert_eq!(found["openGovReferenda"][0]["referendumIndex"], REFERENDUM);
	let found = anyone.get("search?text=intent%20gateway").await?;
	assert_eq!(found["openGovReferenda"][0]["title"], "Upgrade the Intent Gateway");
	assert_eq!(anyone.get("search?text=100%25").await?["openGovReferenda"], json!([]));

	// Reactions and reports need a target that exists, and nobody reacts to their own comment
	let missing = crate::primitives::post_id(crate::primitives::PostKind::Discussion, 999);
	let like = Some(json!({ "reaction": 1 }));
	assert_eq!(
		as_bob
			.call(Method::PUT, &format!("posts/{missing}/reaction"), like.clone())
			.await?
			.0,
		StatusCode::NOT_FOUND
	);
	let own = format!("comments/{}/reaction", spam["_id"].as_str().unwrap_or_default());
	assert_eq!(as_bob.call(Method::PUT, &own, like.clone()).await?.0, StatusCode::BAD_REQUEST);
	assert_eq!(as_alice.call(Method::PUT, &own, like).await?.0, StatusCode::OK);
	let report = |post_type: &str, post_id: &str| {
		Some(json!({ "postType": post_type, "postId": post_id, "reason": "Spam" }))
	};
	assert_eq!(
		as_bob.call(Method::POST, "report", report("anything", &post_id)).await?.0,
		StatusCode::BAD_REQUEST
	);
	assert_eq!(
		as_bob.call(Method::POST, "report", report("post", &missing)).await?.0,
		StatusCode::NOT_FOUND
	);

	// A multisig's signatories are counted once each, whatever format names them
	let repeated = json!({ "action": "add-multisig", "signatories": [address(&alice), generic(&alice)],
		"threshold": 2, "name": "Twice", "timestamp": now + 3 });
	assert_eq!(
		anyone.signed(&multisigs, &alice, repeated, address(&alice)).await?,
		StatusCode::BAD_REQUEST
	);

	// A thread takes a bounded number of replies
	let (_, root) = as_alice
		.call(
			Method::POST,
			&format!("posts/{post_id}/comments"),
			Some(json!({ "content": "Busy" })),
		)
		.await?;
	let root = root["_id"].as_str().unwrap_or_default().to_owned();
	let root_id = crate::primitives::parse_comment_id(&root).context("a comment id")?;
	db.execute(
		"INSERT INTO forum.comments (post_kind, post_number, reply_to, author, content)
		SELECT post_kind, post_number, id, author, 'Me too' FROM forum.comments, generate_series(1, 200)
		WHERE id = $1",
		&[&root_id],
	)
	.await?;
	let busy = format!("posts/{post_id}/comments/{root}/replies");
	assert_eq!(
		as_bob.call(Method::POST, &busy, Some(json!({ "content": "x" }))).await?.0,
		StatusCode::BAD_REQUEST
	);
	let thread = anyone.get(&format!("posts/{post_id}/comments")).await?;
	let busy = thread["items"]
		.as_array()
		.into_iter()
		.flatten()
		.find(|item| item["_id"] == root.as_str());
	assert_eq!(busy.and_then(|item| item["replies"].as_array()).map(Vec::len), Some(200));

	// Uploads must be the image they claim to be, fit a daily quota, and are served so that nothing
	// stored runs in the browser
	let upload = |content_type: &str, data: &[u8]| {
		let mut body = format!(
			"--b\r\nContent-Disposition: form-data; name=\"file\"; filename=\"f\"\r\nContent-Type: {content_type}\r\n\r\n"
		)
		.into_bytes();
		body.extend_from_slice(data);
		body.extend_from_slice(b"\r\n--b--\r\n");
		as_bob
			.http
			.post(format!("{base}/s3/files"))
			.header("cookie", as_bob.cookie.clone().unwrap_or_default())
			.header("x-forwarded-for", "198.51.100.3")
			.header("content-type", "multipart/form-data; boundary=b")
			.body(body)
			.send()
	};
	let svg = upload("image/png", b"<svg onload=alert(1)>").await?;
	assert_eq!(svg.status(), StatusCode::BAD_REQUEST);
	let png = upload("image/png", b"\x89PNG\r\n\x1a\nimage").await?;
	assert_eq!(png.status(), StatusCode::OK);
	let cid = png.json::<Value>().await?["cid"].as_str().unwrap_or_default().to_owned();
	let served = anyone.http.get(format!("{base}/s3/files/{cid}")).send().await?;
	let header = |name: &str| {
		served
			.headers()
			.get(name)
			.and_then(|value| value.to_str().ok())
			.unwrap_or_default()
			.to_owned()
	};
	assert_eq!(header("content-type"), "image/png");
	assert_eq!(header("x-content-type-options"), "nosniff");
	assert!(header("content-security-policy").contains("sandbox"));
	db.execute(
		"INSERT INTO forum.files (id, content_type, data) VALUES ('page', 'text/html', '<script>')",
		&[],
	)
	.await?;
	let page = anyone.http.get(format!("{base}/s3/files/page")).send().await?;
	assert_eq!(
		page.headers().get("content-type").map(|v| v.as_bytes()),
		Some(&b"application/octet-stream"[..])
	);
	assert_eq!(
		page.headers().get("content-disposition").map(|v| v.as_bytes()),
		Some(&b"attachment"[..])
	);

	// The hourly sweep keeps uploads the forum's text links to, and recent ones, and drops the rest
	let (linked, unused) = ("a".repeat(64), "b".repeat(64));
	db.execute(
		"INSERT INTO forum.files (id, content_type, data, uploader, size, created_at)
		SELECT id, 'image/png', '\\x00', $3, 1, NOW() - INTERVAL '2 days' FROM unnest(ARRAY[$1, $2]) id",
		&[&linked, &unused, &address(&bob)],
	)
	.await?;
	db.execute(
		"UPDATE forum.posts SET content = COALESCE(content, '') || ' ![](/api/s3/files/' || $1 || ')'
		WHERE (kind, number) = (SELECT kind, number FROM forum.posts LIMIT 1)",
		&[&linked],
	)
	.await?;
	let (_, sweep) = crate::CLEAN_UPS[2];
	db.batch_execute(&format!("BEGIN; SET LOCAL search_path = forum; {sweep}; COMMIT"))
		.await?;
	let kept = db
		.query(
			"SELECT id FROM forum.files WHERE id = ANY($1) ORDER BY id",
			&[&vec![linked.clone(), unused, cid.clone()]],
		)
		.await?
		.iter()
		.map(|row| row.get::<_, String>(0))
		.collect::<Vec<_>>();
	assert_eq!(kept, [cid, linked]);

	db.execute(
		"INSERT INTO forum.files (id, content_type, data, uploader, size)
		SELECT 'quota' || n, 'image/png', '\\x00', $1, 1 FROM generate_series(1, 50) n",
		&[&address(&bob)],
	)
	.await?;
	let over = upload("image/png", b"\x89PNG\r\n\x1a\nanother").await?;
	assert_eq!(over.status(), StatusCode::BAD_REQUEST, "the daily quota is used up");

	// Rate limits apply to each visitor's writes on their own
	let mut flooder = Visitor::new(&base);
	flooder.forwarded_for = "198.51.100.99";
	for _ in 0..20 {
		flooder
			.call(Method::POST, "auth/login/none", Some(json!({ "challengeAnswer": "0x" })))
			.await?;
	}
	let (status, _) = flooder
		.call(Method::POST, "auth/login/none", Some(json!({ "challengeAnswer": "0x" })))
		.await?;
	assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
	let (status, _) = flooder
		.call(Method::GET, &format!("auth/login/{}", address(&bob)), None)
		.await?;
	assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "challenges count as logins");
	assert_eq!(
		anyone
			.call(Method::POST, "auth/login/none", Some(json!({ "challengeAnswer": "0x" })))
			.await?
			.0,
		StatusCode::BAD_REQUEST
	);
	Ok(())
}
