use crate::error::Error;
use crate::host::ChainID;
use crate::prelude::{String, Vec};
use codec::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode)]
pub struct Request {
    pub nonce: u64,
    pub dest_chain: ChainID,
    pub from: String,
    pub to: String,
    pub timeout_timestamp: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Response {
    pub request: Request,
    pub response: Vec<u8>,
}

pub trait IISMPRouter {
    /// Dispatch a request from a module to the ISMP router.
    fn dispatch(request: Request) -> Result<(), Error>;

    /// Provide a response to a previously received request.
    fn write_response(response: Response) -> Result<(), Error>;
}
