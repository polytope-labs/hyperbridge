use wasm_bindgen::prelude::*;
use ismp::host::{StateMachine};
use ismp::router::{Post, PostResponse};
use ethers::types::{Address, H160};

#[wasm_bindgen]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MessageStatus {
    Pending, // messaging is still residing on the source chain
    Hyperbridge, // messaging is on the hub (hyperbridge)
    Destination, // message has gotten to the destination chain)
    Timeout // message has timed out
}


#[wasm_bindgen]
pub struct WasmPost {
    /// The source state machine of this request.
    pub source: String,
    /// The destination state machine of this request.
    pub dest: String,
    /// The nonce of this request on the source chain
    pub nonce: u64,
    /// Module Id of the sending module
    pub from: Vec<u8>,
    /// Module ID of the receiving module
    pub to: Vec<u8>,
    /// Timestamp which this request expires in seconds.
    pub timeout_timestamp: u64,
    /// Encoded Request.
    pub data: Vec<u8>,
    /// Gas limit for executing the request on destination
    /// This value should be zero if destination module is not a contract
    pub gas_limit: u64,
}


#[wasm_bindgen]
pub struct WasmPostResponse {
    pub post: WasmPost,
    pub response: Vec<u8>,
    pub timeout_timestamp: u64,
    pub gas_limit: u64,
}

#[wasm_bindgen]
pub struct WasmHyperClientConfig {
    pub hyper_bridge_rpc_url: String,
    pub destination_ismp_host_address: String,
    pub source_ismp_host_address: String,
    pub destination_rpc_ws: String,
    pub source_rpc_ws: String,
}

pub struct HyperClientConfig {
    pub hyper_bridge_rpc_url: String,
    pub destination_ismp_host_address: H160,
    pub source_ismp_host_address: H160,
    pub destination_rpc_ws: String,
    pub source_rpc_ws: String,
}




impl From<WasmPostResponse> for PostResponse {
    fn from(wasm_post_response: WasmPostResponse) -> Self {
        Self {
            post: wasm_post_response.post.into(),
            response: wasm_post_response.response,
            timeout_timestamp: wasm_post_response.timeout_timestamp,
            gas_limit: wasm_post_response.gas_limit
        }
    }
}


impl From<WasmPost> for Post {
    fn from(wasm_post: WasmPost) -> Self {
        Post {
            source: wasm_post.source.into(),
            dest: wasm_post.dest.into(),
            nonce: wasm_post.nonce,
            from: wasm_post.from,
            to: wasm_post.to,
            timeout_timestamp: wasm_post.timeout_timestamp,
            data: wasm_post.data,
            gas_limit: wasm_post.gas_limit
        }
    }
}