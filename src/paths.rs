use crate::host::ChainID;
use derive_more::Display;

#[derive(Clone, Debug, Display)]
#[display(fmt = "acknowledgements/{}/{}", "dest_chain", "nonce")]
pub struct AckPath {
    pub dest_chain: ChainID,
    pub nonce: u64,
}

#[derive(Clone, Debug, Display)]
#[display(fmt = "requests/{}/{}", "dest_chain", "nonce")]
pub struct RequestPath {
    pub dest_chain: ChainID,
    pub nonce: u64,
}

#[derive(Clone, Debug, Display)]
#[display(fmt = "responses/{}/{}", "dest_chain", "nonce")]
pub struct ResponsePath {
    pub dest_chain: ChainID,
    pub nonce: u64,
}
