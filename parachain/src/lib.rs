use crate::{
    parachain::api::ismp_parachain::events::NewRelayChainState,
    relay_chain::runtime_types::polkadot_parachain::primitives::Id,
};
use anyhow::anyhow;
use codec::{Decode, Encode};
use futures::{stream, Stream};
use ismp::{consensus_client::StateMachineId, host::StateMachine, messaging::ConsensusMessage};
use ismp_parachain::consensus::ParachainConsensusProof;
use std::pin::Pin;
use subxt::{
    config::Header as _,
    ext::sp_runtime::{generic::Header, traits::BlakeTwo256},
    OnlineClient, PolkadotConfig,
};
use tesseract_primitives::IsmpHost;

mod byzantine;
mod codegen;
mod host;
mod provider;

pub use codegen::*;

pub struct ParachainConfig {
    /// The host state machine for the parachain consensus client
    pub host: StateMachine,
    /// State machine Identifier for this client.
    pub state_machine: StateMachine,
    /// RPC url for the relay chain. Unneeded if the host is a parachain.
    pub relay_chain: String,
    /// RPC url for the parachain
    pub parachain: String,
}

#[derive(Clone)]
pub struct ParachainClient<T: subxt::Config> {
    /// The host state machine for the parachain consensus client
    host: StateMachine,
    /// State machine Identifier for this client.
    pub state_machine: StateMachine,
    /// Subxt client for the relay chain. Unneeded if the host is a parachain.
    relay_chain: OnlineClient<PolkadotConfig>,
    /// Subxt client for the parachain.
    parachain: OnlineClient<T>,
}

impl<T> ParachainClient<T>
where
    T: subxt::Config + Send + Sync + Clone,
    T::Header: Send + Sync,
{
    pub async fn consensus_notifications<C>(
        &self,
        counterparty: C,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ConsensusMessage, anyhow::Error>> + Send + 'static>>,
        anyhow::Error,
    >
    where
        C: IsmpHost + Clone + 'static,
    {
        let client = ParachainClient::clone(&self);

        let stream = stream::try_unfold((), move |state| {
            let client = client.clone();
            let counterparty = counterparty.clone();

            async move {
                match client.host {
                    StateMachine::Polkadot(id) | StateMachine::Kusama(id) => {
                        // we know there's no challenge period
                        let mut subscription =
                            client.parachain.rpc().subscribe_best_block_headers().await?;

                        while let Some(Ok(header)) = subscription.next().await {
                            let events = client.parachain.events().at(header.hash()).await?;

                            let NewRelayChainState { height: relay_height } =
                                match events.find_first::<NewRelayChainState>()? {
                                    Some(s) => s,
                                    None => continue,
                                };

                            let relay_block_hash = client
                                .relay_chain
                                .rpc()
                                .block_hash(Some(relay_height.into()))
                                .await?
                                .ok_or_else(|| {
                                    anyhow!(
                                        "Can't find relay chain block for height {relay_height}"
                                    )
                                })?;

                            let key = relay_chain::storage().paras().heads(Id(id));
                            let header_bytes = client
                                .relay_chain
                                .storage()
                                .at(relay_block_hash)
                                .fetch(&key)
                                .await?
                                .ok_or_else(|| {
                                    anyhow!(
                                        "Parachain with ParaId({id}) not found on the relay chain"
                                    )
                                })?
                                .0;
                            let header = Header::<u32, BlakeTwo256>::decode(&mut &*header_bytes)?;

                            let latest_height = counterparty
                                .query_latest_state_machine_height(StateMachineId {
                                    state_id: client.state_machine,
                                    consensus_client:
                                        ismp_parachain::consensus::PARACHAIN_CONSENSUS_ID,
                                })
                                .await?;

                            // check header height
                            if header.number <= latest_height {
                                continue
                            }

                            let full_key = client.relay_chain.storage().address_bytes(&key)?;
                            let storage_proof = client
                                .relay_chain
                                .rpc()
                                .read_proof(vec![full_key.as_slice()], Some(relay_block_hash))
                                .await?
                                .proof
                                .into_iter()
                                .map(|b| b.0)
                                .collect();

                            let proof = ParachainConsensusProof {
                                para_ids: vec![id],
                                relay_height,
                                storage_proof,
                            };

                            let message = ConsensusMessage {
                                consensus_proof: proof.encode(),
                                consensus_client_id:
                                    ismp_parachain::consensus::PARACHAIN_CONSENSUS_ID,
                            };

                            return Ok(Some((message, state)))
                        }
                    }
                    state_machine => panic!("Unsupported state machine: {state_machine:?}"),
                };

                return Ok(None)
            }
        });

        Ok(Box::pin(stream))
    }
}
