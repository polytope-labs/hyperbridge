use anyhow::{Error, anyhow};
use codec::{Decode, Encode};
use evm_state_machine::{
	presets::{REQUEST_COMMITMENTS_SLOT, RESPONSE_COMMITMENTS_SLOT},
	substrate_evm::{AccountInfo, AccountType, SubstrateEvmError},
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
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use subxt::{
	OnlineClient,
	backend::rpc::RpcClient,
	config::{ExtrinsicParams, HashFor, substrate::SubstrateHeader},
	ext::subxt_rpcs::{LegacyRpcMethods, rpc_params},
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature},
};
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{
	BoxStream, ByzantineHandler, EstimateGasReturnParams, IsmpProvider, Query, Signature,
	StateMachineUpdated, StateProofQueryType, TxResult,
};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct SubstrateEvmClientConfig {
	#[serde(flatten)]
	pub evm: EvmConfig,
	// Substrate websocket url
	pub ws_url: String,
}

#[derive(Clone)]
pub struct SubstrateEvmClient<C: subxt::Config> {
	pub evm: EvmClient,
	pub online_client: OnlineClient<C>,
	pub legacy_rpc: LegacyRpcMethods<C>,
	pub subxt_rpc_client: RpcClient,
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
		let (online_client, rpc_client) =
			subxt_utils::client::ws_client(&config.ws_url, 300u32 * 1024 * 1024).await?;
		let legacy_rpc = LegacyRpcMethods::<C>::new(rpc_client.clone());
		Ok(Self { evm, online_client, legacy_rpc, subxt_rpc_client: rpc_client })
	}

	pub fn storage_key(&self, slot: H256) -> Vec<u8> {
		hashing::blake2_256(slot.as_bytes()).to_vec()
	}

	pub fn contract_info_key(&self, address: H160) -> Vec<u8> {
		let mut key = Vec::new();
		key.extend_from_slice(&hashing::twox_128(b"Revive"));
		key.extend_from_slice(&hashing::twox_128(b"AccountInfoOf"));
		key.extend_from_slice(address.as_bytes());
		key
	}

	pub async fn get_contract_trie_id(
		&self,
		address: H160,
		at: HashFor<C>,
	) -> Result<Vec<u8>, Error> {
		let key = self.contract_info_key(address);

		let data = self
			.online_client
			.storage()
			.at(at)
			.fetch_raw(key)
			.await?
			.ok_or_else(|| anyhow::anyhow!("Contract info not found"))?;

		let input = &data[..];

		let account_info = AccountInfo::decode(&mut &input[..])
			.map_err(|_| SubstrateEvmError::AccountInfoDecodeError)?;

		let AccountType::Contract(contract_info) = account_info.account_type;
		let trie_id: Vec<u8> = contract_info.trie_id;

		Ok(trie_id)
	}

	/// Fetches a combined prrof: Main Trie (ContractInfo + ChildRoot) amd Child Trie (Slots)
	async fn fetch_combined_proof(
		&self,
		at: u64,
		queries: Vec<(H160, Vec<Vec<u8>>)>,
	) -> Result<Vec<u8>, Error> {
		let block_hash = self
			.legacy_rpc
			.chain_get_block_hash(Some(at.into()))
			.await?
			.ok_or_else(|| anyhow::anyhow!("Block hash not found for height {at}"))?;

		let mut main_keys = Vec::new();
		let mut contract_info = BTreeMap::new();

		for (contract_address, _) in &queries {
			let trie_id = self.get_contract_trie_id(*contract_address, block_hash).await?;
			let account_info_key = self.contract_info_key(*contract_address);

			let child_info = ChildInfo::new_default(&trie_id);
			let child_root_key = child_info.prefixed_storage_key().into_inner();

			main_keys.push(sp_storage::StorageKey(account_info_key));
			main_keys.push(sp_storage::StorageKey(child_root_key));

			contract_info.insert(contract_address.as_bytes().to_vec(), child_info);
		}

		let main_proof: ReadProof<H256> = self
			.subxt_rpc_client
			.request("state_getReadProof", rpc_params![main_keys, Some(block_hash)])
			.await?;

		let mut storage_proofs = BTreeMap::new();

		for (contract_address, keys) in queries {
			let child_info = contract_info
				.get(contract_address.as_bytes())
				.expect("Contract Info should exist");

			let keys =  keys.into_iter().map(|key| sp_storage::StorageKey(key)).collect::<Vec<_>>();
			let child_proof: ReadProof<H256> = self
				.subxt_rpc_client
				.request(
					"state_getChildReadProof",
					rpc_params![child_info.prefixed_storage_key(), keys, Some(block_hash)],
				)
				.await?;

			storage_proofs.insert(
				contract_address.as_bytes().to_vec(),
				child_proof.proof.into_iter().map(|b| b.0).collect(),
			);
		}
		let substrate_evm_proof = SubstrateEvmProof {
			main_proof: main_proof.proof.into_iter().map(|b| b.0).collect(),
			storage_proof: storage_proofs,
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

		self.fetch_combined_proof(at, vec![(self.evm.config.ismp_host, storage_keys)])
			.await
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

		self.fetch_combined_proof(at, vec![(self.evm.config.ismp_host, storage_keys)])
			.await
	}

	async fn query_state_proof(
		&self,
		at: u64,
		keys: StateProofQueryType,
	) -> Result<Vec<u8>, Error> {
		match keys {
			StateProofQueryType::Ismp(keys) => {
				if keys.iter().any(|key| key.len() != 32) {
					return Err(anyhow::anyhow!("All ISMP keys must have a length of 32 bytes",));
				}
				let storage_keys: Vec<Vec<u8>> = keys
					.into_iter()
					.map(|key| {
						let slot = H256::from_slice(&key);
						self.storage_key(slot)
					})
					.collect();

				self.fetch_combined_proof(at, vec![(self.evm.config.ismp_host, storage_keys)])
					.await
			},
			StateProofQueryType::Arbitrary(keys) => {
				let mut groups: BTreeMap<H160, Vec<Vec<u8>>> = BTreeMap::new();
				for key in keys.into_iter() {
					if key.len() != 52 {
						anyhow::bail!(
							"All arbitrary keys must have a length of 53 bytes, found {}",
							key.len()
						);
					}
					let address = H160::from_slice(&key[..20]);
					let slot = H256::from_slice(&key[20..]);
					let storage_key = self.storage_key(slot);

					groups.entry(address).or_default().push(storage_key);
				}
				self.fetch_combined_proof(at, groups.into_iter().collect()).await
			},
		}
	}

	async fn query_ismp_events(
		&self,
		previous_height: u64,
		event: StateMachineUpdated,
	) -> Result<Vec<Event>, Error> {
		self.evm.query_ismp_events(previous_height, event).await
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
		_coprocessor: StateMachine,
		counterparty: Arc<dyn IsmpProvider>,
		event: StateMachineUpdated,
	) -> Result<(), Error> {
		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: self.state_machine_id().state_id,
				consensus_state_id: self.state_machine_id().consensus_state_id,
			},
			height: event.latest_height,
		};

		let Some(block_hash) =
			self.legacy_rpc.chain_get_block_hash(Some(event.latest_height.into())).await?
		else {
			// If block header is not found veto the state commitment

			log::info!(
				"Vetoing state commitment for {} on {}: block header not found for {}",
				self.state_machine_id().state_id,
				counterparty.state_machine_id().state_id,
				event.latest_height
			);
			counterparty.veto_state_commitment(height).await?;

			return Ok(());
		};
		let header = self
			.legacy_rpc
			.chain_get_header(Some(block_hash))
			.await?
			.ok_or_else(|| anyhow!("Failed to get block header in byzantine handler"))?;

		let header = SubstrateHeader::<u32, C::Hasher>::decode(&mut &*header.encode())?;

		let state_root: H256 = header.state_root.into();
		let finalized_state_commitment =
			counterparty.query_state_machine_commitment(height).await?;

		if finalized_state_commitment.state_root != state_root.into() {
			log::info!(
				"Vetoing state commitment for {} on {}, state commitment mismatch",
				self.state_machine_id().state_id,
				counterparty.state_machine_id().state_id
			);
			counterparty.veto_state_commitment(height).await?;
		}

		Ok(())
	}

	async fn state_machine_updates(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<Vec<StateMachineUpdated>>, Error> {
		self.evm.state_machine_updates(counterparty_state_id).await
	}
}
