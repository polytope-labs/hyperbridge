//! SCALE-layout replicas of the SP1 BEEFY proof types — no polkadot-sdk pull-in.
//! Field order matches the upstream structs byte-for-byte; SCALE is positional.

extern crate alloc;

use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode};

pub type H256 = [u8; 32];

pub const PROOF_TYPE_SP1: u8 = 0x01;

/// Mirrors `sp_consensus_beefy::mmr::MmrLeafVersion`.
#[derive(Clone, Copy, Debug, Encode, Decode)]
pub struct MmrLeafVersion(pub u8);

/// Mirrors `sp_consensus_beefy::mmr::BeefyAuthoritySet<H256>`.
#[derive(Clone, Debug, Encode, Decode)]
pub struct BeefyAuthoritySet {
    pub id: u64,
    pub len: u32,
    pub keyset_commitment: H256,
}

/// Mirrors `sp_consensus_beefy::mmr::MmrLeaf<u32, H256, H256, H256>`.
#[derive(Clone, Debug, Encode, Decode)]
pub struct MmrLeaf {
    pub version: MmrLeafVersion,
    pub parent_number_and_hash: (u32, H256),
    pub beefy_next_authority_set: BeefyAuthoritySet,
    pub leaf_extra: H256,
}

/// Mirrors `beefy_verifier_primitives::ParachainHeader`.
#[derive(Clone, Debug, Encode, Decode)]
pub struct ParachainHeader {
    pub header: Vec<u8>,
    pub index: u32,
    pub para_id: u32,
}

/// Mirrors `beefy_verifier_primitives::Sp1BeefyProof`.
#[derive(Clone, Debug, Encode, Decode)]
pub struct Sp1BeefyProof {
    pub block_number: u32,
    pub validator_set_id: u64,
    pub mmr_leaf: MmrLeaf,
    pub headers: Vec<ParachainHeader>,
    pub proof: Vec<u8>,
}

/// Mirrors `beefy_verifier_primitives::ConsensusState`.
#[derive(Clone, Debug, Encode, Decode)]
pub struct ConsensusState {
    pub latest_beefy_height: u32,
    pub beefy_activation_block: u32,
    pub mmr_root_hash: H256,
    pub current_authorities: BeefyAuthoritySet,
    pub next_authorities: BeefyAuthoritySet,
}
