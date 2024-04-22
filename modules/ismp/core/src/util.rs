//! ISMP utilities

use crate::router::{PostResponse, Request, Response};
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
    let encoded = req.encode();
    H::keccak256(&encoded)
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
    H::keccak256(&res.encode())
}
