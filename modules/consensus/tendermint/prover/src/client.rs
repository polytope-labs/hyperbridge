use crate::SignedHeader;
use async_trait::async_trait;
use cometbft::{block::Height, validator::Info as Validator};
use cometbft_rpc::{
	endpoint::abci_query::AbciQuery, Client as OtherClient, HttpClient, Paging, Url,
};
use tendermint_primitives::{Client, ProverError};

/// A client implementation for interacting with CometBFT nodes.
///
/// This client uses the official CometBFT RPC client to communicate with
/// Tendermint-compatible blockchain nodes that support the standard RPC interface.
pub struct CometBFTClient {
	client: HttpClient,
}

// Key trait and chain implementations are defined in the `keys` module.

impl CometBFTClient {
	/// Creates a new CometBFT client instance.
	///
	/// # Arguments
	///
	/// * `url` - The RPC endpoint URL of the CometBFT node
	///
	/// # Returns
	///
	/// - `Ok(Self)`: A new CometBFT client instance
	/// - `Err(ProverError)`: If the URL is invalid or the client cannot be created
	pub async fn new(url: &str) -> Result<Self, ProverError> {
		let client_url = url
			.parse::<Url>()
			.map_err(|e| ProverError::ConversionError(format!("Invalid URL: {}", e)))?;

		let client =
			HttpClient::new(client_url).map_err(|e| ProverError::NetworkError(e.to_string()))?;

		Ok(Self { client })
	}

	/// Perform a generic ABCI query for a single key under a module store, returning an ICS23 proof.
	///
	/// - `store_key`: the Cosmos SDK module store key (e.g., "evm"). This is the same string
	///   used by the module when creating its KVStore (Sei EVM uses `StoreKey = "evm"`).
	///   The ABCI path becomes `/store/{store_key}/key`.
	/// - `key`: raw key bytes within that store (including any module-defined prefix).
	///   Example: For Sei EVM, common keys are:
	///   - Nonce: `0x0a || <20-byte evm_address>` (prefix `NonceKeyPrefix`)
	///   - Code hash: `0x08 || <20-byte evm_address>` (prefix `CodeHashKeyPrefix`)
	///   - Code: `0x07 || <20-byte evm_address>` (prefix `CodeKeyPrefix`)
	///   - Storage slot: `0x03 || <20-byte evm_address> || <32-byte storage_key>` (prefix `StateKeyPrefix`)
	///   where the 20-byte address is the Ethereum address bytes and the 32-byte storage key
	///   is the Keccak-256 slot key.
	/// - `height`: consensus height to query at (must match a height you have a verified header for).
	///
	/// Returns the ABCI response including `proof_ops` (ICS23). Verify this proof against the
	/// Tendermint app hash you obtained from the verified signed header at `height`.
	pub async fn abci_query_key(
		&self,
		store_key: &str,
		key: Vec<u8>,
		height: u64,
	) -> Result<AbciQuery, ProverError> {
		let height =
			Height::try_from(height).map_err(|e| ProverError::InvalidHeight(e.to_string()))?;
		self.client
			.abci_query(Some(format!("/store/{}/key", store_key)), key, Some(height), true)
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))
	}
}

#[async_trait]
impl Client for CometBFTClient {
	async fn latest_height(&self) -> Result<u64, ProverError> {
		let status = self
			.client
			.status()
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;
		Ok(status.sync_info.latest_block_height.value())
	}

	async fn signed_header(&self, height: u64) -> Result<SignedHeader, ProverError> {
		let height =
			Height::try_from(height).map_err(|e| ProverError::InvalidHeight(e.to_string()))?;
		let commit_response = self
			.client
			.commit(height)
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;
		Ok(commit_response.signed_header)
	}

	async fn validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		let height =
			Height::try_from(height).map_err(|e| ProverError::InvalidHeight(e.to_string()))?;
		let validators_response = self
			.client
			.validators(height, Paging::All)
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;
		Ok(validators_response.validators)
	}

	async fn next_validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		self.validators(height + 1).await
	}

	async fn chain_id(&self) -> Result<String, ProverError> {
		let status = self
			.client
			.status()
			.await
			.map_err(|e| ProverError::NetworkError(e.to_string()))?;
		Ok(status.node_info.network.to_string())
	}

	async fn is_healthy(&self) -> Result<bool, ProverError> {
		match self.client.health().await {
			Ok(_) => Ok(true),
			Err(_) => Ok(false),
		}
	}
}
