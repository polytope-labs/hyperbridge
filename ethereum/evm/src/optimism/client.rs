use crate::{EvmClient, EvmConfig};
use anyhow::anyhow;
use consensus_client::{optimism::OptimismPayloadProof, presets::L2_OUTPUTS_SLOT};
use ethabi::ethereum_types::{H256, U256};
use ethers::{
	prelude::Provider,
	providers::{Middleware, Ws},
	types::H160,
};
use serde::{Deserialize, Serialize};
use sp_core::keccak_256;
use std::sync::Arc;
use tesseract_primitives::IsmpProvider;

use crate::abi::l2_output_oracle::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpConfig {
	/// WS url for the beacon execution client
	pub beacon_execution_ws: String,
	/// L2Oracle contract address on L1
	pub l2_oracle: H160,
	/// Withdrawals Message Passer contract address on L2
	pub message_parser: H160,
	/// General Evm client config
	#[serde[flatten]]
	pub evm_config: EvmConfig,
}

impl OpConfig {
	/// Convert the config into a client.
	pub async fn into_client<C: IsmpProvider>(
		self,
		counterparty: &C,
	) -> anyhow::Result<EvmClient<OpHost>> {
		let host = OpHost::new(&self).await?;
		let client = EvmClient::new(host, self.evm_config, counterparty).await?;

		Ok(client)
	}
}

#[derive(Clone)]
pub struct OpHost {
	/// Optimism stack execution client
	pub(crate) op_execution_client: Arc<Provider<Ws>>,
	/// Beacon execution client
	pub(crate) beacon_execution_client: Arc<Provider<Ws>>,
	/// L2Oracle contract address on L1
	pub(crate) l2_oracle: H160,
	/// Withdrawals Message Passer contract address on L2
	pub(crate) message_parser: H160,
	/// Config
	pub config: OpConfig,
}

pub fn derive_array_item_key(index_in_array: u64, offset: u64) -> H256 {
	let mut bytes = [0u8; 32];
	U256::from(L2_OUTPUTS_SLOT as u64).to_big_endian(&mut bytes);

	let hash_result = keccak_256(&bytes);

	let array_pos = U256::from_big_endian(&hash_result);
	let item_pos = array_pos + U256::from(index_in_array * 2) + U256::from(offset);

	let mut pos = [0u8; 32];
	item_pos.to_big_endian(&mut pos);

	pos.into()
}

impl OpHost {
	pub async fn new(config: &OpConfig) -> Result<Self, anyhow::Error> {
		let provider =
			Provider::<Ws>::connect_with_reconnects(&config.evm_config.execution_ws, 1000).await?;
		let beacon_client =
			Provider::<Ws>::connect_with_reconnects(&config.beacon_execution_ws, 1000).await?;
		Ok(Self {
			op_execution_client: Arc::new(provider),
			beacon_execution_client: Arc::new(beacon_client),
			l2_oracle: config.l2_oracle,
			message_parser: config.message_parser,
			config: config.clone(),
		})
	}

	pub async fn latest_event(
		&self,
		from: u64,
		to: u64,
	) -> Result<Option<OutputProposedFilter>, anyhow::Error> {
		let client = Arc::new(self.beacon_execution_client.clone());
		let contract = L2OutputOracle::new(self.l2_oracle, client);
		let mut events = contract
			.event::<OutputProposedFilter>()
			.address(self.l2_oracle.into())
			.from_block(from)
			.to_block(to)
			.query()
			.await?
			.into_iter()
			.collect::<Vec<_>>();

		events.sort_unstable_by(|a, b| a.l_2_output_index.cmp(&b.l_2_output_index));

		Ok(events.last().cloned())
	}

	pub async fn fetch_op_payload(
		&self,
		at: u64,
		event: OutputProposedFilter,
	) -> Result<OptimismPayloadProof, anyhow::Error> {
		let output_roots_key = derive_array_item_key(event.l_2_output_index.low_u64(), 0);
		let timestamp_and_block_proof = derive_array_item_key(event.l_2_output_index.low_u64(), 1);

		let proof = self
			.beacon_execution_client
			.get_proof(
				self.l2_oracle,
				vec![output_roots_key, timestamp_and_block_proof],
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
			.get_proof(self.message_parser, vec![], Some(event.l_2_block_number.low_u64().into()))
			.await?;

		let payload = OptimismPayloadProof {
			state_root: block.state_root,
			withdrawal_storage_root: message_parser_proof.storage_hash,
			l2_block_hash: block.hash.ok_or_else(|| anyhow!("Missing optimism block hash"))?,
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
