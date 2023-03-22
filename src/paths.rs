use crate::host::ChainID;
use derive_more::Display;

#[derive(Clone, Debug, Display, PartialEq, Eq)]
#[display(
    fmt = "acknowledgements/{}-{}/{}",
    "source_chain",
    "dest_chain",
    "nonce"
)]
pub struct AckPath {
    pub dest_chain: ChainID,
    pub source_chain: ChainID,
    pub nonce: u64,
}

#[derive(Clone, Debug, Display, PartialEq, Eq)]
#[display(fmt = "requests/{}-{}/{}", "source_chain", "dest_chain", "nonce")]
pub struct RequestPath {
    pub dest_chain: ChainID,
    pub source_chain: ChainID,
    pub nonce: u64,
}

#[derive(Clone, Debug, Display, PartialEq, Eq)]
#[display(fmt = "responses/{}-{}/{}", "source_chain", "dest_chain", "nonce")]
pub struct ResponsePath {
    pub dest_chain: ChainID,
    pub source_chain: ChainID,
    pub nonce: u64,
}
