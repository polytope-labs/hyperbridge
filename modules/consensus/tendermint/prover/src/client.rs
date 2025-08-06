use crate::SignedHeader;
use async_trait::async_trait;
use cometbft::{block::Height, validator::Info as Validator};
use cometbft_rpc::{Client as OtherClient, HttpClient, Paging, Url};
use tendermint_primitives::{Client, ProverError};

/// A client implementation for interacting with CometBFT nodes.
///
/// This client uses the official CometBFT RPC client to communicate with
/// Tendermint-compatible blockchain nodes that support the standard RPC interface.
pub struct CometBFTClient {
	client: HttpClient,
}

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

	async fn signed_headers_range(
		&self,
		start_height: u64,
		end_height: u64,
	) -> Result<Vec<SignedHeader>, ProverError> {
		if start_height >= end_height {
			return Err(ProverError::InvalidHeight(
				"Start height must be less than end height".to_string(),
			));
		}

		let mut headers = Vec::new();
		for height in start_height..=end_height {
			let header = self.signed_header(height).await?;
			headers.push(header);
		}

		Ok(headers)
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
