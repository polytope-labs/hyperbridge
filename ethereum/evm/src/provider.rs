use crate::{
	abi::{beefy::BeefyConsensusState, IIsmpHost, StateMachineUpdatedFilter},
	tx::submit_messages,
	EvmClient,
};
use anyhow::{anyhow, Error};
use beefy_verifier_primitives::{BeefyNextAuthoritySet, ConsensusState};
use codec::Encode;
use consensus_client::types::EvmStateProof;
use ethers::{abi::AbiDecode, providers::Middleware};
use futures::stream::StreamExt;
use ismp::{
	consensus::{ConsensusStateId, StateMachineId},
	events::Event,
	messaging::Message,
	router::Get,
};
use jsonrpsee::{
	core::{client::SubscriptionClientT, params::ObjectParams, traits::ToRpcParams},
	rpc_params,
};

use ethereum_trie::StorageProof;
use sp_core::{H160, H256};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tesseract_primitives::{BoxStream, IsmpHost, IsmpProvider, Query, StateMachineUpdated};

#[async_trait::async_trait]
impl<I: IsmpHost> IsmpProvider for EvmClient<I>
where
	I: Send + Sync,
{
	async fn query_consensus_state(
		&self,
		at: Option<u64>,
		_: ConsensusStateId,
	) -> Result<Vec<u8>, Error> {
		let contract = IIsmpHost::new(self.ismp_host, self.client.clone());
		let value = {
			let call = if let Some(block) = at {
				contract.consensus_state().block(block)
			} else {
				contract.consensus_state()
			};
			call.call().await?
		};

		let beefy_consensus_state = BeefyConsensusState::decode(&value.0)?;
		// Convert this bytes into BeefyConsensusState for rust and scale encode
		let consensus_state = ConsensusState {
			latest_beefy_height: beefy_consensus_state.latest_height.as_u32(),
			mmr_root_hash: Default::default(),
			beefy_activation_block: beefy_consensus_state.beefy_activation_block.as_u32(),
			current_authorities: BeefyNextAuthoritySet {
				id: beefy_consensus_state.current_authority_set.id.as_u64(),
				len: beefy_consensus_state.current_authority_set.len.as_u32(),
				keyset_commitment: H256::from_slice(
					beefy_consensus_state.current_authority_set.root.as_slice(),
				),
			},
			next_authorities: BeefyNextAuthoritySet {
				id: beefy_consensus_state.next_authority_set.id.as_u64(),
				len: beefy_consensus_state.next_authority_set.len.as_u32(),
				keyset_commitment: H256::from_slice(
					beefy_consensus_state.next_authority_set.root.as_slice(),
				),
			},
		};
		Ok(consensus_state.encode())
	}

	async fn query_latest_height(&self, _id: StateMachineId) -> Result<u32, Error> {
		let contract = IIsmpHost::new(self.ismp_host, self.client.clone());
		let value = contract.latest_state_machine_height().call().await?;
		Ok(value.low_u64() as u32)
	}

	async fn query_latest_messaging_height(
		&self,
		id: StateMachineId,
	) -> Result<u64, anyhow::Error> {
		// todo: needs correct implementation
		self.query_latest_height(id).await.map(|height| height.into())
	}

	async fn query_consensus_update_time(&self, _id: ConsensusStateId) -> Result<Duration, Error> {
		let contract = IIsmpHost::new(self.ismp_host, self.client.clone());
		let value = contract.consensus_update_time().call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_challenge_period(&self, _id: ConsensusStateId) -> Result<Duration, Error> {
		let contract = IIsmpHost::new(self.ismp_host, self.client.clone());
		let value = contract.challenge_period().call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_timestamp(&self) -> Result<Duration, Error> {
		let client = Arc::new(self.client.clone());
		let contract = IIsmpHost::new(self.ismp_host, client);
		let value = contract.timestamp().call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_requests_proof(&self, at: u64, keys: Vec<Query>) -> Result<Vec<u8>, Error> {
		let keys = keys
			.into_iter()
			.map(|query| self.request_commitment_key(query.commitment))
			.collect();

		let proof = self.client.get_proof(self.ismp_host, keys, Some(at.into())).await?;
		let proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
			storage_proof: proof
				.storage_proof
				.into_iter()
				.map(|proof| {
					(
						sp_core::keccak_256(&proof.key.0).to_vec(),
						proof.proof.into_iter().map(|bytes| bytes.0.into()).collect(),
					)
				})
				.collect(),
		};
		Ok(proof.encode())
	}

	async fn query_responses_proof(&self, at: u64, keys: Vec<Query>) -> Result<Vec<u8>, Error> {
		let keys = keys
			.into_iter()
			.map(|query| self.response_commitment_key(query.commitment))
			.collect();
		let proof = self.client.get_proof(self.ismp_host, keys, Some(at.into())).await?;
		let proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
			storage_proof: proof
				.storage_proof
				.into_iter()
				.map(|proof| {
					(
						sp_core::keccak_256(&proof.key.0).to_vec(),
						proof.proof.into_iter().map(|bytes| bytes.0.into()).collect(),
					)
				})
				.collect(),
		};
		Ok(proof.encode())
	}

	async fn query_state_proof(&self, at: u64, keys: Vec<Vec<u8>>) -> Result<Vec<u8>, Error> {
		let mut contract_proofs: Vec<_> = vec![];
		let mut map: BTreeMap<Vec<u8>, Vec<Vec<u8>>> = BTreeMap::new();
		for key in keys {
			if key.len() != 52 {
				Err(anyhow!("Invalid key supplied, keys should be 52 bytes"))?
			}

			let contract_address = H160::from_slice(&key[..20]);
			let slot_hash = H256::from_slice(&key[20..]);
			let proof = self
				.client
				.get_proof(contract_address, vec![slot_hash], Some(at.into()))
				.await?;
			contract_proofs
				.push(StorageProof::new(proof.account_proof.into_iter().map(|node| node.0.into())));
			map.insert(
				key,
				proof
					.storage_proof
					.get(0)
					.cloned()
					.ok_or_else(|| {
						anyhow!("Invalid key supplied, storage proof could not be retrieved")
					})?
					.proof
					.into_iter()
					.map(|bytes| bytes.0.into())
					.collect(),
			);
		}

		let contract_proof = StorageProof::merge(contract_proofs);

		let state_proof = EvmStateProof {
			contract_proof: contract_proof.into_nodes().into_iter().collect(),
			storage_proof: map,
		};
		Ok(state_proof.encode())
	}

	async fn query_ismp_events(&self, event: StateMachineUpdated) -> Result<Vec<Event>, Error> {
		let latest_height = Arc::clone(&self.latest_height);
		let previous_height = *latest_height.lock() + 1;
		let events = self.events(previous_height, event.latest_height).await?;
		*latest_height.lock() = event.latest_height;
		Ok(events)
	}

	async fn query_pending_get_requests(&self, _height: u64) -> Result<Vec<Get>, Error> {
		Ok(Default::default())
	}

	fn name(&self) -> String {
		self.state_machine.to_string()
	}

	fn state_machine_id(&self) -> StateMachineId {
		StateMachineId { state_id: self.state_machine, consensus_state_id: self.consensus_state_id }
	}

	fn block_max_gas(&self) -> u64 {
		self.gas_limit
	}

	async fn estimate_gas(&self, _msg: Vec<Message>) -> Result<u64, Error> {
		todo!()
	}

	async fn state_machine_update_notification(
		&self,
		_counterparty_state_id: StateMachineId,
	) -> BoxStream<StateMachineUpdated> {
		use ethers::{contract::parse_log, core::types::Log};
		let mut obj = ObjectParams::new();
		let address = format!("{:?}", self.handler);
		obj.insert("address", address.as_str())
			.expect("handler address should be valid");
		let param = obj.to_rpc_params().ok().flatten().expect("Failed to serialize rpc params");
		let sub = self
			.rpc_client
			.subscribe::<Log, _>("eth_subscribe", rpc_params!("logs", param), "eth_unsubscribe")
			.await
			.expect("Failed to susbcribe");
		let stream = sub.filter_map(|log| async move {
			log.ok().and_then(|log| {
				parse_log::<StateMachineUpdatedFilter>(log).ok().map(|ev| Ok(ev.into()))
			})
		});

		Box::pin(stream)
	}

	async fn submit(&self, messages: Vec<Message>) -> Result<(), Error> {
		submit_messages(&self, messages).await?;
		Ok(())
	}
}
