//! Solidity rust bindings
#![allow(missing_docs)]
use alloy_sol_types::sol;
use sp_std::prelude::*;

sol! {

struct PostRequest {
    // the source state machine of this request
    bytes source;
    // the destination state machine of this request
    bytes dest;
    // request nonce
    uint64 nonce;
    // Module Id of this request origin
    bytes from;
    // destination module id
    bytes to;
    // timestamp by which this request times out.
    uint64 timeoutTimestamp;
    // request body
    bytes body;
    // gas limit for executing this request on destination & its response (if any) on the source.
    uint64 gaslimit;
}

struct GetRequest {
    // the source state machine of this request
    bytes source;
    // the destination state machine of this request
    bytes dest;
    // request nonce
    uint64 nonce;
    // Module Id of this request origin
    bytes from;
    // timestamp by which this request times out.
    uint64 timeoutTimestamp;
    // Storage keys to read.
    bytes[] keys;
    // height at which to read destination state machine
    uint64 height;
    // gas limit for executing this request on destination & its response (if any) on the source.
    uint64 gaslimit;
}

struct StorageValue {
    bytes key;
    bytes value;
}

struct GetResponse {
    // The request that initiated this response
    GetRequest request;
    // storage values for get response
    StorageValue[] values;
}

struct PostResponse {
    // The request that initiated this response
    PostRequest request;
    // bytes for post response
    bytes response;
}

// An object for dispatching post requests to the IsmpDispatcher
struct DispatchPost {
    // bytes representation of the destination chain
    bytes dest;
    // the destination module
    bytes to;
    // the request body
    bytes body;
    // the timestamp at which this request should timeout
    uint64 timeoutTimestamp;
    // gas limit for executing this request on destination & its response (if any) on the source.
    uint64 gaslimit;
}

// An object for dispatching get requests to the IsmpDispatcher
struct DispatchGet {
    // bytes representation of the destination chain
    bytes dest;
    // height at which to read the state machine
    uint64 height;
    // Storage keys to read
    bytes[] keys;
    // the timestamp at which this request should timeout
    uint64 timeoutTimestamp;
    // gas limit for executing this request on destination & its response (if any) on the source.
    uint64 gaslimit;
}


function onAccept(PostRequest memory request) external;
function onPostResponse(PostResponse memory response) external;
function onGetResponse(GetResponse memory response) external;
function onPostTimeout(PostRequest memory request) external;
function onGetTimeout(GetRequest memory request) external;
}
