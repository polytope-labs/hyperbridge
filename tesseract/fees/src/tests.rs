use crate::TransactionPayment;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::{ethereum, StateMachine},
	messaging::{hash_request, hash_response, Message, Proof, RequestMessage, ResponseMessage},
	router::{PostRequest, PostResponse, Request, RequestResponse, Response},
};
use std::sync::Arc;
use tesseract_primitives::{mocks::MockHost, Hasher, Query, TxReceipt};

#[tokio::test]
async fn transaction_payments_flow() {
	let tx_payment = TransactionPayment::initialize("./dev.db").await.unwrap();
	let receipts = (0..500).into_iter().map(|i| {
		let post = PostRequest {
			source: StateMachine::Bsc,
			dest: StateMachine::Polygon,
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
				source: StateMachine::Polygon,
				dest: StateMachine::Bsc,
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
			Arc::new(MockHost::new((), 0, StateMachine::Bsc)),
			Arc::new(MockHost::new((), 0, StateMachine::Polygon)),
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
			source: StateMachine::Bsc,
			dest: StateMachine::Polygon,
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
				source: StateMachine::Polygon,
				dest: StateMachine::Bsc,
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
			source: StateMachine::Ethereum(ethereum::EXECUTION_LAYER),
			dest: StateMachine::Polygon,
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
			dest: StateMachine::Ethereum(ethereum::EXECUTION_LAYER),
			source: StateMachine::Polygon,
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
			dest: StateMachine::Ethereum(ethereum::OPTIMISM),
			source: StateMachine::Ethereum(ethereum::BASE),
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
			source: StateMachine::Ethereum(ethereum::OPTIMISM),
			dest: StateMachine::Ethereum(ethereum::BASE),
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
			source: StateMachine::Bsc,
			dest: StateMachine::Polygon,
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
				source: StateMachine::Polygon,
				dest: StateMachine::Bsc,
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
		.highest_delivery_height(StateMachine::Bsc, StateMachine::Polygon)
		.await
		.unwrap()
		.unwrap();

	assert_eq!(height, 499);
}
