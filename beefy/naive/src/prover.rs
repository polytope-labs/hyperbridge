use crate::HostConfig;
use anyhow::anyhow;
use beefy_primitives::{
    ecdsa_crypto::Signature, known_payloads::MMR_ROOT_ID, mmr::BeefyNextAuthoritySet,
};
use beefy_prover::{relay::fetch_latest_beefy_justification, runtime};
use beefy_verifier_primitives::ConsensusState;
use codec::{Decode, Encode};
use ethabi::ethereum_types::H256;
use ethers::abi::AbiEncode;
use ismp_solidity_abi::beefy::BeefyConsensusProof;

use subxt::config::Header;
use tesseract_substrate::SubstrateConfig;

/// Beefy prover, can either produce zk proofs or naive proofs
#[derive(Clone)]
pub enum Prover<R: subxt::Config, P: subxt::Config> {
    // The naive prover
    Naive(beefy_prover::Prover<R, P>),
    // zk prover
    ZK(zk_beefy::Prover<R, P>),
}

impl<R, P> Prover<R, P>
where
    R: subxt::Config,
    P: subxt::Config,
{
    pub async fn new(
        host: &HostConfig,
        substrate: &SubstrateConfig,
    ) -> Result<Self, anyhow::Error> {
        let max_rpc_payload_size = substrate.max_rpc_payload_size.unwrap_or(15 * 1024 * 1024);
        let relay_chain =
            subxt_utils::client::ws_client::<R>(&host.relay_rpc_ws, max_rpc_payload_size).await?;
        let parachain =
            subxt_utils::client::ws_client::<P>(&substrate.rpc_ws, max_rpc_payload_size).await?;

        let header = relay_chain
            .rpc()
            .header(None)
            .await?
            .ok_or_else(|| anyhow!("No blocks on the relay chain?"))?;
        let key = runtime::storage().mmr().number_of_leaves();
        let leaves = relay_chain
            .storage()
            .at(header.hash())
            .fetch(&key)
            .await?
            .ok_or_else(|| anyhow!("Number of mmr leaves is empty"))?;

        let prover = beefy_prover::Prover {
            beefy_activation_block: (header.number().into() - leaves) as u32,
            relay: relay_chain,
            para: parachain,
            para_ids: vec![crate::extract_para_id(substrate.state_machine)?],
        };

        let prover = if let Some(network) = &host.zk_beefy {
            Prover::ZK(zk_beefy::Prover::new(prover, network.clone())?)
        } else {
            Prover::Naive(prover)
        };

        Ok(prover)
    }

    pub fn inner(&self) -> &beefy_prover::Prover<R, P> {
        match self {
            Prover::ZK(p) => &p.inner,
            Prover::Naive(p) => &p,
        }
    }

    /// Construct a beefy client state to be submitted to the counterparty chain
    pub async fn query_initial_consensus_state(
        &self,
        hash: R::Hash,
    ) -> Result<ConsensusState, anyhow::Error> {
        let inner = self.inner();
        // let latest_finalized_head =
        // 	inner.relay.rpc().request("beefy_getFinalizedHead", rpc_params!()).await?;
        let (signed_commitment, latest_beefy_finalized) =
            fetch_latest_beefy_justification(&inner.relay, hash).await?;

        // Encoding and decoding to fix dependency version conflicts
        let next_authority_set = {
            let key = runtime::storage().beefy_mmr_leaf().beefy_next_authorities();
            let next_authority_set = inner
                .relay
                .storage()
                .at(latest_beefy_finalized)
                .fetch(&key)
                .await?
                .expect("Should retrieve next authority set")
                .encode();
            BeefyNextAuthoritySet::decode(&mut &*next_authority_set)
                .expect("Should decode next authority set correctly")
        };

        let current_authority_set = {
            let key = runtime::storage().beefy_mmr_leaf().beefy_authorities();
            let authority_set = inner
                .relay
                .storage()
                .at(latest_beefy_finalized)
                .fetch(&key)
                .await?
                .expect("Should retrieve next authority set")
                .encode();
            BeefyNextAuthoritySet::decode(&mut &*authority_set)
                .expect("Should decode next authority set correctly")
        };

        let mmr_root_hash = signed_commitment
            .commitment
            .payload
            .get_decoded::<H256>(&MMR_ROOT_ID)
            .expect("Mmr root hash should decode correctly");

        let client_state = ConsensusState {
            mmr_root_hash,
            beefy_activation_block: inner.beefy_activation_block,
            latest_beefy_height: signed_commitment.commitment.block_number as u32,
            current_authorities: current_authority_set.clone(),
            next_authorities: next_authority_set.clone(),
        };

        Ok(client_state)
    }

    /// Generate an encoded proof
    pub async fn consensus_proof(
        &self,
        signed_commitment: beefy_primitives::SignedCommitment<u32, Signature>,
        consensus_state: ConsensusState,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let encoded = match self {
            Prover::Naive(naive) => {
                let message: BeefyConsensusProof =
                    naive.consensus_proof(signed_commitment).await?.into();
                message.encode()
            }
            Prover::ZK(zk) => {
                let message = zk
                    .consensus_proof(signed_commitment, consensus_state)
                    .await?;
                message.encode()
            }
        };

        Ok(encoded)
    }
}
