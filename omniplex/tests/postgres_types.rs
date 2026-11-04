//! Checks the Postgres encodings against a real server. Set `INDEXER_TEST_DATABASE_URL` to run.

// Only `database` is used here.
#[allow(dead_code)]
mod common;

use omniplex::store::{Numeric, Status};
use tokio_postgres::NoTls;

async fn client() -> Option<tokio_postgres::Client> {
	let url = std::env::var("INDEXER_TEST_DATABASE_URL").ok()?;
	let (client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	Some(client)
}

const VALUES: &[&str] = &[
	"0",
	"7",
	"-7",
	"10000",
	"115792089237316195423570985008687907853269984665640564039457584007913129639935",
	"0.5",
	"0.00001",
	"12.3400",
	"-0.000000000000000001",
	"1000000.000000000000000001",
];

#[tokio::test]
async fn numeric_matches_postgres() {
	let Some(db) = client().await else { return };
	for value in VALUES {
		let sent: String = db
			.query_one("SELECT $1::numeric::text", &[&Numeric::parse(value).unwrap()])
			.await
			.unwrap()
			.get(0);
		assert_eq!(sent, *value, "writing {value}");
		let read: Numeric =
			db.query_one(&format!("SELECT '{value}'::numeric"), &[]).await.unwrap().get(0);
		assert_eq!(read.as_str(), *value, "reading {value}");
	}
}

#[tokio::test]
async fn enums_bind_to_subquery_types() {
	// With the migrations applied, which create the enum types.
	let Some(db) = common::database().await else { return };
	let back: Status = db
		.query_one("SELECT $1::app.\"167d3578e1\"", &[&Status::HyperbridgeDelivered])
		.await
		.unwrap()
		.get(0);
	assert_eq!(back, Status::HyperbridgeDelivered);
}
