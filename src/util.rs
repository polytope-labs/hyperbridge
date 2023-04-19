use crate::{
    host::ISMPHost,
    router::{Request, Response},
};
use alloc::{string::ToString, vec::Vec};
use primitive_types::H256;

/// Return the keccak256 hash of a request
/// Commitment is the hash of the concatenation of the data below
/// request.source_chain + request.dest_chain + request.nonce + request.data
pub fn hash_request<H: ISMPHost>(req: &Request) -> H256 {
    let req = match req {
        Request::Post(post) => post,
        _ => unimplemented!(),
    };

    let mut buf = Vec::new();

    let source_chain = req.source_chain.to_string();
    let dest_chain = req.dest_chain.to_string();
    let nonce = req.nonce.to_be_bytes();
    let timestamp = req.timeout_timestamp.to_be_bytes();
    buf.extend_from_slice(source_chain.as_bytes());
    buf.extend_from_slice(dest_chain.as_bytes());
    buf.extend_from_slice(&nonce);
    buf.extend_from_slice(&timestamp);
    buf.extend_from_slice(&req.data);
    buf.extend_from_slice(&req.from);
    buf.extend_from_slice(&req.to);
    H::keccak256(&buf[..])
}

/// Return the keccak256 of a response
pub fn hash_response<H: ISMPHost>(res: &Response) -> H256 {
    let req = match res.request {
        Request::Post(ref post) => post,
        _ => unimplemented!(),
    };
    let mut buf = Vec::new();
    let source_chain = req.source_chain.to_string();
    let dest_chain = req.dest_chain.to_string();
    let nonce = req.nonce.to_be_bytes();
    let timestamp = req.timeout_timestamp.to_be_bytes();
    buf.extend_from_slice(source_chain.as_bytes());
    buf.extend_from_slice(dest_chain.as_bytes());
    buf.extend_from_slice(&nonce);
    buf.extend_from_slice(&timestamp);
    buf.extend_from_slice(&req.data);
    buf.extend_from_slice(&req.from);
    buf.extend_from_slice(&req.to);
    buf.extend_from_slice(&res.response);
    H::keccak256(&buf[..])
}
