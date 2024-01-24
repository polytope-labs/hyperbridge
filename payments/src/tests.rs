use crate::TransactionPayment;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::{Message, Proof, RequestMessage, ResponseMessage},
	router::{Post, PostResponse, RequestResponse, Response},
};
use tesseract_primitives::mocks::MockHost;

#[tokio::test]
async fn transaction_payments_flow() {
	let tx_payment = TransactionPayment::initialize().await.unwrap();
	let request_message = Message::Request(RequestMessage {
		requests: (0..500)
			.into_iter()
			.map(|i| Post {
				source: StateMachine::Bsc,
				dest: StateMachine::Polygon,
				nonce: i,
				from: vec![],
				to: vec![],
				timeout_timestamp: 0,
				data: vec![],
				gas_limit: i,
			})
			.collect(),
		proof: Proof {
			height: StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Polygon,
					consensus_state_id: *b"POLY",
				},
				height: 0,
			},
			proof: vec![],
		},
		signer: vec![],
	});

	let response_message = Message::Response(ResponseMessage {
		datagram: RequestResponse::Response(
			(0..500)
				.into_iter()
				.map(|i| {
					Response::Post(PostResponse {
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
					})
				})
				.collect(),
		),
		proof: Proof {
			height: StateMachineHeight {
				id: StateMachineId { state_id: StateMachine::Bsc, consensus_state_id: *b"POLY" },
				height: 0,
			},
			proof: vec![],
		},
		signer: vec![],
	});

	tx_payment
		.store_messages(vec![request_message, response_message])
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
