use std::sync::Arc;

use anyhow::anyhow;
use codec::{Decode, Encode};
use primitive_types::H256;
use rs_merkle::MerkleTree;
pub use sp1_beefy::BeefyProver;
use sp1_beefy_primitives::{
	AuthoritiesProof, BeefyCommitment, KeccakHasher, MmrLeafProof, ParachainHeader, ParachainProof,
	SignatureWithAuthorityIndex,
};
use sp_consensus_beefy::ecdsa_crypto::Signature;
use sp_crypto_hashing::keccak_256;
use subxt::config::HashFor;

use beefy_prover::util::hash_authority_addresses;
use beefy_verifier_primitives::ConsensusState;
use ismp_abi::sp1_beefy::SP1BeefyProof;

#[cfg(feature = "cluster")]
pub use sp1_beefy::cluster::ClusterProver;

#[cfg(any(feature = "local", test))]
pub use sp1_beefy::local::LocalProver;

// mod plonk;
#[cfg(test)]
mod tests;

/// Consensus prover for zk BEEFY.
pub struct Prover<R: subxt::Config, P: subxt::Config, B: BeefyProver> {
	pub inner: beefy_prover::Prover<R, P>,
	pub sp1_beefy: Arc<B>,
	/// The extrinsic submission account (the `submit_proof` signer). Committed verbatim into
	/// each SP1 proof as its nonce, so `pallet-beefy-consensus-proofs` can bind the proof to
	/// this account and reject it if submitted by anyone else.
	pub account: H256,
}

impl<R, P, B> Clone for Prover<R, P, B>
where
	R: subxt::Config,
	P: subxt::Config,
	B: BeefyProver,
	beefy_prover::Prover<R, P>: Clone,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			sp1_beefy: self.sp1_beefy.clone(),
			account: self.account,
		}
	}
}

impl<R, P, B> Prover<R, P, B>
where
	R: subxt::Config,
	P: subxt::Config,
	B: BeefyProver,
{
	pub fn new(prover: beefy_prover::Prover<R, P>, sp1_beefy: B, account: H256) -> Self {
		Self { inner: prover, sp1_beefy: Arc::new(sp1_beefy), account }
	}

	pub async fn consensus_proof(
		&self,
		signed_commitment: sp_consensus_beefy::SignedCommitment<u32, Signature>,
		consensus_state: ConsensusState,
	) -> Result<SP1BeefyProof, anyhow::Error> {
		// Submission account committed into the proof as its nonce (see struct field docs).
		// Use the raw bytes at each commit site so the conversion is agnostic to the exact
		// `H256`/`FixedBytes` type each consumer expects.
		let account: [u8; 32] = self.account.0;
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

			let indices = message
				.parachain
				.parachains
				.iter()
				.map(|i| i.index as usize)
				.collect::<Vec<_>>();
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
			// Commit our submission account as the proof's nonce. `pallet-beefy-consensus-proofs`
			// requires this to equal the `submit_proof` signer, binding the proof to us so it
			// can't be sniped from the mempool and submitted under another account.
			nonce: account.into(),
		};

		let proof = self.sp1_beefy.prove(commitment).await?;

		tracing::trace!(target: "zk_beefy", "Plonk Proof: {:#?}", hex::encode(proof.bytes()));
		tracing::trace!(target: "zk_beefy", "Public Inputs: {:#?}", proof.public_values.raw());

		Ok(SP1BeefyProof {
			commitment: signed_commitment.commitment.into(),
			mmrLeaf: message.mmr.latest_mmr_leaf.into(),
			proof: proof.bytes().into(),
			headers: message.parachain.parachains.into_iter().map(|i| i.into()).collect(),
			// Carry the committed nonce so the verifier reconstructs matching public inputs.
			nonce: account.into(),
		})
	}
}
