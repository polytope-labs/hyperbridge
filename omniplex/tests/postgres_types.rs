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

/**
 * Two statements sent together on one transaction: once the first fails, the second only fails
 * with 25P02. `first_failure` keeps the first one's error, which says what went wrong.
 */
#[tokio::test]
async fn the_cause_beats_the_aborted_transaction_it_leaves() {
	use omniplex::errors::{first_failure, is_aborted_transaction, is_transient};
	let Some(mut client) = client().await else { return };
	let tx = client.transaction().await.unwrap();
	let refused = async {
		tx.execute("SELECT $1::text", &[&"a\0b"]).await?;
		anyhow::Ok(())
	};
	let after = async {
		tokio::time::sleep(std::time::Duration::from_millis(50)).await;
		tx.execute("SELECT 1", &[]).await?;
		anyhow::Ok(())
	};
	let (after, refused) = futures::join!(after, refused);
	assert!(is_aborted_transaction(after.as_ref().unwrap_err()));
	let err = first_failure(vec![after, refused]).unwrap_err();
	assert!(!is_aborted_transaction(&err) && !is_transient(&err), "{err:#}");
}

/**
 * Errors tokio-postgres raises itself carry no SQLSTATE. One that says the statement doesn't
 * fit its values or its rows repeats on a retry. A lost connection doesn't.
 */
#[tokio::test]
async fn client_errors_that_repeat_are_not_transient() {
	use omniplex::errors::is_transient;
	let Some(db) = client().await else { return };
	let row = db.query_one("SELECT 1::int4 AS number", &[]).await.unwrap();
	let lasting = [
		// A column read as another type, and one the row lacks.
		row.try_get::<_, String>("number").unwrap_err(),
		row.try_get::<_, i32>("missing").unwrap_err(),
		// A parameter of another type, and too few of them.
		db.query("SELECT $1::int4", &[&"text"]).await.unwrap_err(),
		db.query("SELECT $1::int4", &[]).await.unwrap_err(),
		// Fewer or more rows than the one asked for, and two columns read as one value.
		db.query_one("SELECT 1 WHERE false", &[]).await.unwrap_err(),
		db.query_one("SELECT generate_series(1, 2)", &[]).await.unwrap_err(),
		db.query_one_scalar::<i32, _>("SELECT 1, 2", &[]).await.unwrap_err(),
		// A statement that can't be sent: its text holds a NUL.
		db.query("SELECT 1\0", &[]).await.unwrap_err(),
	];
	for err in lasting {
		assert!(err.code().is_none(), "{err}");
		let err = anyhow::Error::new(err).context("a handler's statement");
		assert!(!is_transient(&err), "{err:#}");
	}
	// The server ends the session, and the next statement finds the connection gone.
	let ended = db.execute("SELECT pg_terminate_backend(pg_backend_pid())", &[]).await;
	assert!(ended.is_err_and(|err| is_transient(&err.into())));
	let closed = db.execute("SELECT 1", &[]).await.unwrap_err();
	assert!(closed.code().is_none() && is_transient(&closed.into()));
}
