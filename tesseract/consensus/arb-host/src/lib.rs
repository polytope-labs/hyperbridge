use abi::{IRollup, IRollupBold};
use alloy::{
	eips::BlockId,
	primitives::{Address, B256},
	providers::{Provider, ProviderBuilder},
	rpc::types::Filter,
	sol_types::SolEvent,
};
use anyhow::anyhow;
use arbitrum_verifier::{
	ArbitrumBoldProof, ArbitrumPayloadProof, AssertionState, GlobalState as RustGlobalState,
	ASSERTIONS_SLOT, NODES_SLOT,
};
use geth_primitives::{alloy_u256_to_primitive, CodecHeader};
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use primitive_types::{H160, H256, U256};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tesseract_evm::{derive_map_key, AlloyProvider, EvmClient, EvmConfig};
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
	pub arb_execution_client: Arc<AlloyProvider>,
	/// Beacon execution client
	pub(crate) beacon_execution_client: Arc<AlloyProvider>,
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
		let arb_rpc_url = evm
			.rpc_urls
			.first()
			.ok_or_else(|| anyhow!("No RPC URLs provided for Arbitrum"))?;
		let el = ProviderBuilder::new().connect_http(arb_rpc_url.parse()?);

		let beacon_rpc_url = host
			.ethereum_rpc_url
			.first()
			.ok_or_else(|| anyhow!("No RPC URLs provided for beacon client"))?;
		let beacon_client = ProviderBuilder::new().connect_http(beacon_rpc_url.parse()?);

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
		let block_hash = B256::from_slice(&block.0);
		let block = self
			.arb_execution_client
			.get_block(BlockId::hash(block_hash))
			.await?
			.ok_or_else(|| {
				anyhow!("{} Header not found for {:?}", self.evm.state_machine, block_hash)
			})?;
		let arb_header = block.into();

		Ok(arb_header)
	}

	pub async fn latest_event(
		&self,
		from: u64,
		to: u64,
	) -> Result<Option<IRollup::NodeCreated>, anyhow::Error> {
		if from > to {
			return Ok(None);
		}
		let rollup_addr = Address::from_slice(&self.rollup_core.0);
		let filter = Filter::new().address(rollup_addr).from_block(from).to_block(to);

		let logs = self.beacon_execution_client.get_logs(&filter).await?;

		let mut events: Vec<IRollup::NodeCreated> = logs
			.into_iter()
			.filter_map(|log| IRollup::NodeCreated::decode_log(&log.inner).ok())
			.map(|log| log.data)
			.collect();

		events.sort_unstable_by(|a, b| a.nodeNum.cmp(&b.nodeNum));

		Ok(events.last().cloned())
	}

	pub async fn latest_assertion_event(
		&self,
		from: u64,
		to: u64,
	) -> Result<Option<IRollupBold::AssertionCreated>, anyhow::Error> {
		if from > to {
			return Ok(None);
		}
		let rollup_addr = Address::from_slice(&self.rollup_core.0);
		let filter = Filter::new().address(rollup_addr).from_block(from).to_block(to);

		let logs = self.beacon_execution_client.get_logs(&filter).await?;

		let events: Vec<IRollupBold::AssertionCreated> = logs
			.into_iter()
			.filter_map(|log| IRollupBold::AssertionCreated::decode_log(&log.inner).ok())
			.map(|log| log.data)
			.collect();

		Ok(events.last().cloned())
	}

	pub async fn fetch_arbitrum_payload(
		&self,
		at: u64,
		event: IRollup::NodeCreated,
	) -> Result<ArbitrumPayloadProof, anyhow::Error> {
		let node_num = U256::from(event.nodeNum).to_big_endian();
		let state_hash_key = derive_map_key(node_num.to_vec(), NODES_SLOT as u64);
		let rollup_addr = Address::from_slice(&self.rollup_core.0);
		let proof = self
			.beacon_execution_client
			.get_proof(rollup_addr, vec![B256::from_slice(&state_hash_key.0)])
			.block_id(at.into())
			.await?;
		let arb_block_hash: H256 = event.assertion.afterState.globalState.bytes32Vals[0].0.into();
		let arbitrum_header = self.fetch_header(arb_block_hash).await?;
		let payload = ArbitrumPayloadProof {
			arbitrum_header,
			global_state: RustGlobalState {
				block_hash: arb_block_hash.0.into(),
				send_root: event.assertion.afterState.globalState.bytes32Vals[1].0.into(),
				inbox_position: event.assertion.afterState.globalState.u64Vals[0],
				position_in_message: event.assertion.afterState.globalState.u64Vals[1],
			},
			machine_status: {
				let status = event.assertion.afterState.machineStatus;
				status.try_into().map_err(|e| anyhow!("{:?}", e))?
			},
			inbox_max_count: alloy_u256_to_primitive(event.inboxMaxCount),
			node_number: event.nodeNum,
			storage_proof: proof
				.storage_proof
				.first()
				.cloned()
				.ok_or_else(|| anyhow!("Storage proof not found for arbitrum state_hash"))?
				.proof
				.into_iter()
				.map(|node| node.to_vec())
				.collect(),
			contract_proof: proof.account_proof.into_iter().map(|node| node.to_vec()).collect(),
		};

		Ok(payload)
	}

	pub async fn fetch_arbitrum_bold_payload(
		&self,
		at: u64,
		event: IRollupBold::AssertionCreated,
	) -> Result<ArbitrumBoldProof, anyhow::Error> {
		let assertion_hash_key =
			derive_map_key(event.assertionHash.0.to_vec(), ASSERTIONS_SLOT as u64);
		let rollup_addr = Address::from_slice(&self.rollup_core.0);
		let proof = self
			.beacon_execution_client
			.get_proof(rollup_addr, vec![B256::from_slice(&assertion_hash_key.0)])
			.block_id(at.into())
			.await?;
		let arb_block_hash: H256 = event.assertion.afterState.globalState.bytes32Vals[0].0.into();
		let arbitrum_header = self.fetch_header(arb_block_hash).await?;
		let global_state = RustGlobalState {
			block_hash: arb_block_hash.0.into(),
			send_root: event.assertion.afterState.globalState.bytes32Vals[1].0.into(),
			inbox_position: event.assertion.afterState.globalState.u64Vals[0],
			position_in_message: event.assertion.afterState.globalState.u64Vals[1],
		};

		let machine_status = event
			.assertion
			.afterState
			.machineStatus
			.try_into()
			.map_err(|_| anyhow!("Failed conversion"))?;

		let after_state = AssertionState {
			global_state,
			machine_status,
			end_history_root: event.assertion.afterState.endHistoryRoot.0.into(),
		};

		let payload = ArbitrumBoldProof {
			arbitrum_header,
			after_state,
			previous_assertion_hash: event.parentAssertionHash.0.into(),
			sequencer_batch_acc: event.afterInboxBatchAcc.0.into(),
			storage_proof: proof
				.storage_proof
				.first()
				.cloned()
				.ok_or_else(|| anyhow!("Storage proof not found for arbitrum assertion hash"))?
				.proof
				.into_iter()
				.map(|node| node.to_vec())
				.collect(),
			contract_proof: proof.account_proof.into_iter().map(|node| node.to_vec()).collect(),
		};

		Ok(payload)
	}
}
