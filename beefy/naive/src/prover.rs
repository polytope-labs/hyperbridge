use std::time::Duration;

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

use futures::stream;
use serde::{Deserialize, Serialize};
use subxt::config::Header;
use tesseract_substrate::SubstrateConfig;
use zk_beefy::Network;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeefyProverConfig {
    /// RPC ws url for a relay chain
    pub relay_rpc_ws: String,
    /// The intended network for zk beefy
    pub zk_beefy: Option<Network>,
}

/// Beefy prover, can either produce zk proofs or naive proofs
#[derive(Clone)]
pub enum Prover<R: subxt::Config, P: subxt::Config> {
    // The naive prover
    Naive(beefy_prover::Prover<R, P>),
    // zk prover
    ZK(zk_beefy::Prover<R, P>),
}

struct BeefyProver<R: subxt::Config, P: subxt::Config> {
    consensus_state: ConsensusState,
    prover: Prover<R, P>,
}

impl<R, P> BeefyProver<R, P>
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

        // todo: hydrate consensus state from redis
        let consensus_state = Default::default();

        Ok(BeefyProver {
            consensus_state,
            prover,
        })
    }

    pub fn inner(&self) -> &beefy_prover::Prover<R, P> {
        match self.prover {
            Prover::ZK(ref p) => p.inner,
            Prover::Naive(ref p) => p,
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
        let encoded = match self.prover {
            Prover::Naive(ref naive) => {
                let message: BeefyConsensusProof =
                    naive.consensus_proof(signed_commitment).await?.into();
                message.encode()
            }
            Prover::ZK(ref zk) => {
                let message = zk
                    .consensus_proof(signed_commitment, consensus_state)
                    .await?;
                message.encode()
            }
        };

        Ok(encoded)
    }

    /// Return a stream of latest ISMP events
    pub fn ismp_events_stream(&self) ->  {
        let initial_height = self.consensus_state.latest_beefy_height;
        let interval = Duration::from_secs(12);
        let stream = stream::unfold(
            (initial_height, interval, self_clone),
            move |(latest_height, mut interval, client)| async move {
                interval.tick().await;
                let header = match client.client.rpc().finalized_head().await {
                    Ok(hash) => match client.client.rpc().header(Some(hash)).await {
                        Ok(Some(header)) => header,
                        _ => {
                            return Some((
                                Err(anyhow!("Error encountered while fething finalized head")),
                                (latest_height, interval, client),
                            ))
                        }
                    },
                    Err(err) => {
                        return Some((
                            Err(anyhow!(
                                "Error encountered while fetching finalized head: {err:?}"
                            )),
                            (latest_height, interval, client),
                        ))
                    }
                };

                if header.number().into() <= latest_height {
                    return Some((Ok(None), (latest_height, interval, client)));
                }

                let event = StateMachineUpdated {
                    state_machine_id: client.state_machine_id(),
                    latest_height: header.number().into(),
                };

                let events = match client.query_ismp_events(latest_height, event).await {
                    Ok(e) => e,
                    Err(err) => {
                        return Some((
                            Err(anyhow!(
                                "Error encountered while querying ismp events {err:?}"
                            )),
                            (latest_height, interval, client),
                        ))
                    }
                };

                let event = events
                    .into_iter()
                    .filter_map(|event| match event {
                        Event::StateMachineUpdated(e)
                            if e.state_machine_id == counterparty_state_id =>
                        {
                            Some(e)
                        }
                        _ => None,
                    })
                    .max_by(|x, y| x.latest_height.cmp(&y.latest_height));

                let value = match event {
                    Some(event) => {
                        Some((Ok(Some(event)), (header.number().into(), interval, client)))
                    }
                    None => Some((Ok(None), (header.number().into(), interval, client))),
                };

                return value;
            },
        )
        .filter_map(|res| async move {
            match res {
                Ok(Some(update)) => Some(Ok(update)),
                Ok(None) => None,
                Err(err) => Some(Err(err)),
            }
        });

        Ok(Box::pin(stream))
    }
}
