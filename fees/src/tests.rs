use crate::TransactionPayment;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::{Ethereum, StateMachine},
	messaging::{Message, Proof, RequestMessage, ResponseMessage},
	router::{Post, PostResponse, Request, RequestResponse, Response},
	util::{hash_request, hash_response},
};
use tesseract_primitives::{mocks::MockHost, Hasher, Query, TxReceipt};

#[tokio::test]
async fn transaction_payments_flow() {
	let tx_payment = TransactionPayment::initialize("./dev.db").await.unwrap();
	let receipts = (0..500).into_iter().map(|i| {
		let post = Post {
			source: StateMachine::Bsc,
			dest: StateMachine::Polygon,
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			data: vec![],
			gas_limit: i,
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request(Query {
			source_chain: req.source_chain(),
			dest_chain: req.dest_chain(),
			nonce: req.nonce(),
			commitment,
		})
	});

	let response_receipts = (0..500).into_iter().map(|i| {
		let resp = Response::Post(PostResponse {
			post: Post {
				source: StateMachine::Polygon,
				dest: StateMachine::Bsc,
				nonce: i,
				from: vec![],
				to: vec![],
				timeout_timestamp: 0,
				data: vec![],
				gas_limit: i,
			},
			response: vec![0u8; 64],
			timeout_timestamp: i,
			gas_limit: i,
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
		}
	});

	tx_payment
		.store_messages(receipts.chain(response_receipts).collect())
		.await
		.unwrap();

	let claim_proof = tx_payment
		.create_claim_proof(
			0,
			0,
			&MockHost::new((), 0, StateMachine::Bsc),
			&MockHost::new((), 0, StateMachine::Polygon),
		)
		.await
		.unwrap();

	assert_eq!(claim_proof.commitments.len(), 1000);
}

#[tokio::test]
async fn test_unique_deliveries() -> anyhow::Result<()> {
	let tx_payment = TransactionPayment::initialize("./dev2.db").await.unwrap();
	let receipts = (0..5).into_iter().map(|i| {
		let post = Post {
			source: StateMachine::Bsc,
			dest: StateMachine::Polygon,
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			data: vec![],
			gas_limit: i,
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request(Query {
			source_chain: req.source_chain(),
			dest_chain: req.dest_chain(),
			nonce: req.nonce(),
			commitment,
		})
	});

	let response_receipts = (0..5).into_iter().map(|i| {
		let resp = Response::Post(PostResponse {
			post: Post {
				source: StateMachine::Polygon,
				dest: StateMachine::Bsc,
				nonce: i,
				from: vec![],
				to: vec![],
				timeout_timestamp: 0,
				data: vec![],
				gas_limit: i,
			},
			response: vec![0u8; 64],
			timeout_timestamp: i,
			gas_limit: i,
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
		}
	});

	let receipts2 = (0..5).into_iter().map(|i| {
		let post = Post {
			source: StateMachine::Ethereum(Ethereum::ExecutionLayer),
			dest: StateMachine::Polygon,
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			data: vec![],
			gas_limit: i,
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request(Query {
			source_chain: req.source_chain(),
			dest_chain: req.dest_chain(),
			nonce: req.nonce(),
			commitment,
		})
	});

	let receipts3 = (0..5).into_iter().map(|i| {
		let post = Post {
			dest: StateMachine::Ethereum(Ethereum::ExecutionLayer),
			source: StateMachine::Polygon,
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			data: vec![],
			gas_limit: i,
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request(Query {
			source_chain: req.source_chain(),
			dest_chain: req.dest_chain(),
			nonce: req.nonce(),
			commitment,
		})
	});

	let receipts4 = (0..5).into_iter().map(|i| {
		let post = Post {
			dest: StateMachine::Ethereum(Ethereum::Optimism),
			source: StateMachine::Ethereum(Ethereum::Base),
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			data: vec![],
			gas_limit: i,
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request(Query {
			source_chain: req.source_chain(),
			dest_chain: req.dest_chain(),
			nonce: req.nonce(),
			commitment,
		})
	});

	let receipts5 = (0..5).into_iter().map(|i| {
		let post = Post {
			source: StateMachine::Ethereum(Ethereum::Optimism),
			dest: StateMachine::Ethereum(Ethereum::Base),
			nonce: i,
			from: vec![],
			to: vec![],
			timeout_timestamp: 0,
			data: vec![],
			gas_limit: i,
		};
		let req = Request::Post(post);
		let commitment = hash_request::<Hasher>(&req);
		TxReceipt::Request(Query {
			source_chain: req.source_chain(),
			dest_chain: req.dest_chain(),
			nonce: req.nonce(),
			commitment,
		})
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
