//! Runs the API against a real Postgres loaded with indexed Nexus data. It needs a database it may
//! wipe, so it only runs with `TEST_DATABASE_URL` set:
//!
//!   TEST_DATABASE_URL=postgres://user:pass@localhost/test cargo test -p governance-api --
//! --ignored

use std::{env, net::SocketAddr, sync::Arc};

use anyhow::Context;
use reqwest::{Method, StatusCode};
use serde_json::{json, Value};
use sp_core::{
	crypto::{AccountId32, Ss58AddressFormat, Ss58Codec},
	sr25519, Pair,
};
use tokio_postgres::NoTls;

use super::{router, AppState};
use crate::{db, limits::RateLimiter};

const REFERENDUM: i32 = 80;
const MOTION: i32 = 65;

fn address(pair: &sr25519::Pair) -> String {
	AccountId32::from(pair.public().0).to_ss58check_with_version(Ss58AddressFormat::custom(0))
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
		let mut visitor = Self::new(base);
		let challenge = visitor.get(&format!("auth/login/{}", address(pair))).await?;
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

/// Loads the fixture into fresh schemas and serves the API on a free port.
async fn serve(admins: Vec<String>) -> anyhow::Result<String> {
	let url = env::var("TEST_DATABASE_URL").context("TEST_DATABASE_URL is not set")?;
	let (client, connection) = tokio_postgres::connect(&url, NoTls).await?;
	tokio::spawn(connection);
	client
		.batch_execute(
			"DROP SCHEMA IF EXISTS forum CASCADE; DROP SCHEMA IF EXISTS governance CASCADE;",
		)
		.await?;
	client.batch_execute(include_str!("../../tests/governance.sql")).await?;

	let state = AppState {
		db: Arc::new(db::connect(&url).await?),
		admins: Arc::new(admins),
		limiter: Arc::new(RateLimiter::default()),
	};
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
	let base = format!("http://{}", listener.local_addr()?);
	let app = router(state).into_make_service_with_connect_info::<SocketAddr>();
	tokio::spawn(async move { axum::serve(listener, app).await });
	Ok(base)
}

#[tokio::test]
#[ignore = "needs TEST_DATABASE_URL pointing at a database it may wipe"]
async fn serves_the_forum() -> anyhow::Result<()> {
	let alice = sr25519::Pair::from_string("//Alice", None)?;
	let bob = sr25519::Pair::from_string("//Bob", None)?;
	let base = serve(vec![address(&alice)]).await?;
	let anyone = Visitor::new(&base);

	// Indexed data comes back in the frontend's shapes, and unused filters arrive empty
	let list = anyone
		.get("gov2/referendums?page=1&page_size=25&status=&ongoing=&is_treasury=&sort=")
		.await?;
	assert_eq!(list["total"], 2);
	let referendum = anyone.get(&format!("gov2/referendums/{REFERENDUM}")).await?;
	assert_eq!(referendum["state"]["name"], "Executed");
	assert_eq!(referendum["trackInfo"]["origin"], "WhitelistedCaller");
	assert_eq!(referendum["onchainData"]["timeline"][0]["args"]["referendumIndex"], REFERENDUM);
	let votes = anyone.get(&format!("gov2/referenda/{REFERENDUM}/votes")).await?;
	let stats = anyone.get(&format!("gov2/referendums/{REFERENDUM}/vote-stats")).await?;
	assert_eq!(votes.as_array().map(Vec::len), Some(2));
	assert_eq!(stats["directAddresses"], 2);
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

	// Delegate announcements
	let announcement = json!({
		"action": "set-delegation-announcement",
		"shortDescription": "I vote often",
		"longDescription": "",
		"timestamp": now,
	});
	assert_eq!(
		anyone
			.signed("delegation/announcements", &alice, announcement, address(&alice))
			.await?,
		StatusCode::OK
	);
	let delegate = anyone
		.get(&format!("delegation/referenda/delegates/{}", address(&alice)))
		.await?;
	assert_eq!(delegate["manifesto"]["shortDescription"], "I vote often");

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
	let saved = anyone.get(&multisigs).await?;
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
	assert_eq!(
		anyone
			.call(Method::POST, "auth/login/none", Some(json!({ "challengeAnswer": "0x" })))
			.await?
			.0,
		StatusCode::BAD_REQUEST
	);
	Ok(())
}
