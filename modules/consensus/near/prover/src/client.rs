use async_trait::async_trait;
use near_jsonrpc_client::{methods, JsonRpcClient};
use near_jsonrpc_primitives::types::light_client::RpcLightClientExecutionProofResponse;
use near_primitives::{
	hash::CryptoHash,
	types::{BlockId, BlockReference, TransactionOrReceiptId},
	views::{validator_stake_view::ValidatorStakeView, BlockHeaderView, LightClientBlockView},
};
use near_primitives_ismp::prover::{Client, ProverError};
use std::fmt::{Display, Formatter};

/// Represents different NEAR networks
#[derive(Debug, Clone, Copy)]
pub enum Network {
	Mainnet,
	Testnet,
	Localnet,
	Statelessnet,
}

impl Network {
	pub fn to_endpoint(&self) -> &str {
		const MAINNET_RPC_ENDPOINT: &str = "https://rpc.mainnet.near.org";
		const TESTNET_RPC_ENDPOINT: &str = "https://rpc.testnet.near.org";
		match self {
			Self::Mainnet => MAINNET_RPC_ENDPOINT,
			Self::Testnet => TESTNET_RPC_ENDPOINT,
			Self::Statelessnet => "https://rpc.statelessnet.near.org",
			Self::Localnet => "http://localhost:3030",
		}
	}

	pub fn archive_endpoint(&self) -> &str {
		const MAINNET_RPC_ARCHIVE_ENDPOINT: &str = "https://archival-rpc.mainnet.near.org";
		const TESTNET_RPC_ARCHIVE_ENDPOINT: &str = "https://archival-rpc.testnet.near.org";
		match self {
			Self::Mainnet => MAINNET_RPC_ARCHIVE_ENDPOINT,
			Self::Testnet => TESTNET_RPC_ARCHIVE_ENDPOINT,
			Self::Statelessnet => "https://archival-rpc.statelessnet.near.org",
			Self::Localnet => "http://localhost:3030",
		}
	}
}

impl Display for Network {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let s = match self {
			Self::Mainnet => "mainnet",
			Self::Testnet => "testnet",
			Self::Statelessnet => "statelessnet",
			Self::Localnet => "localnet",
		};
		write!(f, "{}", s)
	}
}

/// Configuration for NEAR RPC client
#[derive(Debug, Clone)]
pub struct Config {
	pub network: Network,
}

impl Config {
	pub fn new(network: Network) -> Self {
		Self { network }
	}
}

impl From<Network> for Config {
	fn from(network: Network) -> Self {
		Config { network }
	}
}

/// A client implementation for interacting with NEAR nodes.
///
/// This client uses the official NEAR JSON-RPC client to communicate with
/// NEAR blockchain nodes.
#[derive(Clone)]
pub struct NearRpcClient {
	client: JsonRpcClient,
	archive: JsonRpcClient,
}

impl std::fmt::Debug for NearRpcClient {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("NearRpcClient").finish()
	}
}

impl NearRpcClient {
	/// Creates a new NEAR RPC client instance.
	///
	/// # Arguments
	///
	/// * `config` - The configuration containing network details
	///
	/// # Returns
	///
	/// A new NEAR RPC client instance
	pub fn new(config: &Config) -> Self {
		let client = JsonRpcClient::connect(config.network.to_endpoint());
		let archive = JsonRpcClient::connect(config.network.archive_endpoint());

		NearRpcClient { client, archive }
	}

	/// Fetch a block by its reference (hash or height).
	pub async fn fetch_block(
		&self,
		block_reference: BlockReference,
	) -> Result<near_primitives::views::BlockView, ProverError> {
		let req = methods::block::RpcBlockRequest { block_reference };
		self.client
			.call(&req)
			.await
			.or_else(|e| {
				tracing::trace!("Error hitting main rpc, falling back to archive: {:?}", e);
				self.archive.call(&req)
			})
			.await
			.map_err(|e| ProverError::RpcError(format!("Failed to fetch block: {:?}", e)))
	}

	/// Fetch a block header by block hash.
	async fn fetch_header_internal(
		&self,
		hash: &CryptoHash,
	) -> Result<BlockHeaderView, ProverError> {
		let req = methods::block::RpcBlockRequest {
			block_reference: BlockReference::BlockId(BlockId::Hash(*hash)),
		};
		self.client
			.call(&req)
			.await
			.or_else(|e| {
				tracing::trace!("Error hitting main rpc, falling back to archive: {:?}", e);
				self.archive.call(&req)
			})
			.await
			.map_err(|e| ProverError::RpcError(format!("Failed to fetch header: {:?}", e)))
			.map(|x| x.header)
	}

	/// Fetch execution proof for a transaction or receipt.
	pub async fn fetch_light_client_proof(
		&self,
		req: TransactionOrReceiptId,
		latest_verified: CryptoHash,
	) -> Result<RpcLightClientExecutionProofResponse, ProverError> {
		let req = methods::light_client_proof::RpcLightClientExecutionProofRequest {
			id: req.clone(),
			light_client_head: latest_verified,
		};
		tracing::debug!("requesting proof: {:?}", req);
		self.client
			.call(&req)
			.await
			.or_else(|e| {
				tracing::trace!("Error hitting main rpc, falling back to archive: {:?}", e);
				self.archive.call(&req)
			})
			.await
			.map_err(|e| ProverError::RpcError(format!("Failed to fetch proof: {:?}", e)))
	}

	/// Fetch block producers (validators) for a given epoch.
	async fn fetch_epoch_validators_internal(
		&self,
		epoch_id: &CryptoHash,
	) -> Result<Vec<ValidatorStakeView>, ProverError> {
		let req = methods::next_light_client_block::RpcLightClientNextBlockRequest {
			last_block_hash: *epoch_id,
		};
		tracing::debug!("requesting validators: {:?}", req);
		self.client
			.call(&req)
			.await
			.or_else(|e| {
				tracing::trace!("Error hitting main rpc, falling back to archive: {:?}", e);
				self.archive.call(&req)
			})
			.await
			.map_err(|e| ProverError::RpcError(format!("Failed to fetch validators: {:?}", e)))
			.and_then(|x| {
				x.ok_or_else(|| {
					ProverError::NoValidators(format!("No block found for epoch {:?}", epoch_id))
				})
			})
			.and_then(|x| {
				x.next_bps.ok_or_else(|| {
					ProverError::NoValidators(format!(
						"No validators found for epoch {:?}",
						epoch_id
					))
				})
			})
	}
}

#[async_trait]
impl Client for NearRpcClient {
	async fn latest_height(&self) -> Result<u64, ProverError> {
		let req = methods::status::RpcStatusRequest;
		self.client
			.call(&req)
			.await
			.or_else(|e| {
				tracing::trace!("Error hitting main rpc, falling back to archive: {:?}", e);
				self.archive.call(&req)
			})
			.await
			.map_err(|e| ProverError::RpcError(format!("Failed to fetch status: {:?}", e)))
			.map(|status| status.sync_info.latest_block_height)
	}

	async fn next_light_client_block(
		&self,
		last_block_hash: CryptoHash,
	) -> Result<Option<LightClientBlockView>, ProverError> {
		let req =
			methods::next_light_client_block::RpcLightClientNextBlockRequest { last_block_hash };
		tracing::debug!("requesting next block: {:?}", req);
		self.client
			.call(&req)
			.await
			.or_else(|e| {
				tracing::trace!("Error hitting main rpc, falling back to archive: {:?}", e);
				self.archive.call(&req)
			})
			.await
			.map_err(|e| ProverError::RpcError(format!("Failed to fetch next block: {:?}", e)))
	}

	async fn light_client_block(
		&self,
		block_hash: CryptoHash,
	) -> Result<LightClientBlockView, ProverError> {
		// First get the next block after this one, which will contain the approvals
		let next_block = self.next_light_client_block(block_hash).await?;
		next_block.ok_or_else(|| {
			ProverError::NoLightClientBlock(format!(
				"No light client block found for {:?}",
				block_hash
			))
		})
	}

	async fn validators(
		&self,
		epoch_id: CryptoHash,
	) -> Result<Vec<ValidatorStakeView>, ProverError> {
		self.fetch_epoch_validators_internal(&epoch_id).await
	}

	async fn block_header(&self, block_hash: CryptoHash) -> Result<BlockHeaderView, ProverError> {
		self.fetch_header_internal(&block_hash).await
	}

	async fn is_healthy(&self) -> Result<bool, ProverError> {
		let req = methods::health::RpcHealthRequest;
		match self.client.call(&req).await {
			Ok(_) => Ok(true),
			Err(_) => Ok(false),
		}
	}
}

#[cfg(test)]
mod tests {
	// Latest height
	#[tokio::test]
	async fn test_latest_height() {
		let client = NearRpcClient::new(&Config::new(Network::Testnet));
		let height = client.latest_height().await.unwrap();
		println!("Latest height: {}", height);
	}
}
