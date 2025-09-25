use abi::{
	dispute_game_factory::{DisputeGameCreatedFilter, DisputeGameFactory},
	fault_dispute_game::FaultDisputeGame,
};
use anyhow::anyhow;
use ethers::{
	core::k256::{ecdsa::SigningKey, SecretKey},
	middleware::{MiddlewareBuilder, SignerMiddleware},
	prelude::Provider,
	providers::{Http, Middleware},
	signers::{LocalWallet, Signer, Wallet},
};
use geth_primitives::Header;
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use op_verifier::{
	calculate_output_root, get_game_uuid, OptimismDisputeGameProof, OptimismPayloadProof,
	DISPUTE_GAMES_SLOT, L2_OUTPUTS_SLOT,
};
use primitive_types::{H160, H256, U256};
use reqwest::Client;
use reqwest_chain::ChainMiddleware;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde::{Deserialize, Serialize};
use sp_core::{bytes::from_hex, keccak_256, Pair};
use std::sync::Arc;
use sync_committee_prover::middleware::SwitchProviderMiddleware;
use tesseract_evm::{derive_map_key, EvmClient, EvmConfig};
use tesseract_primitives::{Hasher, IsmpHost, IsmpProvider};

use abi::l2_output_oracle::*;
mod abi;
mod host;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpConfig {
	/// OpStack Host config
	pub host: HostConfig,
	/// General Evm client config
	#[serde[flatten]]
	pub evm_config: EvmConfig,
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
	/// consensus update frequency in seconds
	pub consensus_update_frequency: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposerConfig {
	/// Proposer account, private key
	pub proposer: String,
	/// Etherscan API key
	pub l1_etherscan_api_key: String,
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
	/// Convert the config into a client.
	pub async fn into_client(self) -> anyhow::Result<Arc<dyn IsmpHost>> {
		let client = OpHost::new(&self.host, &self.evm_config).await?;

		Ok(Arc::new(client))
	}

	pub fn state_machine(&self) -> StateMachine {
		self.evm_config.state_machine
	}
}

#[derive(Clone)]
pub struct OpHost {
	/// Optimism stack execution client
	pub op_execution_client: Arc<Provider<Http>>,
	/// Beacon execution client
	pub(crate) beacon_execution_client: Arc<Provider<Http>>,
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
	/// Consensus state id
	pub consensus_state_id: ConsensusStateId,
	/// Ismp provider
	pub provider: Arc<dyn IsmpProvider>,
	/// Transaction signer
	pub proposer: Option<Arc<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>>,
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

impl OpHost {
	pub async fn new(host: &HostConfig, evm: &EvmConfig) -> Result<Self, anyhow::Error> {
		let el = Provider::new(Http::new_client_with_chain_middleware(
			evm.rpc_urls.iter().map(|url| url.parse()).collect::<Result<_, _>>()?,
			None,
		));
		let beacon_client = Provider::new(Http::new_client_with_chain_middleware(
			host.ethereum_rpc_url.iter().map(|url| url.parse()).collect::<Result<_, _>>()?,
			None,
		));
		let l1_chain_id = beacon_client.get_chainid().await?.low_u64();
		let l1_state_machine = StateMachine::Evm(l1_chain_id as u32);

		let provider = Arc::new(EvmClient::new(evm.clone()).await?);

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
				let signer = LocalWallet::from(SecretKey::from_slice(signer.seed().as_slice())?)
					.with_chain_id(l1_chain_id);

				let client = ClientBuilder::new(Client::new())
					.with(ChainMiddleware::new(SwitchProviderMiddleware::_new(
						proposer_config.beacon_consensus_rpcs,
					)))
					.build();

				(Some(Arc::new(beacon_client.clone().with_signer(signer))), Some(client))
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
			consensus_state_id: {
				let mut consensus_state_id: ConsensusStateId = Default::default();
				consensus_state_id.copy_from_slice(evm.consensus_state_id.as_bytes());
				consensus_state_id
			},
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
	) -> Result<Option<OutputProposedFilter>, anyhow::Error> {
		if from > to {
			return Ok(None);
		}
		let l2_oracle = self.l2_oracle.ok_or_else(|| {
			anyhow!("L2 Oracle address is missing for {}", self.evm.state_machine)
		})?;
		let contract = L2OutputOracle::new(l2_oracle.0, self.beacon_execution_client.clone());
		let mut events = contract
			.event::<OutputProposedFilter>()
			.address(ethers::types::H160(l2_oracle.0).into())
			.from_block(from)
			.to_block(to)
			.query()
			.await?
			.into_iter()
			.collect::<Vec<_>>();

		events.sort_unstable_by(|a, b| a.l_2_output_index.cmp(&b.l_2_output_index));

		Ok(events.last().cloned())
	}

	pub async fn latest_dispute_games(
		&self,
		from: u64,
		to: u64,
		respected_game_types: Vec<u32>,
	) -> Result<Vec<DisputeGameCreatedFilter>, anyhow::Error> {
		if from > to {
			return Ok(Default::default());
		}
		let dispute_game_factory = self.dispute_game_factory.ok_or_else(|| {
			anyhow!("Dispute Factory address is missing for {}", self.evm.state_machine)
		})?;

		let contract =
			DisputeGameFactory::new(dispute_game_factory.0, self.beacon_execution_client.clone());
		let events = contract
			.event::<DisputeGameCreatedFilter>()
			.address(ethers::types::H160(dispute_game_factory.0).into())
			.from_block(from)
			.to_block(to)
			.query()
			.await?
			.into_iter()
			.collect::<Vec<_>>();

		let events = events
			.into_iter()
			.filter(|a| respected_game_types.contains(&a.game_type))
			.collect::<Vec<_>>();

		Ok(events)
	}
	pub async fn fetch_dispute_game_payload(
		&self,
		at: u64,
		respected_game_types: Vec<u32>,
		events: Vec<DisputeGameCreatedFilter>,
	) -> Result<Option<OptimismDisputeGameProof>, anyhow::Error> {
		let mut payloads = vec![];
		let dispute_game_factory = self.dispute_game_factory.ok_or_else(|| {
			anyhow!("Dispute Factory address is missing for {}", self.evm.state_machine)
		})?;

		for event in events {
			let contract =
				FaultDisputeGame::new(event.dispute_proxy.0, self.beacon_execution_client.clone());
			let extra_data = contract.extra_data().call().await?;
			let timestamp = contract.created_at().call().await?;
			let l2_block_number = contract.l_2_block_number().call().await?;
			// Since anyone can create dispute games including bots we need to be sure the block
			// number exists
			if l2_block_number.low_u64() >
				self.op_execution_client.get_block_number().await?.low_u64()
			{
				log::trace!(target: "tesseract", "Found a dispute game event with a block number that does not exist {l2_block_number:?}");
				continue;
			}

			if !respected_game_types.contains(&event.game_type) {
				log::trace!(target: "tesseract", "Found a dispute game event with wrong game type {event:?}");
				continue;
			}

			let game_uuid = get_game_uuid::<Hasher>(
				event.game_type,
				event.root_claim.into(),
				extra_data.0.clone().into(),
			);
			let dispute_game_key = derive_map_key(game_uuid.0.to_vec(), DISPUTE_GAMES_SLOT);

			let proof = self
				.beacon_execution_client
				.get_proof(
					ethers::types::H160(dispute_game_factory.0),
					vec![dispute_game_key.0.into()],
					Some(at.into()),
				)
				.await?;
			let dispute_game_proof = proof
				.storage_proof
				.get(0)
				.cloned()
				.ok_or_else(|| anyhow!("Storage proof not found for dispute game"))?
				.proof
				.into_iter()
				.map(|node| node.0.into())
				.collect();
			let block =
				self.op_execution_client.get_block(l2_block_number.as_u64()).await?.ok_or_else(
					|| {
						anyhow!(
							"{:?} Header not found for {:?}",
							self.evm.state_machine,
							l2_block_number
						)
					},
				)?;
			let header = block.into();
			let l2_block_hash = Header::from(&header).hash::<Hasher>();
			let message_parser_proof = self
				.op_execution_client
				.get_proof(
					ethers::types::H160(self.message_parser.0),
					vec![],
					Some(l2_block_number.low_u64().into()),
				)
				.await?;

			let payload = OptimismDisputeGameProof {
				withdrawal_storage_root: message_parser_proof.storage_hash.0.into(),
				// Version bytes is still the default value
				version: H256::zero(),
				dispute_factory_proof: proof
					.account_proof
					.into_iter()
					.map(|node| node.0.into())
					.collect(),
				dispute_game_proof,
				timestamp,
				header,
				proxy: event.dispute_proxy.0.into(),
				extra_data: extra_data.0.into(),
				game_type: event.game_type,
			};

			// Check if rootClaim matches derived output root.
			let output_root = calculate_output_root::<Hasher>(
				payload.version,
				payload.header.state_root,
				payload.withdrawal_storage_root,
				l2_block_hash,
			);

			if output_root.0 != event.root_claim {
				log::trace!(target: "tesseract", "Found a dispute game event with an invalid output root, Expected: {output_root:?}, Found: {:?}", event.root_claim);
				continue;
			}

			payloads.push(payload)
		}

		payloads.sort_unstable_by(|a, b| a.header.number.cmp(&b.header.number));

		Ok(payloads.last().cloned())
	}

	pub async fn fetch_op_payload(
		&self,
		at: u64,
		event: OutputProposedFilter,
	) -> Result<OptimismPayloadProof, anyhow::Error> {
		let output_roots_key = derive_array_item_key(event.l_2_output_index.low_u64(), 0);
		let timestamp_and_block_proof = derive_array_item_key(event.l_2_output_index.low_u64(), 1);
		let l2_oracle = self.l2_oracle.ok_or_else(|| {
			anyhow!("L2 Oracle address is missing for {}", self.evm.state_machine)
		})?;

		let proof = self
			.beacon_execution_client
			.get_proof(
				ethers::types::H160(l2_oracle.0),
				vec![output_roots_key.0.into(), timestamp_and_block_proof.0.into()],
				Some(at.into()),
			)
			.await?;
		let output_root_proof = proof
			.storage_proof
			.get(0)
			.cloned()
			.ok_or_else(|| anyhow!("Storage proof not found for optimism output root"))?
			.proof
			.into_iter()
			.map(|node| node.0.into())
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
			.map(|node| node.0.into())
			.collect();
		let block = self
			.op_execution_client
			.get_block(event.l_2_block_number.as_u64())
			.await?
			.ok_or_else(|| anyhow!("Header not found for {:?}", event.l_2_block_number))?;
		let message_parser_proof = self
			.op_execution_client
			.get_proof(
				ethers::types::H160(self.message_parser.0),
				vec![],
				Some(event.l_2_block_number.low_u64().into()),
			)
			.await?;

		let payload = OptimismPayloadProof {
			state_root: block.state_root.0.into(),
			withdrawal_storage_root: message_parser_proof.storage_hash.0.into(),
			l2_block_hash: block
				.hash
				.ok_or_else(|| anyhow!("Missing optimism block hash"))?
				.0
				.into(),
			// Version bytes is still the default value
			version: H256::zero(),
			l2_oracle_proof: proof.account_proof.into_iter().map(|node| node.0.into()).collect(),
			output_root_proof,
			multi_proof,
			output_root_index: event.l_2_output_index.low_u64(),
			block_number: event.l_2_block_number.low_u64(),
			timestamp: block.timestamp.low_u64(),
		};

		Ok(payload)
	}
}
