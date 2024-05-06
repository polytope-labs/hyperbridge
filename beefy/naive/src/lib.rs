// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::anyhow;
use beefy_prover::{
    relay::{fetch_latest_beefy_justification, fetch_next_beefy_justification},
    runtime::{self},
};
use beefy_verifier_primitives::ConsensusState;
use codec::Decode;
use ismp::{consensus::ConsensusStateId, host::StateMachine, messaging::ConsensusMessage};
use prover::Prover;
use serde::{Deserialize, Serialize};
use sp_core::H160;
use sp_runtime::traits::Keccak256;
use std::{sync::Arc, time::Duration};
use subxt::{config::Header, ext::sp_runtime::traits::Zero};
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::SubstrateConfig;
use tokio::{sync::broadcast, time};
pub use zk_beefy::Network;

mod byzantine;
mod host;
mod prover;

const VALIDATOR_SET_ID_KEY: [u8; 32] =
    hex_literal::hex!("08c41974a97dbf15cfbec28365bea2da8f05bccc2f70ec66a32999c5761156be");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    /// RPC ws url for a relay chain
    pub relay_rpc_ws: String,
    /// Interval in seconds at which consensus updates should happen
    pub consensus_update_frequency: u64,
    /// The intended network for zk beefy
    pub zk_beefy: Option<Network>,
}

#[derive(Clone)]
pub struct BeefyHost<R, P>
where
    R: subxt::Config,
    P: subxt::Config,
{
    /// Consensus state id on counterparty chain
    pub consensus_state_id: ConsensusStateId,
    /// Grandpa prover
    pub prover: Prover<R, P>,
    /// Minimum time after which a mandatory update must be sent to the counterparty
    /// Should always be less than the epoch duration
    pub consensus_update_interval: Duration,
    /// Since this host is clone-able, we spawn a single background prover task
    /// which produces proofs for all instances of the host.
    /// Sends the consensus message and validator set id who signed the update
    pub sender: broadcast::Sender<(ConsensusMessage, u32, u64)>,
    /// Host Config
    pub host: HostConfig,
    /// a reference to [`IsmpProvider`] for substrate-based chains
    pub provider: Arc<dyn IsmpProvider>,
}

impl<R, P> BeefyHost<R, P>
where
    R: subxt::Config + Send + Sync + Clone,
    P: subxt::Config + Send + Sync + Clone,

    <R::Header as Header>::Number: Ord + Zero,
    u32: From<<R::Header as Header>::Number>,
    sp_core::H256: From<R::Hash>,
    R::Header: codec::Decode,

    <P::Header as Header>::Number: Ord + Zero,
    u32: From<<P::Header as Header>::Number>,
    sp_core::H256: From<P::Hash>,
    P::Header: codec::Decode,
{
    pub async fn new(
        host: &HostConfig,
        substrate: &SubstrateConfig,
        provider: Arc<dyn IsmpProvider>,
    ) -> Result<Self, anyhow::Error> {
        let prover = Prover::new(host, substrate).await?;

        let (sender, _receiver) = broadcast::channel(10);

        let host = BeefyHost {
            // Beefy is the only consensus client on the counterparty
            consensus_state_id: Default::default(),
            consensus_update_interval: Duration::from_secs(host.consensus_update_frequency),
            prover,
            sender,
            host: host.clone(),
            provider,
        };

        Ok(host)
    }

    /// Spawns a prover task, using the counterparty as a source of consensus states
    pub async fn spawn_prover(
        &self,
        counterparties: Vec<Arc<dyn IsmpProvider>>,
    ) -> Result<(), anyhow::Error> {
        let clone_counterparties = counterparties.clone();
        let clone_client = self.clone();
        let mut interval =
            time::interval(Duration::from_secs(self.host.consensus_update_frequency));

        let mut old_consensus_state = highest_consensus_state(&counterparties).await?;

        tokio::spawn(async move {
            let mut started = false;
            let mut syncing = false;
            let mut wait_count = 0;
            loop {
                let new_consensus_state = {
                    match highest_consensus_state(&clone_counterparties).await {
                        Ok(c) => c,
                        Err(err) => {
                            log::error!("zkBeefy prover could not fetch consensus state: {err:?}");
                            continue;
                        }
                    }
                };

                // if the consensus message fails on this counterparty, we will never produce new
                // proofs
                if new_consensus_state.latest_beefy_height
                    <= old_consensus_state.latest_beefy_height
                    && started
                {
                    log::warn!("😴 Consensus state not yet updated, sleeping");
                    // sleep for a bit
                    time::sleep(Duration::from_secs(10)).await;
                    // after a while, the tx has most likely been cancelled.
                    // proceed with proof production
                    if wait_count <= 12 {
                        continue;
                    } else {
                        // we've now waited for a total of 2 minutes.
                        // proceeding with proof generation
                        wait_count = 0;
                    }
                } else {
                    started = true;
                }

                // any pending sync messages?
                match clone_client.sync(&new_consensus_state).await {
                    Ok(Some(message)) => {
                        log::info!("⏩  Sending sync consensus message");
                        syncing = true;
                        // send, if no one is listening, abort
                        if clone_client.sender.send(message).is_err() {
                            log::error!("All proof consumers dropped, dropping prover.");
                            break;
                        }

                        // update the consensus state optimistically
                        old_consensus_state = new_consensus_state;
                        continue;
                    }
                    Ok(None) => {
                        if syncing {
                            log::info!("🔄 Consensus sync completed");
                        }
                        syncing = false;
                    }
                    Err(err) => {
                        log::error!("zkBeefy prover encountered an error in sync: {err:?}");
                        continue;
                    }
                };

                // now we can wait
                interval.tick().await;

                // fetch latest justification as usual
                let result = fetch_next_beefy_justification::<R>(
                    &clone_client.prover.inner().relay,
                    new_consensus_state.latest_beefy_height.into(),
                    new_consensus_state.current_authorities.id,
                )
                .await;

                let signed_commitment = match result {
                    Ok(Some((s, _hash))) => s,
                    Ok(None) => {
                        log::error!("Error fetching commitment");
                        continue;
                    }
                    Err(err) => {
                        log::error!(
                            "zkBeefy prover encountered an error fetching justification: {err:?}"
                        );
                        continue;
                    }
                };

                if let Ok(para_height) = clone_client
                    .query_parachain_height(signed_commitment.commitment.block_number)
                    .await
                {
                    log::info!("⚙️ Generating zkBeefy consensus proof for parachain block height: {para_height}");
                }

                let latest_height = signed_commitment.commitment.block_number;
                let set_id = signed_commitment.commitment.validator_set_id;

                let consensus_proof = match clone_client
                    .prover
                    .consensus_proof(signed_commitment, new_consensus_state.clone())
                    .await
                {
                    Ok(proof) => proof,
                    Err(err) => {
                        // log errror
                        log::error!("zkBeefy prover encountered an error generating consensus proof: {err:?}");
                        continue;
                    }
                };

                let message = ConsensusMessage {
                    consensus_proof,
                    consensus_state_id: clone_client.consensus_state_id.clone(),
                    signer: H160::random().0.to_vec(),
                };

                // send, if no one is listening, abort
                if clone_client
                    .sender
                    .send((message, latest_height, set_id))
                    .is_err()
                {
                    log::error!("All proof consumers dropped, terminating zkBeefy prover.");
                    break;
                }

                old_consensus_state = new_consensus_state;
            }
        });

        Ok(())
    }

    /// Next epoch justification
    pub async fn next_epoch_justification(&self, start: u64) -> anyhow::Result<Option<R::Hash>> {
        for i in start..=(start + 50) {
            let hash = if let Some(hash) = self
                .prover
                .inner()
                .relay
                .rpc()
                .block_hash(Some(i.into()))
                .await?
            {
                hash
            } else {
                continue;
            };

            if let Some(justifications) = self
                .prover
                .inner()
                .relay
                .rpc()
                .block(Some(hash))
                .await?
                .ok_or_else(|| anyhow!("failed to find block for {hash:?}"))?
                .justifications
            {
                let beefy = justifications
                    .into_iter()
                    .find(|justfication| justfication.0 == beefy_primitives::BEEFY_ENGINE_ID);

                if beefy.is_some() {
                    return Ok(Some(hash));
                }
            }
        }
        Ok(None)
    }

    /// Query the parachain height that is finalized at the given relay chain height
    pub async fn query_parachain_height(&self, relay_height: u32) -> Result<u32, anyhow::Error> {
        let hash = self
            .prover
            .inner()
            .relay
            .rpc()
            .block_hash(Some((relay_height - 1).into()))
            .await?
            .ok_or_else(|| anyhow!("Request relay chain block height {relay_height} not found"))?;
        let para_id = extract_para_id(self.provider.state_machine_id().state_id)?;
        let head_data = self
			.prover
			.inner()
			.relay
			.storage()
			.at(hash)
			.fetch(&runtime::storage().paras().heads(
				&runtime::runtime_types::polkadot_parachain_primitives::primitives::Id(para_id),
			))
			.await?
			.ok_or_else(|| {
				anyhow!("Could not fetch header for parachain with id {para_id} at block height {relay_height}")
			})?;

        let header = sp_runtime::generic::Header::<u32, Keccak256>::decode(&mut &head_data.0[..])?;

        Ok(header.number)
    }

    /// attempts to construct a proof for the next epoch
    pub async fn sync(
        &self,
        consensus_state: &ConsensusState,
    ) -> Result<Option<(ConsensusMessage, u32, u64)>, anyhow::Error> {
        let latest = self
            .prover
            .inner()
            .relay
            .rpc()
            .header(None)
            .await?
            .ok_or_else(|| anyhow!("Error syncing beefy client"))?;
        let latest_set_id = self
            .prover
            .inner()
            .relay
            .storage()
            .at(latest.hash())
            .fetch(&runtime::storage().beefy().validator_set_id())
            .await?
            .ok_or_else(|| anyhow!("Couldn't fetch latest beefy authority set"))?;
        let current_set_id = consensus_state.current_authorities.id;

        if !(current_set_id..=(current_set_id + 1)).contains(&latest_set_id) {
            let from = self
                .prover
                .inner()
                .relay
                .rpc()
                .block_hash(Some((consensus_state.latest_beefy_height).into()))
                .await?
                .ok_or_else(|| anyhow!("Block hash should exist"))?;

            let changes = self
                .prover
                .inner()
                .relay
                .rpc()
                .query_storage(vec![&VALIDATOR_SET_ID_KEY[..]], from, Some(latest.hash()))
                .await?;
            let block_hash_and_set_id = changes
                .iter()
                .filter_map(|change| {
                    change.changes[0]
                        .clone()
                        .1
                        .and_then(|data| u64::decode(&mut &*data.0).ok())
                        .map(|id| (change.block, id))
                })
                .filter(|(_, set_id)| *set_id >= consensus_state.next_authorities.id)
                .collect::<Vec<_>>();

            if let Some((block_hash, _)) = block_hash_and_set_id.iter().next() {
                let header = self
                    .prover
                    .inner()
                    .relay
                    .rpc()
                    .header(Some(*block_hash))
                    .await?
                    .ok_or_else(|| anyhow!("Block hash should exist"))?;
                let start: u64 = header.number().into();
                let block_hash =
                    if let Some(hash) = self.next_epoch_justification(start + 1).await? {
                        hash
                    } else {
                        // Sync has reached latest validator set
                        return Ok(None);
                    };
                let (signed_commitment, ..) =
                    fetch_latest_beefy_justification::<R>(&self.prover.inner().relay, block_hash)
                        .await?;
                let latest_height = signed_commitment.commitment.block_number;
                let para_height = self.query_parachain_height(latest_height).await?;
                log::info!(
					"⚙️ Generating zkBeefy consensus proof for parachain block height {para_height}, relay height: {}",
					consensus_state.latest_beefy_height
				);
                let set_id = signed_commitment.commitment.validator_set_id;
                let consensus_proof = self
                    .prover
                    .consensus_proof(signed_commitment, consensus_state.clone())
                    .await?;

                let message = ConsensusMessage {
                    consensus_proof,
                    consensus_state_id: self.consensus_state_id.clone(),
                    signer: H160::random().0.to_vec(),
                };

                return Ok(Some((message, latest_height, set_id)));
            }
        }

        Ok(None)
    }
}

pub(crate) fn extract_para_id(state_machine: StateMachine) -> Result<u32, anyhow::Error> {
    let para_id = match state_machine {
        StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id,
        _ => Err(anyhow!("Invalid state machine: {state_machine:?}"))?,
    };

    Ok(para_id)
}

async fn highest_consensus_state(
    clients: &[Arc<dyn IsmpProvider>],
) -> Result<ConsensusState, anyhow::Error> {
    let mut consensus_states = vec![];
    for client in clients {
        match client
            .query_consensus_state(None, client.state_machine_id().consensus_state_id.clone())
            .await
        {
            Ok(cs_state) => {
                let consensus_state = ConsensusState::decode(&mut &cs_state[..])
                    .expect("Consensus state should always decode correctly");
                consensus_states.push(consensus_state);
            }

            Err(_) => {
                log::error!(
                    "Failed to fetch consensus state for {:?} in beefy prover",
                    client.state_machine_id().state_id
                )
            }
        }
    }

    let max = consensus_states
        .into_iter()
        .max_by(|a, b| a.latest_beefy_height.cmp(&b.latest_beefy_height))
        .ok_or_else(|| anyhow!("No consensus state found for all clients"))?;
    Ok(max)
}
