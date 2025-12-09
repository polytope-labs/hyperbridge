use anyhow::Error;
use codec::{Decode, Encode};
use evm_state_machine::{
	presets::{REQUEST_COMMITMENTS_SLOT, RESPONSE_COMMITMENTS_SLOT},
	types::SubstrateEvmProof,
};
use ismp::{
	consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
	events::{Event, StateCommitmentVetoed},
	host::StateMachine,
	messaging::{CreateConsensusState, Message},
};
use pallet_ismp_host_executive::HostParam;
use polkadot_sdk::*;
use primitive_types::U256;
use sp_core::{Bytes, H160, H256, hashing, storage::ChildInfo};
use std::{sync::Arc, time::Duration};
use subxt::{
	config::{ExtrinsicParams, HashFor},
	ext::subxt_rpcs::rpc_params,
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature},
};
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{
	BoxStream, ByzantineHandler, EstimateGasReturnParams, IsmpProvider, Query, Signature,
	StateMachineUpdated, StateProofQueryType, TxResult,
};
use tesseract_substrate::{SubstrateClient, SubstrateConfig};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct SubstrateEvmClientConfig {
	#[serde(flatten)]
	pub evm: EvmConfig,
	#[serde(flatten)]
	pub substrate: SubstrateConfig,
}

#[derive(Clone)]
pub struct SubstrateEvmClient<C: subxt::Config> {
	pub evm: EvmClient,
	pub substrate: SubstrateClient<C>,
}

#[derive(serde::Deserialize)]
pub struct ReadProof<H> {
	pub at: H,
	pub proof: Vec<Bytes>,
}

impl<C: subxt::Config> SubstrateEvmClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	C::AccountId: From<AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync + Encode,
	C::Signature: From<MultiSignature> + Send + Sync,
	H256: From<HashFor<C>>,
	<C::ExtrinsicParams as ExtrinsicParams<C>>::Params: Send + Sync + DefaultParams,
{
	pub async fn new(config: SubstrateEvmClientConfig) -> Result<Self, anyhow::Error> {
		let evm = EvmClient::new(config.evm).await?;
		let substrate = SubstrateClient::new(config.substrate).await?;
		Ok(Self { evm, substrate })
	}

	pub fn storage_key(&self, slot: H256) -> Vec<u8> {
		hashing::blake2_256(slot.as_bytes()).to_vec()
	}

	pub async fn get_contract_trie_id(
		&self,
		address: H160,
		at: HashFor<C>,
	) -> Result<Vec<u8>, Error> {
		let mut key = Vec::new();
		key.extend_from_slice(&hashing::twox_128(b"Revive"));
		key.extend_from_slice(&hashing::twox_128(b"AccountInfoOf"));
		key.extend_from_slice(address.as_bytes());

		let data = self
			.substrate
			.client
			.storage()
			.at(at)
			.fetch_raw(key)
			.await?
			.ok_or_else(|| anyhow::anyhow!("Contract info not found"))?;

		let mut input = &data[..];
		let variant_index = u8::decode(&mut input)?;
		if variant_index != 0 {
			return Err(anyhow::anyhow!("Account is not a contract"));
		}
		let trie_id: Vec<u8> = Vec::<u8>::decode(&mut input)?;
		Ok(trie_id)
	}

	/// Fetches a combined prrof: Main Trie (ContractInfo + ChildRoot) amd Child Trie (Slots)
	async fn fetch_combined_proof(&self, at: u64, keys: Vec<Vec<u8>>) -> Result<Vec<u8>, Error> {
		let contract_address: H160 = self.evm.config.ismp_host;
		let block_hash = self
			.substrate
			.rpc
			.chain_get_block_hash(Some(at.into()))
			.await?
			.ok_or_else(|| anyhow::anyhow!("Block hash not found for height {at}"))?;

		let trie_id = self.get_contract_trie_id(contract_address, block_hash).await?;
		let mut account_info_key = Vec::new();
		account_info_key.extend_from_slice(&hashing::twox_128(b"Revive"));
		account_info_key.extend_from_slice(&hashing::twox_128(b"AccountInfoOf"));
		account_info_key.extend_from_slice(contract_address.as_bytes());

		let child_info = ChildInfo::new_default(&trie_id);
		let child_root_key = child_info.prefixed_storage_key().into_inner();

		let main_keys = vec![account_info_key, child_root_key];

		let main_proof: ReadProof<H256> = self
			.substrate
			.rpc_client
			.request("state_getReadProof", rpc_params![main_keys, Some(block_hash)])
			.await?;
		let child_proof: ReadProof<H256> = self
			.substrate
			.rpc_client
			.request(
				"state_getChildReadProof",
				rpc_params![child_info.prefixed_storage_key(), keys, Some(block_hash)],
			)
			.await?;

		let substrate_evm_proof = SubstrateEvmProof {
			main_proof: main_proof.proof.into_iter().map(|b| b.0).collect(),
			child_proof: child_proof.proof.into_iter().map(|b| b.0).collect(),
		};

		Ok(substrate_evm_proof.encode())
	}
}

#[async_trait::async_trait]
impl<C> IsmpProvider for SubstrateEvmClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	C::AccountId: From<AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync + Encode,
	C::Signature: From<MultiSignature> + Send + Sync,
	H256: From<HashFor<C>>,
	<C::ExtrinsicParams as ExtrinsicParams<C>>::Params: Send + Sync + DefaultParams,
{
	async fn query_consensus_state(
		&self,
		at: Option<u64>,
		id: ConsensusStateId,
	) -> Result<Vec<u8>, Error> {
		self.evm.query_consensus_state(at, id).await
	}

	async fn query_latest_height(&self, id: StateMachineId) -> Result<u32, Error> {
		self.evm.query_latest_height(id).await
	}

	async fn query_finalized_height(&self) -> Result<u64, Error> {
		self.evm.query_finalized_height().await
	}

	async fn query_state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, Error> {
		self.evm.query_state_machine_commitment(height).await
	}

	async fn query_state_machine_update_time(
		&self,
		height: StateMachineHeight,
	) -> Result<Duration, Error> {
		self.evm.query_state_machine_update_time(height).await
	}

	async fn query_challenge_period(&self, id: StateMachineId) -> Result<Duration, Error> {
		self.evm.query_challenge_period(id).await
	}

	async fn query_timestamp(&self) -> Result<Duration, Error> {
		self.evm.query_timestamp().await
	}

	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let storage_keys: Vec<Vec<u8>> = keys
			.into_iter()
			.map(|q| {
				let slot_hash = tesseract_evm::derive_map_key(
					q.commitment.0.to_vec(),
					REQUEST_COMMITMENTS_SLOT,
				);
				self.storage_key(slot_hash)
			})
			.collect();

		self.fetch_combined_proof(at, storage_keys).await
	}

	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let storage_keys: Vec<Vec<u8>> = keys
			.into_iter()
			.map(|q| {
				let slot_hash = tesseract_evm::derive_map_key(
					q.commitment.0.to_vec(),
					RESPONSE_COMMITMENTS_SLOT,
				);
				self.storage_key(slot_hash)
			})
			.collect();

		self.fetch_combined_proof(at, storage_keys).await
	}

	async fn query_state_proof(
		&self,
		at: u64,
		keys: StateProofQueryType,
	) -> Result<Vec<u8>, Error> {
		let contract_addr: H160 = self.evm.config.ismp_host;

		let storage_keys: Vec<Vec<u8>> = match keys {
			StateProofQueryType::Ismp(keys) => {
				if keys.iter().any(|key| key.len() != 32) {
					return Err(anyhow::anyhow!("All ISMP keys must have a length of 32 bytes",));
				}
				keys.into_iter()
					.map(|key| {
						let slot = H256::from_slice(&key);
						self.storage_key(slot)
					})
					.collect()
			},
			StateProofQueryType::Arbitrary(keys) => {
				let mut storage_keys = Vec::new();
				for key in keys.into_iter() {
					if key.len() != 52 {
						anyhow::bail!(
							"All arbitrary keys must have a length of 53 bytes, found {}",
							key.len()
						);
					}
					let contract_address = H160::from_slice(&key[..20]);
					if contract_address != contract_addr {
						anyhow::bail!(
							"Arbitrary keys must belong to the configured ISMP host contract for Revive proof queries"
						);
					}
					let slot = H256::from_slice(&key[20..]);
					storage_keys.push(self.storage_key(slot));
				}
				storage_keys
			},
		};

		self.fetch_combined_proof(at, storage_keys).await
	}

	async fn query_ismp_events(
		&self,
		previous_height: u64,
		event: StateMachineUpdated,
	) -> Result<Vec<Event>, Error> {
		let adjusted_event = StateMachineUpdated {
			state_machine_id: event.state_machine_id,
			latest_height: event.latest_height,
		};
		self.evm.query_ismp_events(previous_height, adjusted_event).await
	}

	fn name(&self) -> String {
		self.evm.name()
	}

	fn state_machine_id(&self) -> StateMachineId {
		self.evm.state_machine_id()
	}

	fn block_max_gas(&self) -> u64 {
		self.evm.block_max_gas()
	}

	fn initial_height(&self) -> u64 {
		self.evm.initial_height()
	}

	async fn estimate_gas(&self, msg: Vec<Message>) -> Result<Vec<EstimateGasReturnParams>, Error> {
		self.evm.estimate_gas(msg).await
	}

	async fn query_request_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		self.evm.query_request_fee_metadata(hash).await
	}

	async fn query_request_receipt(&self, hash: H256) -> Result<Vec<u8>, Error> {
		self.evm.query_request_receipt(hash).await
	}

	async fn query_response_receipt(&self, hash: H256) -> Result<Vec<u8>, Error> {
		self.evm.query_response_receipt(hash).await
	}

	async fn query_response_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		self.evm.query_response_fee_metadata(hash).await
	}

	async fn state_machine_update_notification(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, Error> {
		self.evm.state_machine_update_notification(counterparty_state_id).await
	}

	async fn state_commitment_vetoed_notification(
		&self,
		from: u64,
		height: StateMachineHeight,
	) -> BoxStream<StateCommitmentVetoed> {
		self.evm.state_commitment_vetoed_notification(from, height).await
	}

	async fn submit(
		&self,
		messages: Vec<Message>,
		coprocessor: StateMachine,
	) -> Result<TxResult, Error> {
		self.evm.submit(messages, coprocessor).await
	}

	fn request_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.evm.request_commitment_full_key(commitment)
	}

	fn request_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.evm.request_receipt_full_key(commitment)
	}

	fn response_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.evm.request_commitment_full_key(commitment)
	}

	fn response_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.evm.request_receipt_full_key(commitment)
	}

	fn address(&self) -> Vec<u8> {
		self.evm.address()
	}

	fn sign(&self, msg: &[u8]) -> Signature {
		self.evm.sign(msg)
	}

	async fn set_latest_finalized_height(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), Error> {
		self.evm.set_latest_finalized_height(counterparty).await
	}

	async fn set_initial_consensus_state(
		&self,
		message: CreateConsensusState,
	) -> Result<(), Error> {
		self.evm.set_initial_consensus_state(message).await
	}

	async fn veto_state_commitment(&self, height: StateMachineHeight) -> Result<(), Error> {
		self.evm.veto_state_commitment(height).await
	}

	async fn query_host_params(
		&self,
		state_machine: StateMachine,
	) -> Result<HostParam<u128>, Error> {
		self.evm.query_host_params(state_machine).await
	}

	fn max_concurrent_queries(&self) -> usize {
		self.evm.max_concurrent_queries()
	}

	async fn fee_token_decimals(&self) -> Result<u8, Error> {
		self.evm.fee_token_decimals().await
	}
}

#[async_trait::async_trait]
impl<C> ByzantineHandler for SubstrateEvmClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	C::AccountId: From<AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync + Encode,
	C::Signature: From<MultiSignature> + Send + Sync,
	H256: From<HashFor<C>>,
	<C::ExtrinsicParams as ExtrinsicParams<C>>::Params: Send + Sync + DefaultParams,
{
	async fn check_for_byzantine_attack(
		&self,
		coprocessor: StateMachine,
		counterparty: Arc<dyn IsmpProvider>,
		challenge_event: StateMachineUpdated,
	) -> Result<(), Error> {
		self.evm
			.check_for_byzantine_attack(coprocessor, counterparty, challenge_event)
			.await
	}

	async fn state_machine_updates(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<Vec<StateMachineUpdated>>, Error> {
		self.evm.state_machine_updates(counterparty_state_id).await
	}
}
