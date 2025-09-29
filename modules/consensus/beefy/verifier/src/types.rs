use codec::{Encode, Decode};
use polkadot_sdk::*;
use sp_core::H256;

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct MmrLeaf {
    pub k_index: u32,
    pub leaf_index: u32,
    pub hash: H256
}