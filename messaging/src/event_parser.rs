use ismp::{
	consensus::StateMachineHeight,
	events::{Event as IsmpEvent, StateMachineUpdated},
	host::StateMachine,
	messaging::{Message, Proof, RequestMessage, ResponseMessage},
	router::{Request, RequestResponse, Response},
	util::{hash_request, hash_response, Keccak256},
};
use sp_core::{keccak_256, H256};
use tesseract_primitives::{IsmpHost, IsmpProvider, Query};

/// Short description of a request/response event
#[derive(Debug)]
pub struct Meta {
	/// The source state machine of this request.
	pub source: StateMachine,
	/// The destination state machine of this request.
	pub dest: StateMachine,
	/// The nonce of this request on the source chain
	pub nonce: u64,
}

#[derive(Debug)]
pub enum Event {
	/// Emitted when a state machine is successfully updated to a new height after the challenge
	/// period has elapsed
	StateMachineUpdated(StateMachineUpdated),
	/// An event that is emitted when a post request is dispatched
	PostRequest(Meta),
	/// An event that is emitted when a post response is dispatched
	PostResponse(Meta),
	/// An event that is emitted when a get request is dispatched
	GetRequest(Meta),
}

impl From<IsmpEvent> for Event {
	fn from(value: IsmpEvent) -> Self {
		match value {
			IsmpEvent::StateMachineUpdated(e) => Event::StateMachineUpdated(e),
			IsmpEvent::PostRequest(e) =>
				Event::PostRequest(Meta { nonce: e.nonce, dest: e.dest, source: e.source }),
			IsmpEvent::PostResponse(e) => Event::PostResponse(Meta {
				nonce: e.post.nonce,
				dest: e.post.dest,
				source: e.post.source,
			}),
			IsmpEvent::GetRequest(e) =>
				Event::GetRequest(Meta { nonce: e.nonce, dest: e.dest, source: e.source }),
		}
	}
}

/// Parse events emitted from [`source`] into messages to be submitted to the counterparty
/// The [`state_machine_height`] parameter is the latest available height of [`source`] on
/// the counterparty chain
/// Returns a tuple where the first item are messages to be submitted to the sink
/// and the second items are messages to be submitted to the source
pub async fn parse_ismp_events<A, B>(
	source: &A,
	sink: &B,
	events: Vec<IsmpEvent>,
	state_machine_height: StateMachineHeight,
) -> Result<Vec<Message>, anyhow::Error>
where
	A: IsmpHost + IsmpProvider,
	B: IsmpHost + IsmpProvider,
{
	let mut post_request_queries = vec![];
	let mut response_queries = vec![];
	let mut post_requests = vec![];
	let mut post_responses = vec![];
	let counterparty_timestamp = sink.query_timestamp().await?;
	for event in events.iter() {
		match event {
			IsmpEvent::PostRequest(post) => {
				// Skip timed out requests
				if post.timeout_timestamp != 0 &&
					post.timeout_timestamp < counterparty_timestamp.as_secs()
				{
					continue
				}
				let req = Request::Post(post.clone());
				let hash = hash_request::<Hasher>(&req);
				post_requests.push(post.clone());
				post_request_queries.push(Query {
					source_chain: req.source_chain(),
					dest_chain: req.dest_chain(),
					nonce: req.nonce(),
					commitment: hash,
				})
			},
			IsmpEvent::PostResponse(post_response) => {
				let resp = Response::Post(post_response.clone());
				let hash = hash_response::<Hasher>(&resp);
				response_queries.push(Query {
					source_chain: resp.source_chain(),
					dest_chain: resp.dest_chain(),
					nonce: resp.nonce(),
					commitment: hash,
				});
				post_responses.push(resp);
			},
			_ => {},
		}
	}
	let mut messages = vec![];

	if !post_request_queries.is_empty() {
		let chunks = chunk_size(sink.state_machine_id().state_id);
		let query_chunks = post_request_queries.chunks(chunks);
		let post_request_chunks = post_requests.chunks(chunks);
		for (queries, post_requests) in query_chunks.into_iter().zip(post_request_chunks) {
			let requests_proof = source
				.query_requests_proof(state_machine_height.height, queries.to_vec())
				.await?;
			let msg = RequestMessage {
				requests: post_requests.to_vec(),
				proof: Proof { height: state_machine_height, proof: requests_proof },
				signer: sink.address(),
			};
			messages.push(Message::Request(msg));
		}
	}

	if !response_queries.is_empty() {
		let chunks = chunk_size(sink.state_machine_id().state_id);
		let query_chunks = response_queries.chunks(chunks);
		let post_request_chunks = post_responses.chunks(chunks);
		for (queries, post_responses) in query_chunks.into_iter().zip(post_request_chunks) {
			let responses_proof = source
				.query_responses_proof(state_machine_height.height, queries.to_vec())
				.await?;
			let msg = ResponseMessage {
				datagram: RequestResponse::Response(post_responses.to_vec()),
				proof: Proof { height: state_machine_height, proof: responses_proof },
				signer: sink.address(),
			};
			messages.push(Message::Response(msg));
		}
	}

	Ok(messages)
}

/// Return true for Request and Response events designated for the counterparty
pub fn filter_events(
	router_id: Option<StateMachine>,
	counterparty: StateMachine,
	ev: &IsmpEvent,
) -> bool {
	// Is the counterparty the routing chain?
	let is_router = router_id == Some(counterparty);

	match ev {
		IsmpEvent::PostRequest(post) => (post.dest == counterparty) || is_router,
		IsmpEvent::PostResponse(resp) => (resp.post.source == counterparty) || is_router,
		_ => false,
	}
}

fn chunk_size(state_machine: StateMachine) -> usize {
	match state_machine {
		StateMachine::Ethereum(_) => 100,
		_ => 200,
	}
}

pub struct Hasher;

impl Keccak256 for Hasher {
	fn keccak256(bytes: &[u8]) -> H256 {
		keccak_256(bytes).into()
	}
}
