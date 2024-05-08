mod plonk;
mod prover;

pub use plonk::Network;
pub use prover::*;

use ismp_solidity_abi::beefy::{
	BeefyConsensusProof, BeefyMmrLeaf, Commitment, ParachainProof, RelayChainProof,
};

#[derive(
	Clone,
	ethers::contract::EthAbiType,
	ethers::contract::EthAbiCodec,
	Default,
	Debug,
	PartialEq,
	Eq,
	Hash,
)]
/// Variant of beefy proof that uses plonk
pub struct PlonkProof {
	pub commitment: Commitment,
	pub latest_mmr_leaf: BeefyMmrLeaf,
	pub mmr_proof: Vec<[u8; 32]>,
	pub proof: ethers::core::types::Bytes,
}

#[derive(
	Clone,
	ethers::contract::EthAbiType,
	ethers::contract::EthAbiCodec,
	Default,
	Debug,
	PartialEq,
	Eq,
	Hash,
)]
/// Variant of BEEFY proof that uses plonk
pub struct PlonkConsensusProof {
	pub relay: PlonkProof,
	pub parachain: ParachainProof,
}

impl From<RelayChainProof> for PlonkProof {
	fn from(value: RelayChainProof) -> Self {
		PlonkProof {
			commitment: value.signed_commitment.commitment,
			latest_mmr_leaf: value.latest_mmr_leaf,
			mmr_proof: value.mmr_proof,
			proof: Default::default(),
		}
	}
}

impl From<BeefyConsensusProof> for PlonkConsensusProof {
	fn from(value: BeefyConsensusProof) -> Self {
		PlonkConsensusProof { parachain: value.parachain, relay: value.relay.into() }
	}
}
