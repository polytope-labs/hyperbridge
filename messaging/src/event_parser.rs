use ismp::{
    consensus::StateMachineHeight,
    events::Event,
    host::StateMachine,
    messaging::{Message, Proof, RequestMessage, ResponseMessage},
    router::{Request, Response},
    util::{hash_request, hash_response, Keccak256},
};
use sp_core::{keccak_256, H256};
use tesseract_primitives::{IsmpHost, IsmpProvider, Query};

/// Parse events emitted from [`source`] into messages to be submitted to the counterparty
/// The [`state_machine_height`] parameter is the latest available height of [`source`] on
/// the counterparty chain
/// Returns a tuple where the first item are messages to be submitted to the sink
/// and the second items are messages to be submitted to the source
pub async fn parse_ismp_events<A, B>(
    source: &A,
    sink: &B,
    events: Vec<Event>,
    state_machine_height: StateMachineHeight,
) -> Result<(Vec<Message>, Vec<Message>), anyhow::Error>
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
            Event::PostRequest(post) => {
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
            }
            Event::PostResponse(post_response) => {
                let resp = Response::Post(post_response.clone());
                let hash = hash_response::<Hasher>(&resp);
                response_queries.push(Query {
                    source_chain: resp.source_chain(),
                    dest_chain: resp.dest_chain(),
                    nonce: resp.nonce(),
                    commitment: hash,
                });
                post_responses.push(resp);
            }
            _ => {}
        }
    }
    let mut messages = vec![];
    let mut get_responses = vec![];

    if !post_request_queries.is_empty() {
        let requests_proof =
            source.query_requests_proof(state_machine_height.height, post_request_queries).await?;
        let msg = RequestMessage {
            requests: post_requests,
            proof: Proof { height: state_machine_height, proof: requests_proof },
        };
        messages.push(Message::Request(msg));
    }

    // Let's handle get requests
    let sink_latest_height_on_source =
        source.query_latest_state_machine_height(sink.state_machine_id()).await? as u64;
    let get_requests = source.query_pending_get_requests(sink_latest_height_on_source).await?;
    for get_request in get_requests {
        if get_request.timeout_timestamp != 0 &&
            get_request.timeout_timestamp < counterparty_timestamp.as_secs()
        {
            continue
        }
        let height = get_request.height;
        let state_proof = sink.query_state_proof(height, get_request.keys.clone()).await?;
        let msg = ResponseMessage::Get {
            requests: vec![Request::Get(get_request)],
            proof: Proof {
                height: StateMachineHeight { id: sink.state_machine_id(), height },
                proof: state_proof,
            },
        };
        get_responses.push(Message::Response(msg))
    }

    if !response_queries.is_empty() {
        let responses_proof =
            source.query_responses_proof(state_machine_height.height, response_queries).await?;
        let msg = ResponseMessage::Post {
            responses: post_responses,
            proof: Proof { height: state_machine_height, proof: responses_proof },
        };
        messages.push(Message::Response(msg));
    }

    Ok((messages, get_responses))
}

/// Return true for Request and Response events designated for the counterparty
pub fn filter_events(counterparty: StateMachine, ev: &Event) -> bool {
    match ev {
        Event::PostRequest(post) => post.dest == counterparty,
        Event::PostResponse(post_response) => post_response.post.source == counterparty,
        _ => false,
    }
}

pub struct Hasher;

impl Keccak256 for Hasher {
    fn keccak256(bytes: &[u8]) -> H256 {
        keccak_256(bytes).into()
    }
}
