// Copyright (C) Polytope Labs Ltd.
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

#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

use anyhow::anyhow;
use codec::{Decode, Encode};
use cumulus_relay_chain_interface::RelayChainInterface;
use polkadot_sdk::*;
use sp_api::ApiExt;
use sp_runtime::{
	generic::{BlockId, Header},
	traits::{BlakeTwo256, Block as BlockT},
};
use std::sync::Arc;

use ismp::{consensus::StateMachineId, host::StateMachine, messaging::ConsensusMessage};
use ismp_parachain::{
	consensus::{parachain_header_storage_key, ParachainConsensusProof},
	parachain_consensus_state_id,
};
use ismp_parachain_runtime_api::IsmpParachainApi;
use pallet_ismp_runtime_api::IsmpRuntimeApi;

/// Implements [`InherentDataProvider`](sp_inherents::InherentDataProvider) for providing parachain
/// consensus updates as inherents.
pub struct ConsensusInherentProvider(Option<ConsensusMessage>);

impl ConsensusInherentProvider {
	/// Create the [`ConsensusMessage`] for the latest height. Will be [`None`] if no para ids have
	/// been configured.
	pub async fn create<C, B>(
		parent: B::Hash,
		client: Arc<C>,
		relay_chain_interface: Arc<dyn RelayChainInterface>,
	) -> Result<ConsensusInherentProvider, anyhow::Error>
	where
		C: sp_api::ProvideRuntimeApi<B> + sp_blockchain::HeaderBackend<B>,
		C::Api: IsmpParachainApi<B> + IsmpRuntimeApi<B, B::Hash>,
		B: BlockT,
	{
		// Check if it has the parachain runtime api
		if !client.runtime_api().has_api::<dyn IsmpParachainApi<B>>(parent)? {
			log::trace!("IsmpParachainApi not implemented");
			return Ok(ConsensusInherentProvider(None));
		}

		let para_ids = client.runtime_api().para_ids(parent)?;

		log::trace!("ParaIds from runtime: {para_ids:?}");

		if para_ids.is_empty() {
			return Ok(ConsensusInherentProvider(None));
		}

		let state = client.runtime_api().current_relay_chain_state(parent)?;
		log::trace!("Current relay chain state: {state:?}");

		// parachain is just starting
		if state.number == 0u32 {
			return Ok(ConsensusInherentProvider(None));
		}

		let relay_header = if let Ok(Some(header)) =
			relay_chain_interface.header(BlockId::Number(state.number)).await
		{
			header
		} else {
			log::trace!("Relay chain header not available for {}", state.number);
			return Ok(ConsensusInherentProvider(None));
		};

		let host_state_machine = client.runtime_api().host_state_machine(parent)?;
		let mut para_ids_to_fetch = vec![];
		for id in para_ids {
			let Some(head) = relay_chain_interface
				.get_storage_by_key(relay_header.hash(), parachain_header_storage_key(id).as_ref())
				.await?
			else {
				log::trace!("Failed to fetch parachain header for {id} from relay chain");
				continue;
			};

			let Ok(intermediate) = Vec::<u8>::decode(&mut &head[..]) else {
				continue;
			};

			let Ok(header) = Header::<u32, BlakeTwo256>::decode(&mut &intermediate[..]) else {
				continue;
			};

			let state_id = match host_state_machine {
				StateMachine::Polkadot(_) => StateMachine::Polkadot(id),
				StateMachine::Kusama(_) => StateMachine::Kusama(id),
				id => Err(anyhow!("Unsupported state machine: {id:?}"))?,
			};
			let height = client
				.runtime_api()
				.latest_state_machine_height(
					parent,
					StateMachineId {
						consensus_state_id: parachain_consensus_state_id(host_state_machine),
						state_id,
					},
				)?
				.unwrap_or_default();

			if height >= header.number as u64 {
				log::trace!("Skipping stale height {height} for parachain {id}");
				continue;
			}

			para_ids_to_fetch.push(id);
		}

		if para_ids_to_fetch.is_empty() {
			return Ok(ConsensusInherentProvider(None));
		}

		let keys = para_ids_to_fetch.iter().map(|id| parachain_header_storage_key(*id).0).collect();
		let storage_proof = relay_chain_interface
			.prove_read(relay_header.hash(), &keys)
			.await?
			.into_iter_nodes()
			.collect();

		let consensus_proof = ParachainConsensusProof { relay_height: state.number, storage_proof };
		let message = ConsensusMessage {
			consensus_state_id: parachain_consensus_state_id(host_state_machine),
			consensus_proof: consensus_proof.encode(),
			signer: Default::default(),
		};

		Ok(ConsensusInherentProvider(Some(message)))
	}
}

#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for ConsensusInherentProvider {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut sp_inherents::InherentData,
	) -> Result<(), sp_inherents::Error> {
		if let Some(ref message) = self.0 {
			inherent_data.put_data(ismp_parachain::INHERENT_IDENTIFIER, message)?;
		}

		Ok(())
	}

	async fn try_handle_error(
		&self,
		_: &sp_inherents::InherentIdentifier,
		_: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		None
	}
}
