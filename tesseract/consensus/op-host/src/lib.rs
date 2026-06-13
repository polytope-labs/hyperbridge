/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = "consensus-op-host";

use abi::{DisputeGameFactory, FaultDisputeGame, L2OutputOracle};
use alloy::{
	eips::BlockId,
	network::EthereumWallet,
	primitives::{Address, B256},
	providers::{Provider, ProviderBuilder},
	rpc::types::Filter,
	signers::local::PrivateKeySigner,
	sol_types::SolEvent,
};
use anyhow::anyhow;
use geth_primitives::{alloy_u256_to_primitive, Header};
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use op_verifier::{
	calculate_output_root, get_game_uuid, DisputeGameImpl, GameTypeConfig,
	OptimismDisputeGameProof, OptimismPayloadProof, AGGREGATE_VERIFIER_COUNTERED_BY_SLOT,
	DISPUTE_GAMES_SLOT, FAULT_DISPUTE_CLAIM_DATA_SLOT, GAME_IMPLS_SLOT, L2_OUTPUTS_SLOT,
};
use primitive_types::{H160, H256, U256};
use reqwest::Client;
use reqwest_chain::ChainMiddleware;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde::{Deserialize, Serialize};
use sp_core::{bytes::from_hex, keccak_256, Pair};
use std::sync::Arc;
use sync_committee_prover::middleware::SwitchProviderMiddleware;
use tesseract_evm::{derive_map_key, AlloyProvider, AlloySignerProvider, EvmClient, EvmConfig};
use tesseract_primitives::{Hasher, IsmpHost, IsmpProvider};

pub mod abi;
mod host;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpConfig {
	/// OpStack Host config
	#[serde(flatten)]
	pub host: HostConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
	/// WS url for the beacon execution client
	pub ethereum_rpc_url: Vec<String>,
	/// L2Oracle contract address on L1
	pub l2_oracle: Option<H160>,
	/// DisputeGameFactory contract address on L1
	pub dispute_game_factory: Option<H160>,
	/// Withdrawals Message Passer contract address on L2
	pub message_parser: H160,
	/// proposer config
	pub proposer_config: Option<ProposerConfig>,
	/// State machine Identifier for the L1/Beacon chain.
	#[serde(with = "serde_hex_utils::as_string")]
	pub l1_state_machine: StateMachine,
	/// L1 Consensus state Id representation.
	pub l1_consensus_state_id: String,
	/// Consensus state id used by this op-host consensus client. Always
	/// overrides the paired `EvmConfig.consensus_state_id` for both the
	/// host's own queries **and** the underlying `EvmClient` provider, so the
	/// messaging and consensus paths agree on the same id.
	pub consensus_state_id: String,
	/// consensus update frequency in seconds
	pub consensus_update_frequency: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposerConfig {
	/// Proposer account, private key
	pub proposer: String,
	/// beacon consensus client rpc
	pub beacon_consensus_rpcs: Vec<String>,
	/// Proposer interval
	/// This represents the interval which the opstack proposer uses to propose output roots in
	/// seconds
	pub proposer_interval: u64,
	/// Address of the official op-proposer
	pub op_proposer: String,
}

impl OpConfig {
	/// Convert the config into a client. Caller supplies the chain's EVM host
	/// config; we no longer bundle it into this struct.
	pub async fn into_client(self, evm_config: EvmConfig) -> anyhow::Result<Arc<dyn IsmpHost>> {
		let client = OpHost::new(&self.host, &evm_config).await?;

		Ok(Arc::new(client))
	}
}

#[derive(Clone)]
pub struct OpHost {
	/// Optimism stack execution client
	pub op_execution_client: Arc<AlloyProvider>,
	/// Beacon execution client
	pub(crate) beacon_execution_client: Arc<AlloyProvider>,
	/// L2Oracle contract address on L1
	pub(crate) l2_oracle: Option<H160>,
	/// Dispute Game factory address
	pub(crate) dispute_game_factory: Option<H160>,
	/// Withdrawals Message Passer contract address on L2
	pub(crate) message_parser: H160,
	/// Host config
	pub host: HostConfig,
	/// Evm Config
	pub evm: EvmConfig,
	/// Resolved state machine identifier for this host's chain.
	pub state_machine: StateMachine,
	/// Resolved IsmpHost contract address on this host's chain.
	pub ismp_host: H160,
	/// Consensus state id
	pub consensus_state_id: ConsensusStateId,
	/// Ismp provider
	pub provider: Arc<dyn IsmpProvider>,
	/// Transaction signer
	pub proposer: Option<Arc<AlloySignerProvider>>,
	/// L1 state machine id
	pub l1_state_machine: StateMachine,
	/// beacon consensus client
	pub beacon_consensus_client: Option<ClientWithMiddleware>,
	/// L1 Consensus state Id representation.
	pub l1_consensus_state_id: ConsensusStateId,
}

pub fn derive_array_item_key(index_in_array: u64, offset: u64) -> H256 {
	let bytes = U256::from(L2_OUTPUTS_SLOT as u64).to_big_endian();

	let hash_result = keccak_256(&bytes);

	let array_pos = U256::from_big_endian(&hash_result);
	let item_pos = array_pos + U256::from(index_in_array * 2) + U256::from(offset);

	let pos = item_pos.to_big_endian();

	pos.into()
}

/// Whether the game's "not challenged" storage slot, read as a 32-byte big-endian word,
/// indicates the game has been challenged. `OPSuccinct` games have no challenge mechanism and
/// are never challenged. For the other two kinds the check follows the verifier's on-chain
/// contract layouts exactly so filtered games are precisely those that `verify_not_challenged`
/// would reject.
pub fn game_is_challenged(kind: &DisputeGameImpl, slot_value: alloy::primitives::U256) -> bool {
	const ZERO_ADDRESS: [u8; 20] = [0u8; 20];
	match kind {
		DisputeGameImpl::OPSuccinct => false,
		DisputeGameImpl::FaultDisputeGame => {
			// claimData[0] packs (uint32 parentIndex, address counteredBy, ...) with
			// counteredBy at bytes [8..28] of the 32-byte word viewed big-endian.
			let bytes = slot_value.to_be_bytes::<32>();
			&bytes[8..28] != ZERO_ADDRESS.as_slice()
		},
		// `counteredByIntermediateRootIndexPlusOne == 0` iff unchallenged.
		DisputeGameImpl::AggregateVerifier => !slot_value.is_zero(),
	}
}

/// The storage slot(s) to prove on the game proxy to establish "not challenged". Returned as
/// `B256` keys for `eth_getProof`. Empty for `OPSuccinct` games, which have no challenge state.
pub fn challenge_slot_keys(kind: &DisputeGameImpl) -> Vec<B256> {
	match kind {
		DisputeGameImpl::OPSuccinct => Vec::new(),
		DisputeGameImpl::FaultDisputeGame => {
			// claimData[0] lives at keccak256(abi.encode(claimDataSlot)).
			let slot = U256::from(FAULT_DISPUTE_CLAIM_DATA_SLOT).to_big_endian();
			let hash = keccak_256(&slot);
			vec![B256::from_slice(&hash)]
		},
		DisputeGameImpl::AggregateVerifier => {
			let mut key = [0u8; 32];
			key[24..].copy_from_slice(&AGGREGATE_VERIFIER_COUNTERED_BY_SLOT.to_be_bytes());
			vec![B256::from_slice(&key)]
		},
	}
}

impl OpHost {
	pub async fn new(host: &HostConfig, evm: &EvmConfig) -> Result<Self, anyhow::Error> {
		// Always overwrite the EvmConfig's consensus state id with the
		// host-level value so the underlying `EvmClient` and the op-host
		// agree on the same id.
		let evm_owned = {
			let mut evm_override = evm.clone();
			evm_override.consensus_state_id = Some(host.consensus_state_id.clone());
			evm_override
		};
		let evm: &EvmConfig = &evm_owned;

		let el = tesseract_evm::create_provider(&evm.rpc_urls)?;
		let beacon_client = tesseract_evm::create_provider(&host.ethereum_rpc_url)?;

		let l1_chain_id = beacon_client.get_chain_id().await?;
		let l1_state_machine = StateMachine::Evm(l1_chain_id as u32);

		let inner = EvmClient::new(evm.clone()).await?;
		let state_machine = inner.state_machine;
		let ismp_host = inner.ismp_host;
		let consensus_state_id = inner.consensus_state_id;
		let provider = Arc::new(inner);

		let (proposer, beacon_consensus_client) =
			if let Some(proposer_config) = host.proposer_config.clone() {
				let bytes = match from_hex(proposer_config.proposer.as_str()) {
					Ok(bytes) => bytes,
					Err(_) => {
						// it's probably a file.
						let contents =
							tokio::fs::read_to_string(proposer_config.proposer.as_str()).await?;
						from_hex(contents.as_str())?
					},
				};

				let signer = sp_core::ecdsa::Pair::from_seed_slice(&bytes)?;
				let signing_key =
					alloy::signers::k256::ecdsa::SigningKey::from_slice(signer.seed().as_slice())?;
				let wallet_signer = PrivateKeySigner::from_signing_key(signing_key);
				let wallet = EthereumWallet::from(wallet_signer);

				let beacon_provider = tesseract_evm::create_provider(&host.ethereum_rpc_url)?;
				let signer_provider =
					ProviderBuilder::new().wallet(wallet).connect_provider(beacon_provider);

				let client = ClientBuilder::new(Client::new())
					.with(ChainMiddleware::new(SwitchProviderMiddleware::_new(
						proposer_config.beacon_consensus_rpcs,
					)))
					.build();

				(Some(Arc::new(signer_provider)), Some(client))
			} else {
				(None, None)
			};

		Ok(Self {
			op_execution_client: Arc::new(el),
			beacon_execution_client: Arc::new(beacon_client),
			l2_oracle: host.l2_oracle,
			dispute_game_factory: host.dispute_game_factory,
			message_parser: host.message_parser,
			evm: evm.clone(),
			host: host.clone(),
			state_machine,
			ismp_host,
			consensus_state_id,
			provider,
			proposer,
			l1_state_machine,
			beacon_consensus_client,
			l1_consensus_state_id: {
				let mut consensus_state_id: ConsensusStateId = Default::default();
				consensus_state_id.copy_from_slice(host.l1_consensus_state_id.as_bytes());
				consensus_state_id
			},
		})
	}

	pub async fn latest_event(
		&self,
		from: u64,
		to: u64,
	) -> Result<Option<L2OutputOracle::OutputProposed>, anyhow::Error> {
		if from > to {
			return Ok(None);
		}
		let l2_oracle = self
			.l2_oracle
			.ok_or_else(|| anyhow!("L2 Oracle address is missing for {}", self.state_machine))?;
		let oracle_addr = Address::from_slice(&l2_oracle.0);
		let filter = Filter::new().address(oracle_addr).from_block(from).to_block(to);

		let logs = self.beacon_execution_client.get_logs(&filter).await?;

		let mut events: Vec<L2OutputOracle::OutputProposed> = logs
			.into_iter()
			.filter_map(|log| L2OutputOracle::OutputProposed::decode_log(&log.inner).ok())
			.map(|log| log.data)
			.collect();

		events.sort_unstable_by(|a, b| a.l2OutputIndex.cmp(&b.l2OutputIndex));

		Ok(events.last().cloned())
	}

	pub async fn latest_dispute_games(
		&self,
		from: u64,
		to: u64,
		game_type_configs: Vec<GameTypeConfig>,
	) -> Result<Vec<DisputeGameFactory::DisputeGameCreated>, anyhow::Error> {
		if from > to {
			return Ok(Default::default());
		}
		let dispute_game_factory = self.dispute_game_factory.ok_or_else(|| {
			anyhow!("Dispute Factory address is missing for {}", self.state_machine)
		})?;

		let factory_addr = Address::from_slice(&dispute_game_factory.0);
		let filter = Filter::new().address(factory_addr).from_block(from).to_block(to);

		let logs = self.beacon_execution_client.get_logs(&filter).await?;

		let candidates: Vec<DisputeGameFactory::DisputeGameCreated> = logs
			.into_iter()
			.filter_map(|log| DisputeGameFactory::DisputeGameCreated::decode_log(&log.inner).ok())
			.map(|log| log.data)
			.filter(|a| game_type_configs.iter().any(|c| c.game_type == a.gameType))
			.collect();

		// Drop events whose game has already been challenged — they will always fail
		// verification downstream, so there's no point carrying them further. Reads the proxy's
		// "not challenged" storage slot directly via `eth_getStorageAt` (cheaper than
		// `eth_getProof`, which `fetch_dispute_game_payload` does anyway for payloads we keep).
		let mut events = Vec::with_capacity(candidates.len());
		for event in candidates {
			let Some(config) = game_type_configs.iter().find(|c| c.game_type == event.gameType)
			else {
				continue;
			};
			let challenged = match challenge_slot_keys(&config.kind).first() {
				None => false,
				Some(slot) => {
					let value = self
						.beacon_execution_client
						.get_storage_at(
							event.disputeProxy,
							alloy::primitives::U256::from_be_slice(slot.as_slice()),
						)
						.block_id(to.into())
						.await?;
					game_is_challenged(&config.kind, value)
				},
			};
			if challenged {
				log::trace!(target: LOG_TARGET, "Skipping challenged dispute game {:?} (game_type {})", event.disputeProxy, event.gameType);
				continue;
			}
			events.push(event);
		}

		Ok(events)
	}

	pub async fn fetch_dispute_game_payload(
		&self,
		at: u64,
		game_type_configs: Vec<GameTypeConfig>,
		events: Vec<DisputeGameFactory::DisputeGameCreated>,
	) -> Result<Option<OptimismDisputeGameProof>, anyhow::Error> {
		let mut payloads = vec![];
		// Count games skipped due to *errors* (RPC/proof failures), as opposed to legitimate
		// filtering (wrong game type, invalid output root, nonexistent block).
		let mut errored = 0usize;
		let dispute_game_factory = self.dispute_game_factory.ok_or_else(|| {
			anyhow!("Dispute Factory address is missing for {}", self.state_machine)
		})?;

		for event in events {
			let proxy_addr = event.disputeProxy;
			let contract = FaultDisputeGame::new(proxy_addr, &*self.beacon_execution_client);

			// A single un-provable game (e.g. one whose backing L2 block is older than the RPC's
			// proof window, or any transient per-game RPC error) must not abort the whole batch —
			// otherwise once the relayer falls behind, the oldest game in the L1 range poisons
			// every update and consensus can never make progress. Skip such games and let the
			// newest provable one produce the update.
			let extra_data = match contract.extraData().block(BlockId::latest()).call().await {
				Ok(v) => v,
				Err(e) => {
					log::warn!(target: LOG_TARGET, "Skipping dispute game {proxy_addr:?} (game_type {}): extraData() failed: {e:?}", event.gameType);
					errored += 1;
					continue;
				},
			};
			let timestamp = match contract.createdAt().block(BlockId::latest()).call().await {
				Ok(v) => v,
				Err(e) => {
					log::warn!(target: LOG_TARGET, "Skipping dispute game {proxy_addr:?} (game_type {}): createdAt() failed: {e:?}", event.gameType);
					errored += 1;
					continue;
				},
			};

			// All game types we support lay out their `extraData` with the L2 block number as
			// the first 32 bytes: Cannon encodes it alone, AggregateVerifier prefixes it before
			// the intermediate roots and final root claim. Decoding here avoids depending on
			// a top-level `l2SequenceNumber()` getter that not every implementation exposes.
			if extra_data.len() < 32 {
				log::trace!(target: LOG_TARGET, "Skipping dispute game with extraData shorter than 32 bytes ({} bytes)", extra_data.len());
				continue;
			}
			let l2_block_num = alloy::primitives::U256::from_be_slice(&extra_data[..32])
				.try_into()
				.unwrap_or(u64::MAX);

			// Since anyone can create dispute games including bots we need to be sure the block
			// number exists
			let current_block = match self.op_execution_client.get_block_number().await {
				Ok(v) => v,
				Err(e) => {
					log::warn!(target: LOG_TARGET, "Skipping dispute game {proxy_addr:?} (game_type {}): fetching L2 head failed: {e:?}", event.gameType);
					errored += 1;
					continue;
				},
			};
			if l2_block_num > current_block {
				log::trace!(target: LOG_TARGET, "Found a dispute game event with a block number that does not exist {l2_block_num}");
				continue;
			}

			let config = match game_type_configs.iter().find(|c| c.game_type == event.gameType) {
				Some(config) => config.clone(),
				None => {
					log::trace!(target: LOG_TARGET, "Found a dispute game event with wrong game type {}", event.gameType);
					continue;
				},
			};

			let game_uuid = get_game_uuid::<Hasher>(
				event.gameType,
				event.rootClaim.0.into(),
				extra_data.to_vec(),
			);
			let dispute_game_key = derive_map_key(game_uuid.0.to_vec(), DISPUTE_GAMES_SLOT);

			// Build the key for gameImpls[game_type]: keccak256(keccak256(padded_u32 . slot)).
			let game_impl_key = {
				let mut k = vec![0u8; 32];
				k[28..].copy_from_slice(&event.gameType.to_be_bytes());
				derive_map_key(k, GAME_IMPLS_SLOT)
			};

			let factory_addr = Address::from_slice(&dispute_game_factory.0);
			let factory_proof = match self
				.beacon_execution_client
				.get_proof(
					factory_addr,
					vec![B256::from_slice(&dispute_game_key.0), B256::from_slice(&game_impl_key.0)],
				)
				.block_id(at.into())
				.await
			{
				Ok(v) => v,
				Err(e) => {
					log::warn!(target: LOG_TARGET, "Skipping dispute game {proxy_addr:?} (game_type {}): factory get_proof failed: {e:?}", event.gameType);
					errored += 1;
					continue;
				},
			};

			let Some(dispute_game_storage) = factory_proof.storage_proof.get(0).cloned() else {
				log::warn!(target: LOG_TARGET, "Skipping dispute game {proxy_addr:?} (game_type {}): storage proof missing for dispute game slot", event.gameType);
				errored += 1;
				continue;
			};
			let dispute_game_proof =
				dispute_game_storage.proof.into_iter().map(|node| node.to_vec()).collect();

			let Some(game_impl_storage) = factory_proof.storage_proof.get(1).cloned() else {
				log::warn!(target: LOG_TARGET, "Skipping dispute game {proxy_addr:?} (game_type {}): storage proof missing for gameImpls[gameType]", event.gameType);
				errored += 1;
				continue;
			};
			let game_impl_proof =
				game_impl_storage.proof.into_iter().map(|node| node.to_vec()).collect();

			// Account + storage proof for the proxy's "not challenged" slot.
			let challenge_slots = challenge_slot_keys(&config.kind);
			let proxy_proof = match self
				.beacon_execution_client
				.get_proof(proxy_addr, challenge_slots.clone())
				.block_id(at.into())
				.await
			{
				Ok(v) => v,
				Err(e) => {
					log::warn!(target: LOG_TARGET, "Skipping dispute game {proxy_addr:?} (game_type {}): proxy get_proof failed: {e:?}", event.gameType);
					errored += 1;
					continue;
				},
			};
			let proxy_account_proof = proxy_proof
				.account_proof
				.iter()
				.cloned()
				.map(|n| n.to_vec())
				.collect::<Vec<_>>();
			let challenge_proof = if challenge_slots.is_empty() {
				Vec::new()
			} else {
				let Some(challenge_storage) = proxy_proof.storage_proof.get(0).cloned() else {
					log::warn!(target: LOG_TARGET, "Skipping dispute game {proxy_addr:?} (game_type {}): storage proof missing for challenge slot", event.gameType);
					errored += 1;
					continue;
				};
				challenge_storage.proof.into_iter().map(|node| node.to_vec()).collect()
			};

			let block = match self
				.op_execution_client
				.get_block(BlockId::number(l2_block_num))
				.await
			{
				Ok(Some(b)) => b,
				Ok(None) => {
					log::warn!(target: LOG_TARGET, "Skipping dispute game {proxy_addr:?} (game_type {}): header not found for L2 block {l2_block_num}", event.gameType);
					errored += 1;
					continue;
				},
				Err(e) => {
					log::warn!(target: LOG_TARGET, "Skipping dispute game {proxy_addr:?} (game_type {}): get_block for L2 block {l2_block_num} failed: {e:?}", event.gameType);
					errored += 1;
					continue;
				},
			};

			let header = block.into();
			let l2_block_hash = Header::from(&header).hash::<Hasher>();
			let message_parser_addr = Address::from_slice(&self.message_parser.0);
			// We only need the message-parser account's storage root, not a merkle proof, so use
			// `eth_getAccount` instead of the heavier `eth_getProof`.
			let message_parser_account = match self
				.op_execution_client
				.get_account(message_parser_addr)
				.block_id(l2_block_num.into())
				.await
			{
				Ok(v) => v,
				Err(e) => {
					log::warn!(target: LOG_TARGET, "Skipping dispute game {proxy_addr:?} (game_type {}): message-parser get_account at L2 block {l2_block_num} failed: {e:?}", event.gameType);
					errored += 1;
					continue;
				},
			};

			let payload = OptimismDisputeGameProof {
				withdrawal_storage_root: message_parser_account.storage_root.0.into(),
				// Version bytes is still the default value
				version: H256::zero(),
				dispute_factory_proof: factory_proof
					.account_proof
					.iter()
					.cloned()
					.map(|node| node.to_vec())
					.collect(),
				dispute_game_proof,
				game_impl_proof,
				proxy_account_proof,
				challenge_proof,
				timestamp,
				header,
				proxy: proxy_addr.0 .0.into(),
				extra_data: extra_data.to_vec(),
				game_type: event.gameType,
			};

			// Check if rootClaim matches derived output root.
			let output_root = calculate_output_root::<Hasher>(
				payload.version,
				payload.header.state_root,
				payload.withdrawal_storage_root,
				l2_block_hash,
			);

			if output_root.0 != event.rootClaim.0 {
				log::trace!(target: LOG_TARGET, "Found a dispute game event with an invalid output root, Expected: {output_root:?}, Found: {:?}", event.rootClaim);
				continue;
			}

			payloads.push(payload)
		}

		payloads.sort_unstable_by(|a, b| a.header.number.cmp(&b.header.number));

		// If we produced nothing but some games failed with errors (rather than being legitimately
		// filtered out), surface an error so the caller retries the range instead of silently
		// advancing past games it never managed to evaluate.
		if payloads.is_empty() && errored > 0 {
			return Err(anyhow!(
				"All {errored} dispute game(s) in batch at L1 height {at} failed to produce a payload \
				 (see warnings for per-game causes, e.g. RPC proof-window limits)"
			));
		}

		Ok(payloads.last().cloned())
	}

	pub async fn fetch_op_payload(
		&self,
		at: u64,
		event: L2OutputOracle::OutputProposed,
	) -> Result<OptimismPayloadProof, anyhow::Error> {
		let l2_output_index = alloy_u256_to_primitive(event.l2OutputIndex).as_u64();
		let l2_block_number = alloy_u256_to_primitive(event.l2BlockNumber).as_u64();

		let output_roots_key = derive_array_item_key(l2_output_index, 0);
		let timestamp_and_block_proof = derive_array_item_key(l2_output_index, 1);
		let l2_oracle = self
			.l2_oracle
			.ok_or_else(|| anyhow!("L2 Oracle address is missing for {}", self.state_machine))?;

		let oracle_addr = Address::from_slice(&l2_oracle.0);
		let proof = self
			.beacon_execution_client
			.get_proof(
				oracle_addr,
				vec![
					B256::from_slice(&output_roots_key.0),
					B256::from_slice(&timestamp_and_block_proof.0),
				],
			)
			.block_id(at.into())
			.await?;

		let output_root_proof = proof
			.storage_proof
			.first()
			.cloned()
			.ok_or_else(|| anyhow!("Storage proof not found for optimism output root"))?
			.proof
			.into_iter()
			.map(|node| node.to_vec())
			.collect();

		let multi_proof = proof
			.storage_proof
			.get(1)
			.cloned()
			.ok_or_else(|| {
				anyhow!("Storage proof not found for optimism timestamp and block number")
			})?
			.proof
			.into_iter()
			.map(|node| node.to_vec())
			.collect();

		let block = self
			.op_execution_client
			.get_block(BlockId::number(l2_block_number))
			.await?
			.ok_or_else(|| anyhow!("Header not found for {:?}", l2_block_number))?;

		let message_parser_addr = Address::from_slice(&self.message_parser.0);
		// We only need the message-parser account's storage root, not a merkle proof, so use
		// `eth_getAccount` instead of the heavier `eth_getProof`.
		let message_parser_account = self
			.op_execution_client
			.get_account(message_parser_addr)
			.block_id(l2_block_number.into())
			.await?;

		let payload = OptimismPayloadProof {
			state_root: block.header.state_root.0.into(),
			withdrawal_storage_root: message_parser_account.storage_root.0.into(),
			l2_block_hash: block.header.hash.0.into(),
			// Version bytes is still the default value
			version: H256::zero(),
			l2_oracle_proof: proof.account_proof.into_iter().map(|node| node.to_vec()).collect(),
			output_root_proof,
			multi_proof,
			output_root_index: l2_output_index,
			block_number: l2_block_number,
			timestamp: block.header.timestamp,
		};

		Ok(payload)
	}
}
