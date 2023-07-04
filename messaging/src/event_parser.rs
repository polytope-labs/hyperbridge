use ismp::{
    consensus::StateMachineHeight,
    host::StateMachine,
    messaging::{Message, Proof, RequestMessage, ResponseMessage},
    router::Request,
};
use pallet_ismp::events::Event;
use tesseract_primitives::{IsmpHost, Query};

/// Parse events emitted from [`source`] into messages to be submitted to the counterparty
/// The [`state_machine_height`] parameter is the latest available height of [`source`] on
/// the counterparty chain
/// Returns a tuple where the first item are messages to be submitted to the sink
/// and the second items are messages to be submitted to the source
pub async fn parse_ismp_events<A: IsmpHost, B: IsmpHost>(
    source: &A,
    sink: &B,
    events: Vec<Event>,
    state_machine_height: StateMachineHeight,
) -> Result<(Vec<Message>, Vec<Message>), anyhow::Error> {
    let mut request_queries = vec![];
    let mut response_queries = vec![];

    for event in events {
        match event {
            Event::Response { dest_chain, source_chain, request_nonce } => {
                let query = Query { source_chain, dest_chain, nonce: request_nonce };

                response_queries.push(query)
            }
            Event::Request { dest_chain, source_chain, request_nonce } => {
                let query = Query { source_chain, dest_chain, nonce: request_nonce };

                request_queries.push(query)
            }
            _ => {}
        }
    }
    let mut messages = vec![];
    let mut get_responses = vec![];

    if !request_queries.is_empty() {
        let requests = source.query_requests(request_queries.clone()).await?;
        let mut post_requests = vec![];

        for request in requests {
            if let Request::Post(post) = request {
                post_requests.push(post)
            }
        }

        let post_request_queries: Vec<_> = post_requests
            .iter()
            .map(|req| Query {
                source_chain: req.source_chain,
                dest_chain: req.dest_chain,
                nonce: req.nonce,
            })
            .collect();
        if !post_request_queries.is_empty() {
            let requests_proof = source
                .query_requests_proof(state_machine_height.height, post_request_queries)
                .await?;
            let msg = RequestMessage {
                requests: post_requests,
                proof: Proof { height: state_machine_height, proof: requests_proof },
            };
            messages.push(Message::Request(msg));
        }
    };

    // Let's handle get requests
    let sink_latest_height_on_source =
        source.query_latest_state_machine_height(sink.state_machine_id()).await? as u64;
    let get_requests = source.query_pending_get_requests(sink_latest_height_on_source).await?;
    log::info!(
        target: "tesseract",
        "Get requests {:?}",
        get_requests
    );
    for get_request in get_requests {
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
        let responses = source.query_responses(response_queries.clone()).await?;
        let responses_proof =
            source.query_responses_proof(state_machine_height.height, response_queries).await?;
        let msg = ResponseMessage::Post {
            responses,
            proof: Proof { height: state_machine_height, proof: responses_proof },
        };
        messages.push(Message::Response(msg))
    };

    Ok((messages, get_responses))
}

/// Return true for Request and Response events designated for the counterparty
pub fn filter_events(counterparty: StateMachine, ev: &Event) -> bool {
    match ev {
        Event::Response { dest_chain, .. } => *dest_chain == counterparty,
        Event::Request { dest_chain, .. } => *dest_chain == counterparty,
        _ => false,
    }
}
