use std::sync::Arc;

use anyhow::anyhow;
use codec::{Decode, Encode};
use primitive_types::H256;
use rs_merkle::MerkleTree;
use sp1_beefy::{Sp1Beefy, SP1_BEEFY};
use sp1_beefy_primitives::{
	AuthoritiesProof, BeefyCommitment, KeccakHasher, MmrLeafProof, ParachainHeader, ParachainProof,
	SignatureWithAuthorityIndex,
};
use sp_consensus_beefy::ecdsa_crypto::Signature;
use sp_crypto_hashing::keccak_256;
use subxt::config::HashFor;

use beefy_prover::util::hash_authority_addresses;
use beefy_verifier_primitives::ConsensusState;
use ismp_solidity_abi::sp1_beefy::Sp1BeefyProof;

// mod plonk;
#[cfg(test)]
mod tests;

/// Consensus prover for zk BEEFY.
#[derive(Clone)]
pub struct Prover<R: subxt::Config, P: subxt::Config> {
	pub inner: beefy_prover::Prover<R, P>,
	pub sp1_beefy: Arc<Sp1Beefy>,
}

impl<R, P> Prover<R, P>
where
	R: subxt::Config,
	P: subxt::Config,
{
	pub fn new(prover: beefy_prover::Prover<R, P>) -> Self {
		Self { inner: prover, sp1_beefy: Arc::new(Sp1Beefy::new(true)) }
	}

	pub async fn consensus_proof(
		&self,
		signed_commitment: sp_consensus_beefy::SignedCommitment<u32, Signature>,
		consensus_state: ConsensusState,
	) -> Result<Sp1BeefyProof, anyhow::Error> {
		let authority = match signed_commitment.commitment.validator_set_id {
			id if id == consensus_state.current_authorities.id =>
				consensus_state.current_authorities,
			id if id == consensus_state.next_authorities.id => consensus_state.next_authorities,
			_ => Err(anyhow::anyhow!(
				"Unknown validator set {}",
				signed_commitment.commitment.validator_set_id
			))?,
		};

		let message = self.inner.consensus_proof(signed_commitment.clone()).await?;

		let num: subxt::ext::subxt_rpcs::methods::legacy::BlockNumber =
			signed_commitment.commitment.block_number.into();
		let block_hash = self
			.inner
			.relay_rpc
			.chain_get_block_hash(Some(num))
			.await?
			.ok_or_else(|| anyhow!("Failed to query blockhash for blocknumber"))?;

		let authorities_witness = {
			let authorities = self.inner.beefy_authorities(Some(block_hash)).await?;
			let leaf_hashes =
				hash_authority_addresses(authorities.into_iter().map(|x| x.encode()).collect())?;
			let indices = message
				.mmr
				.signed_commitment
				.signatures
				.iter()
				.map(|s| s.index as usize)
				.collect::<Vec<_>>();

			let tree = MerkleTree::<KeccakHasher>::from_leaves(&leaf_hashes);
			let proof = tree.proof(&indices);
			proof.proof_hashes().iter().map(|item| item.clone().into()).collect()
		};

		let (para_header_witness, paras_len) = {
			let block_hash = message.mmr.latest_mmr_leaf.parent_number_and_hash.1;
			let paras = beefy_prover::relay::paras_parachains(
				&self.inner.relay_rpc,
				Some(HashFor::<R>::decode(&mut &*block_hash.encode())?),
			)
			.await?;
			let leaf_hashes = paras.iter().map(|l| keccak_256(&l.encode())).collect::<Vec<_>>();
			let tree = MerkleTree::<KeccakHasher>::from_leaves(&leaf_hashes);

			let indices = message.parachain.parachains.iter().map(|i| i.index).collect::<Vec<_>>();
			let proof = tree.proof(&indices);
			let witness = proof.proof_hashes().iter().map(|item| item.clone().into()).collect();

			(witness, paras.len() as u32)
		};

		let commitment = BeefyCommitment {
			authorities: AuthoritiesProof {
				len: authority.len,
				proof: authorities_witness,
				root: authority.keyset_commitment.0.into(),
				votes: message
					.mmr
					.signed_commitment
					.signatures
					.into_iter()
					.map(|i| SignatureWithAuthorityIndex {
						index: i.index,
						signature: i.signature.to_vec(),
					})
					.collect(),
			},
			commitment: message.mmr.signed_commitment.commitment.encode(),
			mmr: MmrLeafProof {
				proof: {
					if signed_commitment.commitment.block_number == 25420895 {
						let mut items = message.mmr.mmr_proof.items;
						if let Some(item) = items.last_mut() {
							*item = H256(hex_literal::hex!(
								"3a1754334582d9352eb0d02ad61d7f163bd52169286ba7c440ab16d253bf9884"
							))
						}
						items.into_iter().map(|item| item.0.into()).collect()
					} else {
						message.mmr.mmr_proof.items.into_iter().map(|item| item.0.into()).collect()
					}
				},
				count: message.mmr.mmr_proof.leaf_count,
				index: message.mmr.mmr_proof.leaf_indices[0],
				leaf: message.mmr.latest_mmr_leaf.encode(),
			},
			parachain: ParachainProof {
				headers: message
					.parachain
					.parachains
					.clone()
					.into_iter()
					.map(|p| ParachainHeader {
						header: p.header,
						para_id: p.para_id,
						index: p.index as u32,
					})
					.collect(),
				proof: para_header_witness,
				total_count: paras_len,
			},
		};

		let proof = self.sp1_beefy.prove(SP1_BEEFY, commitment)?;

		tracing::trace!(target: "zk_beefy", "Plonk Proof: {:#?}", hex::encode(proof.bytes()));
		tracing::trace!(target: "zk_beefy", "Public Inputs: {:#?}", proof.public_values.raw());

		Ok(Sp1BeefyProof {
			commitment: signed_commitment.commitment.into(),
			mmr_leaf: message.mmr.latest_mmr_leaf.into(),
			proof: proof.bytes().into(),
			headers: message.parachain.parachains.into_iter().map(|i| i.into()).collect(),
		})
	}
}
