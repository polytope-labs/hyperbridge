use subxt::dynamic::{Value};
use codec::Encode;
use subxt::ext::scale_value::{Composite, value};
use ismp::consensus::StateMachineId;
use ismp::host::StateMachine;
use ismp::messaging::{ConsensusMessage, FraudProofMessage, Message, RequestMessage, ResponseMessage, TimeoutMessage};
use ismp::router::{GetRequest, GetResponse, PostRequest, PostResponse, Request, RequestResponse, Response};


pub fn message_to_value(message: &Message) -> Value<()> {
    match message {
        Message::Consensus(msg) => Value::variant("Consensus", Composite::named(vec![
            ("consensus_proof".to_string(), Value::from_bytes(msg.consensus_proof.clone())),
            ("consensus_state_id".to_string(), Value::from_bytes(msg.consensus_state_id.to_vec())),
            ("signer".to_string(), Value::from_bytes(msg.signer.clone())),
        ])),
        Message::FraudProof(msg) => Value::variant("FraudProof", Composite::named(vec![
            ("proof_1".to_string(), Value::from_bytes(msg.proof_1.clone())),
            ("proof_2".to_string(), Value::from_bytes(msg.proof_2.clone())),
            ("consensus_state_id".to_string(), Value::from_bytes(msg.consensus_state_id.to_vec())),
            ("signer".to_string(), Value::from_bytes(msg.signer.clone())),
        ])),
        Message::Request(msg) => Value::variant("Request", Composite::named(vec![
            ("requests".to_string(), Value::unnamed_composite(msg.requests.iter().map(post_request_to_value))),
            ("proof".to_string(), Value::from_bytes(msg.proof.encode())),
            ("signer".to_string(), Value::from_bytes(msg.signer.clone())),
        ])),
        Message::Response(msg) => Value::variant("Response", response_message_to_composite(msg)),
        Message::Timeout(msg) => {
            let timeout_variant = timeout_message_to_value(msg);
            Value::variant("Timeout", Composite::unnamed(vec![timeout_variant]))
        },
    }
}

fn response_message_to_composite(msg: &ResponseMessage) -> Composite<()> {
    let datagram_value = match &msg.datagram {
        RequestResponse::Request(reqs) => {
            Value::variant("Request", Composite::unnamed(reqs.iter().map(request_to_value)))
        },
        RequestResponse::Response(resps) => {
            Value::variant("Response", Composite::unnamed(resps.iter().map(response_to_value)))
        },
    };
    Composite::named(vec![
        ("datagram".to_string(), datagram_value),
        ("proof".to_string(), Value::from_bytes(msg.proof.encode())),
        ("signer".to_string(), Value::from_bytes(msg.signer.clone())),
    ])
}

fn timeout_message_to_value(msg: &TimeoutMessage) -> Value<()> {
    match msg {
        TimeoutMessage::Post { requests, timeout_proof } => {
            Value::variant("Post", Composite::named(vec![
                ("requests".to_string(), Value::unnamed_composite(requests.iter().map(request_to_value))),
                ("timeout_proof".to_string(), Value::from_bytes(timeout_proof.encode())),
            ]))
        },
        TimeoutMessage::PostResponse { responses, timeout_proof } => {
            Value::variant("PostResponse", Composite::named(vec![
                ("responses".to_string(), Value::unnamed_composite(responses.iter().map(post_response_to_value))),
                ("timeout_proof".to_string(), Value::from_bytes(timeout_proof.encode())),
            ]))
        },
        TimeoutMessage::Get { requests } => {
            Value::variant("Get", Composite::named(vec![
                ("requests".to_string(), Value::unnamed_composite(requests.iter().map(request_to_value))),
            ]))
        },
    }
}

fn request_to_value(req: &Request) -> Value<()> {
    match req {
        Request::Post(post) => Value::variant("Post", Composite::named(vec![
            ("source".to_string(), state_machine_to_value(&post.source)),
            ("dest".to_string(), state_machine_to_value(&post.dest)),
            ("nonce".to_string(), Value::u128(post.nonce.into())),
            ("from".to_string(), Value::from_bytes(post.from.clone())),
            ("to".to_string(), Value::from_bytes(post.to.clone())),
            ("timeout_timestamp".to_string(), Value::u128(post.timeout_timestamp.into())),
            ("body".to_string(), Value::from_bytes(post.body.clone())),
        ])),
        Request::Get(get) => Value::variant("Get", Composite::named(vec![
            ("source".to_string(), state_machine_to_value(&get.source)),
            ("dest".to_string(), state_machine_to_value(&get.dest)),
            ("nonce".to_string(), Value::u128(get.nonce.into())),
            ("from".to_string(), Value::from_bytes(get.from.clone())),
            ("keys".to_string(), Value::unnamed_composite(get.keys.iter().map(|k| Value::from_bytes(k.clone())))),
            ("height".to_string(), Value::u128(get.height.into())),
            ("context".to_string(), Value::from_bytes(get.context.clone())),
            ("timeout_timestamp".to_string(), Value::u128(get.timeout_timestamp.into())),
        ])),
    }
}

fn response_to_value(resp: &Response) -> Value<()> {
    match resp {
        Response::Post(post) => Value::variant("Post", Composite::named(vec![
            ("post".to_string(), post_response_to_value(post)),
            ("response".to_string(), Value::from_bytes(post.response.clone())),
            ("timeout_timestamp".to_string(), Value::u128(post.timeout_timestamp.into())),
        ])),
        Response::Get(get) => Value::variant("Get", Composite::named(vec![
            ("get".to_string(), get_request_to_value(&get.get)),
            ("values".to_string(), Value::unnamed_composite(
                get.values.iter().map(|v| {
                    Value::named_composite(vec![
                        ("key".to_string(), Value::from_bytes(v.key.clone())),
                        ("value".to_string(), Value::from_bytes(v.value.clone().unwrap_or_default())),
                    ])
                })
            )),
        ])),
    }
}

fn post_request_to_value(post: &PostRequest) -> Value<()> {
    Value::named_composite(vec![
        ("source".to_string(), state_machine_to_value(&post.source)),
        ("dest".to_string(), state_machine_to_value(&post.dest)),
        ("nonce".to_string(), Value::u128(post.nonce.into())),
        ("from".to_string(), Value::from_bytes(post.from.clone())),
        ("to".to_string(), Value::from_bytes(post.to.clone())),
        ("timeout_timestamp".to_string(), Value::u128(post.timeout_timestamp.into())),
        ("body".to_string(), Value::from_bytes(post.body.clone())),
    ])
}

fn get_request_to_value(get: &GetRequest) -> Value<()> {
    Value::named_composite(vec![
        ("source".to_string(), state_machine_to_value(&get.source)),
        ("dest".to_string(), state_machine_to_value(&get.dest)),
        ("nonce".to_string(), Value::u128(get.nonce.into())),
        ("from".to_string(), Value::from_bytes(get.from.clone())),
        ("keys".to_string(), Value::unnamed_composite(get.keys.iter().map(|k| Value::from_bytes(k.clone())))),
        ("height".to_string(), Value::u128(get.height.into())),
        ("context".to_string(), Value::from_bytes(get.context.clone())),
        ("timeout_timestamp".to_string(), Value::u128(get.timeout_timestamp.into())),
    ])
}

fn post_response_to_value(post: &PostResponse) -> Value<()> {
    Value::named_composite(vec![
        ("post".to_string(), post_request_to_value(&post.post)),
        ("response".to_string(), Value::from_bytes(post.response.clone())),
        ("timeout_timestamp".to_string(), Value::u128(post.timeout_timestamp.into())),
    ])
}

pub fn state_machine_to_value(sm: &StateMachine) -> Value<()> {
    match sm {
        StateMachine::Evm(id) => Value::variant("Evm", Composite::unnamed(vec![Value::u128((*id).into())])),
        StateMachine::Polkadot(id) => Value::variant("Polkadot", Composite::unnamed(vec![Value::u128((*id).into())])),
        StateMachine::Kusama(id) => Value::variant("Kusama", Composite::unnamed(vec![Value::u128((*id).into())])),
        StateMachine::Substrate(id) => Value::variant("Substrate", Composite::unnamed(vec![Value::from_bytes(id.to_vec())])),
        StateMachine::Tendermint(id) => Value::variant("Tendermint", Composite::unnamed(vec![Value::from_bytes(id.to_vec())])),
        StateMachine::Relay { relay, para_id } => {
            let composite = Composite::named(vec![
                ("relay".to_string(), Value::from_bytes(relay.to_vec())),
                ("para_id".to_string(), Value::u128((*para_id).into())),
            ]);
            Value::variant("Relay", composite)
        },
    }
}

pub fn state_machine_id_to_value(state_machine_id: &StateMachineId) -> Value {
    let state_id_value = state_machine_to_value(&state_machine_id.state_id);

    let state_machine_id_value = value!({
        state_id: state_id_value,
        consensus_state_id: state_machine_id.consensus_state_id.to_vec()
    });

    state_machine_id_value
}