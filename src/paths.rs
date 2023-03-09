use crate::host::ChainID;
use derive_more::Display;

#[derive(Clone, Debug, Display)]
#[display(fmt = "ack/{}/{}", "chain_id", "nonce")]
pub struct AckPath {
    pub chain_id: ChainID,
    pub nonce: u64,
}
