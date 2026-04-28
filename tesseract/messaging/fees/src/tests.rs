use crate::TransactionPayment;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::{hash_request, hash_response, Message, Proof, RequestMessage, ResponseMessage},
	router::{PostRequest, PostResponse, Request, RequestResponse, Response},
};
use std::sync::Arc;
use tesseract_primitives::{mocks::MockHost, Hasher, Query, TxReceipt};

/// Build a unique on-disk path for a test database.
///
/// `prisma-client-rust` writes through to disk on `initialize`, so
/// `:memory:` doesn't work cleanly here; using `/tmp` with the test
/// name keeps the artifacts out of the workspace and avoids collisions
/// when tests run in parallel.
fn temp_db_path(test_name: &str) -> String {
	let pid = std::process::id();
	let nanos = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|d| d.as_nanos())
		.unwrap_or(0);
	let path = format!("/tmp/tesseract-fees-test-{test_name}-{pid}-{nanos}.db");
	let _ = std::fs::remove_file(&path);
	path
}

fn cleanup_db(path: &str) {
	let _ = std::fs::remove_file(path);
	let _ = std::fs::remove_file(format!("{path}-journal"));
}

#[tokio::test]
async fn transaction_payments_flow() {
	let tx_payment = TransactionPayment::initialize("./dev.db").await.unwrap();
	let receipts = (0..500).into_iter().map(|i| {
		let post = PostRequest {
			source: StateMachine::Evm(97),
			dest: StateMachine::Evm(8002),
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			body: vec![],
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request {
			query: Query {
				source_chain: req.source_chain(),
				dest_chain: req.dest_chain(),
				nonce: req.nonce(),
				commitment,
			},
			height: Default::default(),
		}
	});

	let response_receipts = (0..500).into_iter().map(|i| {
		let resp = Response::Post(PostResponse {
			post: PostRequest {
				source: StateMachine::Evm(8002),
				dest: StateMachine::Evm(97),
				nonce: i,
				from: vec![],
				to: vec![],
				timeout_timestamp: 0,
				body: vec![],
			},
			response: vec![0u8; 64],
			timeout_timestamp: i,
		});

		let commitment = hash_response::<Hasher>(&resp);
		let request_commitment = hash_request::<Hasher>(&resp.request());

		TxReceipt::Response {
			query: Query {
				source_chain: resp.source_chain(),
				dest_chain: resp.dest_chain(),
				nonce: resp.nonce(),
				commitment,
			},
			request_commitment,
			height: Default::default(),
		}
	});

	tx_payment
		.store_messages(receipts.chain(response_receipts).collect())
		.await
		.unwrap();

	let proofs = tx_payment
		.create_claim_proof(
			0,
			0,
			Arc::new(MockHost::new((), 0, StateMachine::Evm(97))),
			Arc::new(MockHost::new((), 0, StateMachine::Evm(8002))),
			&MockHost::new((), 0, StateMachine::Kusama(2000)),
		)
		.await
		.unwrap();

	assert_eq!(proofs.iter().fold(0, |acc, proof| proof.commitments.len() + acc), 1000);
	tx_payment
		.delete_claimed_entries(proofs.into_iter().fold(vec![], |mut acc, proof| {
			acc.extend(proof.commitments);
			acc
		}))
		.await
		.unwrap();
	let deliveries = tx_payment.db.deliveries().count(Default::default()).exec().await.unwrap();
	assert_eq!(deliveries, 0)
}

#[tokio::test]
#[ignore]
async fn test_unique_deliveries() -> anyhow::Result<()> {
	let tx_payment = TransactionPayment::initialize("./dev2.db").await.unwrap();
	let receipts = (0..5).into_iter().map(|i| {
		let post = PostRequest {
			source: StateMachine::Evm(97),
			dest: StateMachine::Evm(8002),
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			body: vec![],
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request {
			query: Query {
				source_chain: req.source_chain(),
				dest_chain: req.dest_chain(),
				nonce: req.nonce(),
				commitment,
			},
			height: Default::default(),
		}
	});

	let response_receipts = (0..5).into_iter().map(|i| {
		let resp = Response::Post(PostResponse {
			post: PostRequest {
				source: StateMachine::Evm(8002),
				dest: StateMachine::Evm(97),
				nonce: i,
				from: vec![],
				to: vec![],
				timeout_timestamp: 0,
				body: vec![],
			},
			response: vec![0u8; 64],
			timeout_timestamp: i,
		});

		let commitment = hash_response::<Hasher>(&resp);
		let request_commitment = hash_request::<Hasher>(&resp.request());

		TxReceipt::Response {
			query: Query {
				source_chain: resp.source_chain(),
				dest_chain: resp.dest_chain(),
				nonce: resp.nonce(),
				commitment,
			},
			request_commitment,
			height: Default::default(),
		}
	});

	let receipts2 = (0..5).into_iter().map(|i| {
		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Evm(8002),
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			body: vec![],
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request {
			query: Query {
				source_chain: req.source_chain(),
				dest_chain: req.dest_chain(),
				nonce: req.nonce(),
				commitment,
			},
			height: Default::default(),
		}
	});

	let receipts3 = (0..5).into_iter().map(|i| {
		let post = PostRequest {
			dest: StateMachine::Evm(1),
			source: StateMachine::Evm(8002),
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			body: vec![],
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request {
			query: Query {
				source_chain: req.source_chain(),
				dest_chain: req.dest_chain(),
				nonce: req.nonce(),
				commitment,
			},
			height: Default::default(),
		}
	});

	let receipts4 = (0..5).into_iter().map(|i| {
		let post = PostRequest {
			dest: StateMachine::Evm(100),
			source: StateMachine::Evm(200),
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			body: vec![],
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request {
			query: Query {
				source_chain: req.source_chain(),
				dest_chain: req.dest_chain(),
				nonce: req.nonce(),
				commitment,
			},
			height: Default::default(),
		}
	});

	let receipts5 = (0..5).into_iter().map(|i| {
		let post = PostRequest {
			source: StateMachine::Evm(100),
			dest: StateMachine::Evm(200),
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			body: vec![],
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request {
			query: Query {
				source_chain: req.source_chain(),
				dest_chain: req.dest_chain(),
				nonce: req.nonce(),
				commitment,
			},
			height: Default::default(),
		}
	});

	tx_payment
		.store_messages(
			receipts
				.chain(receipts2)
				.chain(receipts3)
				.chain(receipts4)
				.chain(receipts5)
				.chain(response_receipts)
				.collect(),
		)
		.await
		.unwrap();

	let unique = tx_payment.distinct_deliveries().await?;

	// there are only 3 unique chain pairs in the db
	assert_eq!(unique.len(), 3);

	Ok(())
}

#[tokio::test]
async fn highest_delivery_height() {
	let tx_payment = TransactionPayment::initialize("./dev_2.db").await.unwrap();
	let receipts = (0..500).into_iter().map(|i| {
		let post = PostRequest {
			source: StateMachine::Evm(97),
			dest: StateMachine::Evm(8002),
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			body: vec![],
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request {
			query: Query {
				source_chain: req.source_chain(),
				dest_chain: req.dest_chain(),
				nonce: req.nonce(),
				commitment,
			},
			height: i,
		}
	});

	let response_receipts = (0..500).into_iter().map(|i| {
		let resp = Response::Post(PostResponse {
			post: PostRequest {
				source: StateMachine::Evm(8002),
				dest: StateMachine::Evm(97),
				nonce: i,
				from: vec![],
				to: vec![],
				timeout_timestamp: 0,
				body: vec![],
			},
			response: vec![0u8; 64],
			timeout_timestamp: i,
		});

		let commitment = hash_response::<Hasher>(&resp);
		let request_commitment = hash_request::<Hasher>(&resp.request());

		TxReceipt::Response {
			query: Query {
				source_chain: resp.source_chain(),
				dest_chain: resp.dest_chain(),
				nonce: resp.nonce(),
				commitment,
			},
			request_commitment,
			height: i,
		}
	});

	tx_payment
		.store_messages(receipts.chain(response_receipts).collect())
		.await
		.unwrap();

	let height = tx_payment
		.highest_delivery_height(StateMachine::Evm(97), StateMachine::Evm(8002))
		.await
		.unwrap()
		.unwrap();

	assert_eq!(height, 499);
}

// ─── Outbound consensus delivery claim persistence ──────────────────

/// `list_pending_rotation_claims` returns every row in creation order.
#[tokio::test]
async fn outbound_rotation_claims_pending_round_trip() {
	let path = temp_db_path("rotation_pending");
	let tx_payment = TransactionPayment::initialize(&path).await.unwrap();

	tx_payment.insert_pending_rotation_claims("EVM-97", &[(7, 100)]).await.unwrap();
	tx_payment.insert_pending_rotation_claims("EVM-97", &[(8, 105)]).await.unwrap();
	tx_payment.insert_pending_rotation_claims("EVM-1", &[(4, 120)]).await.unwrap();

	let pending = tx_payment.list_pending_rotation_claims().await.unwrap();
	assert_eq!(pending.len(), 3);
	let keys: Vec<(String, i64)> = pending.iter().map(|r| (r.dest.clone(), r.set_id)).collect();
	assert!(keys.contains(&("EVM-97".to_string(), 7)));
	assert!(keys.contains(&("EVM-97".to_string(), 8)));
	assert!(keys.contains(&("EVM-1".to_string(), 4)));

	cleanup_db(&path);
}

/// Reopening the database surfaces the same pending claims that were
/// inserted before the previous handle was dropped. This is the crash
/// recovery path the relayer's startup replay relies on.
#[tokio::test]
async fn outbound_rotation_claims_survive_reopen() {
	let path = temp_db_path("rotation_reopen");

	{
		let tx_payment = TransactionPayment::initialize(&path).await.unwrap();
		tx_payment.insert_pending_rotation_claims("EVM-1", &[(11, 300)]).await.unwrap();
		tx_payment.insert_pending_rotation_claims("EVM-1", &[(12, 360)]).await.unwrap();
	}

	let tx_payment = TransactionPayment::initialize(&path).await.unwrap();
	let pending = tx_payment.list_pending_rotation_claims().await.unwrap();
	assert_eq!(pending.len(), 2);
	let mut keys: Vec<(String, i64, i64)> =
		pending.iter().map(|r| (r.dest.clone(), r.set_id, r.rotation_height)).collect();
	keys.sort();
	assert_eq!(keys, vec![("EVM-1".to_string(), 11, 300), ("EVM-1".to_string(), 12, 360),]);

	cleanup_db(&path);
}

/// `delete_rotation_claim` removes the row entirely so a subsequent
/// `list_pending_rotation_claims` no longer returns it.
#[tokio::test]
async fn outbound_rotation_claims_delete_drops_row() {
	let path = temp_db_path("rotation_delete");
	let tx_payment = TransactionPayment::initialize(&path).await.unwrap();

	tx_payment.insert_pending_rotation_claims("EVM-100", &[(3, 50)]).await.unwrap();
	tx_payment.insert_pending_rotation_claims("EVM-100", &[(4, 60)]).await.unwrap();

	tx_payment.delete_rotation_claim("EVM-100", 3).await.unwrap();

	let pending = tx_payment.list_pending_rotation_claims().await.unwrap();
	assert_eq!(pending.len(), 1);
	assert_eq!(pending[0].set_id, 4);

	let total: i64 = tx_payment.db.outbound_rotation_claims().count(vec![]).exec().await.unwrap();
	assert_eq!(total, 1, "deleted row should be gone from the table entirely");

	cleanup_db(&path);
}

/// Deleting an absent row is a no-op rather than an error. Keeps the
/// caller side sloppy: they don't have to check whether the row was
/// actually inserted before deleting on success.
#[tokio::test]
async fn outbound_rotation_claims_delete_absent_is_noop() {
	let path = temp_db_path("rotation_delete_absent");
	let tx_payment = TransactionPayment::initialize(&path).await.unwrap();

	tx_payment.delete_rotation_claim("EVM-56", 999).await.unwrap();

	let total: i64 = tx_payment.db.outbound_rotation_claims().count(vec![]).exec().await.unwrap();
	assert_eq!(total, 0);

	cleanup_db(&path);
}

/// Inserting the same `(destination, set_id)` twice is a no-op — the
/// upsert keeps the original row untouched. Lets the outbound task be
/// sloppy about deduplicating retries.
#[tokio::test]
async fn outbound_rotation_claims_upsert_is_idempotent() {
	let path = temp_db_path("rotation_upsert");
	let tx_payment = TransactionPayment::initialize(&path).await.unwrap();

	tx_payment
		.insert_pending_rotation_claims("EVM-8453", &[(17, 900)])
		.await
		.unwrap();
	// Same key, different rotation_height — upsert should not overwrite,
	// it should just leave the existing row alone (empty update vec).
	tx_payment
		.insert_pending_rotation_claims("EVM-8453", &[(17, 999)])
		.await
		.unwrap();

	use crate::db::{
		outbound_rotation_claims::WhereParam,
		read_filters::{BigIntFilter, StringFilter},
	};
	let rows = tx_payment
		.db
		.outbound_rotation_claims()
		.find_many(vec![
			WhereParam::Dest(StringFilter::Equals("EVM-8453".to_string())),
			WhereParam::SetId(BigIntFilter::Equals(17)),
		])
		.exec()
		.await
		.unwrap();
	assert_eq!(rows.len(), 1, "upsert with the same key must not duplicate");
	assert_eq!(rows[0].rotation_height, 900, "upsert must not overwrite");

	cleanup_db(&path);
}
