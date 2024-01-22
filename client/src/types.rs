use wasm_bindgen::prelude::*;
use ismp::host::{StateMachine};
use ismp::router::Post;


#[wasm_bindgen]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MessageStatus {
    Pending,
    Hyperbridge,
    Destination,
    Timeout
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



pub fn build_state_machine(wasm_state_machine: String) -> StateMachine {
    wasm_state_machine.into()
}


pub fn build_post(wasm_post: WasmPost) -> Post {
    Post {
        source: build_state_machine(wasm_post.source),
        dest: build_state_machine(wasm_post.dest),
        nonce: wasm_post.nonce,
        from: wasm_post.from,
        to: wasm_post.to,
        timeout_timestamp: wasm_post.timeout_timestamp,
        data: wasm_post.data,
        gas_limit: wasm_post.gas_limit
    }
}