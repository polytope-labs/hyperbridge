use abi::{
	i_rollup::*,
	i_rollup_bold::{AssertionCreatedFilter, IRollupBold},
};
use anyhow::anyhow;
use arbitrum_verifier::{
	ArbitrumBoldProof, ArbitrumPayloadProof, AssertionState, GlobalState as RustGlobalState,
	ASSERTIONS_SLOT, NODES_SLOT,
};
use ethers::{
	prelude::Provider,
	providers::{Http, Middleware},
};
use geth_primitives::{new_u256, CodecHeader};
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use primitive_types::{H160, H256, U256};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tesseract_evm::{derive_map_key, EvmClient, EvmConfig};
use tesseract_primitives::{IsmpHost, IsmpProvider};
mod abi;
mod host;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbConfig {
	/// Arbitrum Orbit Chain Host config
	pub host: HostConfig,

	/// General evm config
	#[serde[flatten]]
	pub evm_config: EvmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
	/// RPC url for beacon execution client
	pub ethereum_rpc_url: Vec<String>,
	/// RollupCore contract address on L1
	pub rollup_core: H160,
	/// State machine Identifier for the L1/Beacon chain.
	#[serde(with = "serde_hex_utils::as_string")]
	pub l1_state_machine: StateMachine,
	/// L1 Consensus state Id representation.
	pub l1_consensus_state_id: String,
	/// consensus update frequency in seconds
	pub consensus_update_frequency: Option<u64>,
}

impl ArbConfig {
	/// Convert the config into a client.
	pub async fn into_client(self) -> anyhow::Result<Arc<dyn IsmpHost>> {
		let client = ArbHost::new(&self.host, &self.evm_config).await?;

		Ok(Arc::new(client))
	}

	pub fn state_machine(&self) -> StateMachine {
		self.evm_config.state_machine
	}
}

#[derive(Clone)]
pub struct ArbHost {
	/// Arbitrum execution client
	pub arb_execution_client: Arc<Provider<Http>>,
	/// Beacon execution client
	pub(crate) beacon_execution_client: Arc<Provider<Http>>,
	/// Rollup core contract address
	pub(crate) rollup_core: H160,
	/// Host config
	pub host: HostConfig,
	/// Evm Config
	pub evm: EvmConfig,
	/// Consensus State Id
	pub consensus_state_id: ConsensusStateId,
	/// Ismp provider
	pub provider: Arc<dyn IsmpProvider>,
	/// State machine Identifier for the L1/Beacon chain.
	pub l1_state_machine: StateMachine,
	/// L1 Consensus state Id representation.
	pub l1_consensus_state_id: ConsensusStateId,
}

impl ArbHost {
	pub async fn new(host: &HostConfig, evm: &EvmConfig) -> Result<Self, anyhow::Error> {
		let el = Provider::new(Http::new_client_with_chain_middleware(
			evm.rpc_urls.iter().map(|url| url.parse()).collect::<Result<_, _>>()?,
			None,
		));
		let beacon_client = Provider::new(Http::new_client_with_chain_middleware(
			host.ethereum_rpc_url.iter().map(|url| url.parse()).collect::<Result<_, _>>()?,
			None,
		));

		let provider = Arc::new(EvmClient::new(evm.clone()).await?);

		Ok(Self {
			arb_execution_client: Arc::new(el),
			beacon_execution_client: Arc::new(beacon_client),
			rollup_core: host.rollup_core,
			host: host.clone(),
			evm: evm.clone(),
			consensus_state_id: {
				let mut consensus_state_id: ConsensusStateId = Default::default();
				consensus_state_id.copy_from_slice(evm.consensus_state_id.as_bytes());
				consensus_state_id
			},
			provider,
			l1_state_machine: host.l1_state_machine,
			l1_consensus_state_id: {
				{
					let mut consensus_state_id: ConsensusStateId = Default::default();
					consensus_state_id.copy_from_slice(host.l1_consensus_state_id.as_bytes());
					consensus_state_id
				}
			},
		})
	}

	async fn fetch_header(&self, block: H256) -> Result<CodecHeader, anyhow::Error> {
		let block = self
			.arb_execution_client
			.get_block(ethers::types::H256(block.0))
			.await?
			.ok_or_else(|| {
				anyhow!("{} Header not found for {:?}", self.evm.state_machine, block)
			})?;
		let arb_header = block.into();

		Ok(arb_header)
	}

	pub async fn latest_event(
		&self,
		from: u64,
		to: u64,
	) -> Result<Option<NodeCreatedFilter>, anyhow::Error> {
		if from > to {
			return Ok(None);
		}
		let client = Arc::new(self.beacon_execution_client.clone());
		let contract = IRollup::new(ethers::types::H160(self.rollup_core.0), client);
		let mut events = contract
			.event::<NodeCreatedFilter>()
			.address(ethers::types::H160(self.rollup_core.0).into())
			.from_block(from)
			.to_block(to)
			.query()
			.await?
			.into_iter()
			.collect::<Vec<_>>();

		events.sort_unstable_by(|a, b| a.node_num.cmp(&b.node_num));

		Ok(events.last().cloned())
	}

	pub async fn latest_assertion_event(
		&self,
		from: u64,
		to: u64,
	) -> Result<Option<AssertionCreatedFilter>, anyhow::Error> {
		if from > to {
			return Ok(None);
		}
		let client = Arc::new(self.beacon_execution_client.clone());
		let contract = IRollupBold::new(ethers::types::H160(self.rollup_core.0), client);
		let events = contract
			.event::<AssertionCreatedFilter>()
			.address(ethers::types::H160(self.rollup_core.0).into())
			.from_block(from)
			.to_block(to)
			.query()
			.await?
			.into_iter()
			.collect::<Vec<_>>();

		Ok(events.last().cloned())
	}

	pub async fn fetch_arbitrum_payload(
		&self,
		at: u64,
		event: NodeCreatedFilter,
	) -> Result<ArbitrumPayloadProof, anyhow::Error> {
		let node_num = U256::from(event.node_num).to_big_endian();
		let state_hash_key = derive_map_key(node_num.to_vec(), NODES_SLOT as u64);
		let proof = self
			.beacon_execution_client
			.get_proof(
				ethers::types::H160(self.rollup_core.0),
				vec![state_hash_key.0.into()],
				Some(at.into()),
			)
			.await?;
		let arb_block_hash = event.assertion.after_state.global_state.bytes_32_vals[0].into();
		let arbitrum_header = self.fetch_header(arb_block_hash).await?;
		let payload = ArbitrumPayloadProof {
			arbitrum_header,
			global_state: RustGlobalState {
				block_hash: arb_block_hash.0.into(),
				send_root: event.assertion.after_state.global_state.bytes_32_vals[1].into(),
				inbox_position: event.assertion.after_state.global_state.u_64_vals[0],
				position_in_message: event.assertion.after_state.global_state.u_64_vals[1],
			},
			machine_status: {
				let status = event.assertion.after_state.machine_status;
				status.try_into().map_err(|e| anyhow!("{:?}", e))?
			},
			inbox_max_count: new_u256(event.inbox_max_count),
			node_number: event.node_num,
			storage_proof: proof
				.storage_proof
				.get(0)
				.cloned()
				.ok_or_else(|| anyhow!("Storage proof not found for arbitrum state_hash"))?
				.proof
				.into_iter()
				.map(|node| node.0.into())
				.collect(),
			contract_proof: proof.account_proof.into_iter().map(|node| node.0.into()).collect(),
		};

		Ok(payload)
	}

	pub async fn fetch_arbitrum_bold_payload(
		&self,
		at: u64,
		event: AssertionCreatedFilter,
	) -> Result<ArbitrumBoldProof, anyhow::Error> {
		let assertion_hash_key =
			derive_map_key(event.assertion_hash.into(), ASSERTIONS_SLOT as u64);
		let proof = self
			.beacon_execution_client
			.get_proof(
				ethers::types::H160(self.rollup_core.0),
				vec![assertion_hash_key.0.into()],
				Some(at.into()),
			)
			.await?;
		let arb_block_hash = event.assertion.after_state.global_state.bytes_32_vals[0].into();
		let arbitrum_header = self.fetch_header(arb_block_hash).await?;
		let global_state = RustGlobalState {
			block_hash: arb_block_hash,
			send_root: event.assertion.after_state.global_state.bytes_32_vals[1].into(),
			inbox_position: event.assertion.after_state.global_state.u_64_vals[0],
			position_in_message: event.assertion.after_state.global_state.u_64_vals[1],
		};

		let machine_status = event
			.assertion
			.after_state
			.machine_status
			.try_into()
			.map_err(|_| anyhow!("Failed conversion"))?;

		let after_state = AssertionState {
			global_state,
			machine_status,
			end_history_root: event.assertion.after_state.end_history_root.into(),
		};

		let payload = ArbitrumBoldProof {
			arbitrum_header,
			after_state,
			previous_assertion_hash: event.parent_assertion_hash.into(),
			sequencer_batch_acc: event.after_inbox_batch_acc.into(),
			storage_proof: proof
				.storage_proof
				.get(0)
				.cloned()
				.ok_or_else(|| anyhow!("Storage proof not found for arbitrum assertion hash"))?
				.proof
				.into_iter()
				.map(|node| node.0.into())
				.collect(),
			contract_proof: proof.account_proof.into_iter().map(|node| node.0.into()).collect(),
		};

		Ok(payload)
	}
}
