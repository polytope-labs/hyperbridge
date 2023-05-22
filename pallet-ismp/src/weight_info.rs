//! Users of ismp should benchmark consensus clients and module callbacks

use crate::Config;
use alloc::boxed::Box;
use frame_support::weights::Weight;
use ismp_rs::{
    consensus::ConsensusClientId,
    messaging::{ConsensusMessage, Message, Proof, ResponseMessage, TimeoutMessage},
    router::{Request, Response},
};

/// A trait that provides information about how consensus client execute in the runtime
pub trait ConsensusClientWeight {
    /// Returns the weight that would be used in processing this consensus message
    fn verify_consensus(&self, msg: ConsensusMessage) -> Weight;
    /// Returns weight used in verifying this membership proof
    /// `items` is the number of values being verified
    /// The weight should ideally depend on the number of items being verified
    fn verify_membership(&self, items: usize, proof: &Proof) -> Weight;
    /// Returns weight used in verifying this state proof
    /// `items` is the number of keys being verified
    /// The weight should ideally depend on the number of items being verified
    fn verify_state_proof(&self, items: usize, proof: &Proof) -> Weight;
}

impl ConsensusClientWeight for () {
    fn verify_consensus(&self, _msg: ConsensusMessage) -> Weight {
        Weight::zero()
    }

    fn verify_membership(&self, _items: usize, _proof: &Proof) -> Weight {
        Weight::zero()
    }

    fn verify_state_proof(&self, _items: usize, _proof: &Proof) -> Weight {
        Weight::zero()
    }
}

/// A trait that provides weight information about how module callbacks execute
pub trait IsmpModuleWeight {
    /// Returns the weight used in processing this request
    fn on_accept(&self, request: &Request) -> Weight;
    /// Returns the weight used in processing this timeout
    fn on_timeout(&self, request: &Request) -> Weight;
    /// Returns the weight used in processing this response
    fn on_response(&self, response: &Response) -> Weight;
}

impl IsmpModuleWeight for () {
    fn on_accept(&self, _request: &Request) -> Weight {
        Weight::zero()
    }

    fn on_timeout(&self, _request: &Request) -> Weight {
        Weight::zero()
    }

    fn on_response(&self, _response: &Response) -> Weight {
        Weight::zero()
    }
}

pub trait WeightProvider {
    fn consensus_client(id: ConsensusClientId) -> Option<Box<dyn ConsensusClientWeight>>;

    fn module_callback(dest_module: &[u8]) -> Option<Box<dyn IsmpModuleWeight>>;
}

impl WeightProvider for () {
    fn consensus_client(_id: ConsensusClientId) -> Option<Box<dyn ConsensusClientWeight>> {
        None
    }

    fn module_callback(_dest_module: &[u8]) -> Option<Box<dyn IsmpModuleWeight>> {
        None
    }
}

/// These functions account fot storage reads and writes in the ismp message handlers
pub trait WeightInfo {
    fn create_consensus_client() -> Weight;
    fn handle_request_message() -> Weight;
    fn handle_response_message() -> Weight;
    fn handle_timeout_message() -> Weight;
}

impl WeightInfo for () {
    fn create_consensus_client() -> Weight {
        Weight::zero()
    }

    fn handle_request_message() -> Weight {
        Weight::zero()
    }

    fn handle_response_message() -> Weight {
        Weight::zero()
    }

    fn handle_timeout_message() -> Weight {
        Weight::zero()
    }
}

pub fn get_weight<T: Config>(messages: &[Message]) -> Weight {
    messages.into_iter().fold(Weight::zero(), |acc, msg| {
        match msg {
            Message::Consensus(_) => acc + <T as Config>::WeightInfo::create_consensus_client(),
            Message::Request(msg) => {
                let cb_weight = msg.requests.iter().fold(Weight::zero(), |acc, req| {
                    let dest_module = match req {
                        Request::Post(ref post) => post.to.as_slice(),
                        // Get requests are never submitted
                        _ => return acc,
                    };
                    let handle = <T as Config>::WeightProvider::module_callback(dest_module)
                        .unwrap_or(Box::new(()));
                    acc + handle.on_accept(&req)
                });

                let consensus_handler = <T as Config>::WeightProvider::consensus_client(
                    msg.proof.height.id.consensus_client,
                )
                .unwrap_or(Box::new(()));

                let proof_verification_weight =
                    consensus_handler.verify_membership(msg.requests.len(), &msg.proof);

                acc + cb_weight +
                    proof_verification_weight +
                    <T as Config>::WeightInfo::handle_request_message()
            }
            Message::Response(msg) => match msg {
                ResponseMessage::Post { responses, proof } => {
                    let cb_weight = responses.iter().fold(Weight::zero(), |acc, res| {
                        let dest_module = match res {
                            Response::Post { ref post, .. } => post.from.as_slice(),
                            _ => return acc,
                        };
                        let handle = <T as Config>::WeightProvider::module_callback(dest_module)
                            .unwrap_or(Box::new(()));
                        acc + handle.on_response(&res)
                    });

                    let consensus_handler = <T as Config>::WeightProvider::consensus_client(
                        proof.height.id.consensus_client,
                    )
                    .unwrap_or(Box::new(()));

                    let proof_verification_weight =
                        consensus_handler.verify_membership(responses.len(), &proof);

                    acc + cb_weight +
                        proof_verification_weight +
                        <T as Config>::WeightInfo::handle_response_message()
                }
                ResponseMessage::Get { requests, proof } => {
                    let cb_weight = requests.iter().fold(Weight::zero(), |acc, req| {
                        let dest_module = match req {
                            Request::Get(ref get) => get.from.as_slice(),
                            _ => return acc,
                        };
                        let handle = <T as Config>::WeightProvider::module_callback(dest_module)
                            .unwrap_or(Box::new(()));
                        acc + handle.on_response(&Response::Get {
                            get: req.get_request().unwrap(),
                            values: Default::default(),
                        })
                    });

                    let consensus_handler = <T as Config>::WeightProvider::consensus_client(
                        proof.height.id.consensus_client,
                    )
                    .unwrap_or(Box::new(()));

                    let proof_verification_weight =
                        consensus_handler.verify_state_proof(requests.len(), &proof);

                    acc + cb_weight +
                        proof_verification_weight +
                        <T as Config>::WeightInfo::handle_response_message()
                }
            },
            Message::Timeout(msg) => match msg {
                TimeoutMessage::Post { requests, timeout_proof } => {
                    let cb_weight = requests.iter().fold(Weight::zero(), |acc, req| {
                        let dest_module = match req {
                            Request::Post(ref post) => post.from.as_slice(),
                            _ => return acc,
                        };
                        let handle = <T as Config>::WeightProvider::module_callback(dest_module)
                            .unwrap_or(Box::new(()));
                        acc + handle.on_timeout(&req)
                    });

                    let consensus_handler = <T as Config>::WeightProvider::consensus_client(
                        timeout_proof.height.id.consensus_client,
                    )
                    .unwrap_or(Box::new(()));

                    let proof_verification_weight =
                        consensus_handler.verify_state_proof(requests.len(), &timeout_proof);

                    acc + cb_weight +
                        proof_verification_weight +
                        <T as Config>::WeightInfo::handle_response_message()
                }
                TimeoutMessage::Get { requests } => {
                    let cb_weight = requests.iter().fold(Weight::zero(), |acc, req| {
                        let dest_module = match req {
                            Request::Get(ref get) => get.from.as_slice(),
                            _ => return acc,
                        };
                        let handle = <T as Config>::WeightProvider::module_callback(dest_module)
                            .unwrap_or(Box::new(()));
                        acc + handle.on_timeout(&req)
                    });
                    acc + cb_weight + <T as Config>::WeightInfo::handle_timeout_message()
                }
            },
        }
    })
}
