use ismp::{
    consensus_client::StateMachineHeight,
    messaging::{Message, Proof, RequestMessage, ResponseMessage},
};
use pallet_ismp::events::Event;
use tesseract_primitives::{IsmpHost, Query};

/// Parse events emitted from [`source`] into messages to be submitted to the counterparty
/// The [`state_machine_height`] parameter is the latest available height of [`source`] on
/// the counterparty chain
pub async fn parse_ismp_events<A: IsmpHost>(
    source: &A,
    events: Vec<Event>,
    state_machine_height: StateMachineHeight,
) -> Result<Vec<Message>, anyhow::Error> {
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

    let requests = source.query_requests(request_queries.clone()).await?;
    let responses = source.query_responses(response_queries.clone()).await?;

    let responses_proof =
        source.query_responses_proof(state_machine_height.height, response_queries).await?;
    let requests_proof =
        source.query_requests_proof(state_machine_height.height, request_queries).await?;

    let request_message = RequestMessage {
        requests,
        proof: Proof { height: state_machine_height, proof: requests_proof },
    };

    let response_message = ResponseMessage {
        responses,
        proof: Proof { height: state_machine_height, proof: responses_proof },
    };

    Ok(vec![Message::Request(request_message), Message::Response(response_message)])
}
