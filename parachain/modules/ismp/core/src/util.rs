//! ISMP utilities

use crate::router::{PostResponse, Request, Response};
use alloc::{string::ToString, vec::Vec};
use primitive_types::H256;

/// A trait that returns a 256 bit keccak has of some bytes
pub trait Keccak256 {
    /// Returns a keccak256 hash of a byte slice
    fn keccak256(bytes: &[u8]) -> H256
    where
        Self: Sized;
}

/// Return the keccak256 hash of a request
pub fn hash_request<H: Keccak256>(req: &Request) -> H256 {
    match req {
        Request::Post(post) => {
            let mut buf = Vec::new();

            buf.extend_from_slice(post.source.to_string().as_bytes());
            buf.extend_from_slice(post.dest.to_string().as_bytes());
            buf.extend_from_slice(&post.nonce.to_be_bytes());
            buf.extend_from_slice(&post.timeout_timestamp.to_be_bytes());
            buf.extend_from_slice(&post.from);
            buf.extend_from_slice(&post.to);
            buf.extend_from_slice(&post.data);
            buf.extend_from_slice(&post.gas_limit.to_be_bytes());
            H::keccak256(&buf[..])
        },
        Request::Get(get) => {
            let mut buf = Vec::new();
            buf.extend_from_slice(get.source.to_string().as_bytes());
            buf.extend_from_slice(get.dest.to_string().as_bytes());
            buf.extend_from_slice(&get.nonce.to_be_bytes());
            buf.extend_from_slice(&get.height.to_be_bytes());
            buf.extend_from_slice(&get.timeout_timestamp.to_be_bytes());
            buf.extend_from_slice(&get.from);
            get.keys.iter().for_each(|key| buf.extend_from_slice(key));
            buf.extend_from_slice(&get.gas_limit.to_be_bytes());
            H::keccak256(&buf[..])
        },
    }
}

/// Return the keccak256 of a response
pub fn hash_response<H: Keccak256>(res: &Response) -> H256 {
    match res {
        Response::Post(res) => hash_post_response::<H>(res),
        Response::Get(res) => hash_request::<H>(&Request::Get(res.get.clone())),
    }
}

/// Return the keccak256 of a response
pub fn hash_post_response<H: Keccak256>(res: &PostResponse) -> H256 {
    let req = &res.post;
    let mut buf = Vec::new();
    buf.extend_from_slice(req.source.to_string().as_bytes());
    buf.extend_from_slice(req.dest.to_string().as_bytes());
    buf.extend_from_slice(&req.nonce.to_be_bytes());
    buf.extend_from_slice(&req.timeout_timestamp.to_be_bytes());
    buf.extend_from_slice(&req.from);
    buf.extend_from_slice(&req.to);
    buf.extend_from_slice(&req.data);
    buf.extend_from_slice(&req.gas_limit.to_be_bytes());
    buf.extend_from_slice(&res.response);
    buf.extend_from_slice(&res.timeout_timestamp.to_be_bytes());
    buf.extend_from_slice(&res.gas_limit.to_be_bytes());
    H::keccak256(&buf[..])
}
