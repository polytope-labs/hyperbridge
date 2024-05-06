use crate::{
	plonk,
	plonk::{PublicKey, Sibling, Vote},
	Network, PlonkConsensusProof,
};
use anyhow::anyhow;
use beefy_primitives::ecdsa_crypto::Signature;
use beefy_prover::runtime;
use beefy_verifier_primitives::{ConsensusMessage, ConsensusState};
use codec::Encode;
use ethers::prelude::H160;
use ismp_solidity_abi::beefy::BeefyConsensusProof;
use sp_core::{keccak_256, H256};

/// Consensus prover for zk BEEFY.
#[derive(Clone)]
pub struct Prover<R: subxt::Config, P: subxt::Config> {
	pub inner: beefy_prover::Prover<R, P>,
	pub plonk: plonk::Prover,
}

impl<R, P> Prover<R, P>
where
	R: subxt::Config,
	P: subxt::Config,
{
	pub fn new(prover: beefy_prover::Prover<R, P>, chain: Network) -> Result<Self, anyhow::Error> {
		Ok(Self { inner: prover, plonk: plonk::Prover::new(chain)? })
	}

	pub async fn consensus_proof(
		&self,
		signed_commitment: beefy_primitives::SignedCommitment<u32, Signature>,
		consensus_state: ConsensusState,
	) -> Result<PlonkConsensusProof, anyhow::Error> {
		let message = self.inner.consensus_proof(signed_commitment.clone()).await?;
		let proof = self
			.prove_consensus(consensus_state, message.clone(), signed_commitment)
			.await?;
		let mut plonk_consensus_proof: PlonkConsensusProof =
			BeefyConsensusProof::from(message).into();
		plonk_consensus_proof.relay.proof = proof.into();

		Ok(plonk_consensus_proof)
	}

	pub async fn prove_consensus(
		&self,
		state: ConsensusState,
		msg: ConsensusMessage,
		signed_commitment: beefy_primitives::SignedCommitment<u32, Signature>,
	) -> Result<Vec<u8>, anyhow::Error> {
		let relay = msg.mmr;

		let msg_hash = keccak_256(&signed_commitment.commitment.encode());
		let msg = plonk::to_field_element(&msg_hash);

		let root = {
			match relay.signed_commitment.commitment.validator_set_id {
				id if id == state.current_authorities.id =>
					plonk::to_field_element(&state.current_authorities.keyset_commitment.0),
				id if id == state.next_authorities.id =>
					plonk::to_field_element(&state.next_authorities.keyset_commitment.0),
				id => Err(anyhow!(
					"Unknown authority set with id: {id}, current: {}, next: {}",
					state.current_authorities.id,
					state.next_authorities.id
				))?,
			}
		};

		let subxt_block_number: subxt::rpc::types::BlockNumber =
			(signed_commitment.commitment.block_number - 1).into();
		let block_hash = self
			.inner
			.relay
			.rpc()
			.block_hash(Some(subxt_block_number))
			.await?
			.ok_or_else(|| anyhow!("Failed to query blockhash for blocknumber"))?;

		let authorities = {
			let key = runtime::storage().beefy().authorities();
			self.inner
				.relay
				.storage()
				.at(block_hash)
				.fetch(&key)
				.await?
				.ok_or_else(|| anyhow!("No beefy authorities found!"))?
				.0
		};

		let siblings = signed_commitment
			.signatures
			.into_iter()
			.zip(authorities)
			.enumerate()
			.filter_map(|(i, (item, public))| match item {
				None => {
					let public = libsecp256k1::PublicKey::parse_compressed(&public.0 .0)
						.unwrap()
						.serialize();
					let address = keccak_256(&public[1..])[12..].to_vec();
					let pre_hash = keccak_256(&address);
					let [lo, hi] = plonk::to_field_element(&pre_hash);

					Some(Sibling { hash: [lo, hi], index: H160::from_low_u64_be(i as u64) })
				},
				Some(_) => None,
			})
			.collect::<Vec<_>>();

		let votes = relay
			.signed_commitment
			.signatures
			.into_iter()
			.map(|sig| {
				let Ok(public) = sp_io::crypto::secp256k1_ecdsa_recover(&sig.signature, &msg_hash)
				else {
					panic!("Signature should be valid; qed");
				};
				let (x, y) = public.split_at(32);
				let (x, y) = (H256::from_slice(x), H256::from_slice(y));

				let (r, s) = sig.signature[..64].split_at(32);
				let (r, s) = (H256::from_slice(r), H256::from_slice(s));
				let [a0, a1] = plonk::to_field_element(&r.0);
				let [a2, a3] = plonk::to_field_element(&s.0);

				Vote {
					key: PublicKey {
						x: plonk::to_field_element(&x.0),
						y: plonk::to_field_element(&y.0),
					},
					signature: [a0, a1, a2, a3],
					index: H160::from_low_u64_be(sig.index as u64),
				}
			})
			.collect::<Vec<_>>();

		let params = plonk::ProverParams { votes, siblings, msg, root };
		let plonk_prover = Clone::clone(&self.plonk);

		let handle = tokio::task::spawn_blocking(move || plonk_prover.prove(params));

		let proof = handle.await??;

		Ok(proof)
	}
}
