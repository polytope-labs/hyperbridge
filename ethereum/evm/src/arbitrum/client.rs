use crate::{abi::i_rollup::*, derive_map_key, EvmClient, EvmConfig};
use anyhow::anyhow;
use ethabi::ethereum_types::U256;
use ethers::{
	prelude::Provider,
	providers::{Middleware, Ws},
	types::{H160, H256},
};
use geth_primitives::CodecHeader;
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use ismp_sync_committee::{
	arbitrum::{ArbitrumPayloadProof, GlobalState as RustGlobalState},
	presets::NODES_SLOT,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbConfig {
	/// WS URL url for beacon execution client
	pub beacon_execution_ws: String,
	/// RollupCore contract address on L1
	pub rollup_core: H160,
	/// General evm config
	#[serde[flatten]]
	pub evm_config: EvmConfig,
}

impl ArbConfig {
	/// Convert the config into a client.
	pub async fn into_client(self) -> anyhow::Result<EvmClient<ArbHost>> {
		let host = ArbHost::new(&self).await?;
		let client = EvmClient::new(host, self.evm_config).await?;

		Ok(client)
	}

	pub fn state_machine(&self) -> StateMachine {
		self.evm_config.state_machine
	}
}

#[derive(Clone)]
pub struct ArbHost {
	/// Arbitrum execution client
	pub(crate) arb_execution_client: Arc<Provider<Ws>>,
	/// Beacon execution client
	pub(crate) beacon_execution_client: Arc<Provider<Ws>>,
	/// Rollup core contract address
	pub(crate) rollup_core: H160,
	/// Config
	pub config: ArbConfig,
	/// Consensus State Id
	pub consensus_state_id: ConsensusStateId,
}

impl ArbHost {
	pub async fn new(config: &ArbConfig) -> Result<Self, anyhow::Error> {
		let provider =
			Provider::<Ws>::connect_with_reconnects(&config.evm_config.execution_ws, 1000).await?;
		let beacon_client =
			Provider::<Ws>::connect_with_reconnects(&config.beacon_execution_ws, 1000).await?;
		Ok(Self {
			arb_execution_client: Arc::new(provider),
			beacon_execution_client: Arc::new(beacon_client),
			rollup_core: config.rollup_core,
			config: config.clone(),
			consensus_state_id: {
				let mut consensus_state_id: ConsensusStateId = Default::default();
				consensus_state_id.copy_from_slice(config.evm_config.consensus_state_id.as_bytes());
				consensus_state_id
			},
		})
	}

	async fn fetch_header(&self, block: H256) -> Result<CodecHeader, anyhow::Error> {
		let block = self
			.arb_execution_client
			.get_block(block)
			.await?
			.ok_or_else(|| anyhow!("Header not found for {:?}", block))?;
		let arb_header = block.into();

		Ok(arb_header)
	}

	pub async fn latest_event(
		&self,
		from: u64,
		to: u64,
	) -> Result<Option<NodeCreatedFilter>, anyhow::Error> {
		let client = Arc::new(self.beacon_execution_client.clone());
		let contract = IRollup::new(self.rollup_core, client);
		let mut events = contract
			.event::<NodeCreatedFilter>()
			.address(self.rollup_core.into())
			.from_block(from)
			.to_block(to)
			.query()
			.await?
			.into_iter()
			.collect::<Vec<_>>();

		events.sort_unstable_by(|a, b| a.node_num.cmp(&b.node_num));

		Ok(events.last().cloned())
	}

	pub async fn fetch_arbitrum_payload(
		&self,
		at: u64,
		event: NodeCreatedFilter,
	) -> Result<ArbitrumPayloadProof, anyhow::Error> {
		let mut node_num = [0u8; 32];
		U256::from(event.node_num).to_big_endian(&mut node_num);
		let state_hash_key = derive_map_key(node_num.to_vec(), NODES_SLOT as u64);
		let proof = self
			.beacon_execution_client
			.get_proof(self.rollup_core, vec![state_hash_key], Some(at.into()))
			.await?;
		let arb_block_hash = event.assertion.after_state.global_state.bytes_32_vals[0].into();
		let arbitrum_header = self.fetch_header(arb_block_hash).await?;
		let payload = ArbitrumPayloadProof {
			arbitrum_header,
			global_state: RustGlobalState {
				block_hash: arb_block_hash,
				send_root: event.assertion.after_state.global_state.bytes_32_vals[1].into(),
				inbox_position: event.assertion.after_state.global_state.u_64_vals[0],
				position_in_message: event.assertion.after_state.global_state.u_64_vals[1],
			},
			machine_status: {
				let status = event.assertion.after_state.machine_status;
				status.try_into().map_err(|e| anyhow!("{:?}", e))?
			},
			inbox_max_count: event.inbox_max_count,
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
}
