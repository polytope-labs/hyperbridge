use std::str::FromStr;
use tendermint::{block::Height, validator::Info as Validator};
use tendermint_rpc::{Client, HttpClient, Paging, Url};

use crate::{error::ProverError, SignedHeader};

pub struct TendermintRpcClient {
	client: HttpClient,
}

impl TendermintRpcClient {
	pub async fn new(url: &str) -> Result<Self, ProverError> {
		let client_url = Url::from_str(url)
			.map_err(|e| ProverError::ConversionError(format!("Invalid URL: {}", e)))?;
		let client =
			HttpClient::new(client_url).map_err(|e| ProverError::NetworkError(e.to_string()))?;

		client
			.health()
			.await
			.map_err(|e| ProverError::NetworkError(format!("Health check failed: {}", e)))?;

		Ok(Self { client })
	}

	pub async fn latest_height(&self) -> Result<u64, ProverError> {
		let status = self.client.status().await?;
		Ok(status.sync_info.latest_block_height.value())
	}

	pub async fn signed_header(&self, height: u64) -> Result<SignedHeader, ProverError> {
		let height =
			Height::try_from(height).map_err(|e| ProverError::InvalidHeight(e.to_string()))?;

		let commit_response = self.client.commit(height).await?;
		Ok(commit_response.signed_header)
	}

	pub async fn validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		let height =
			Height::try_from(height).map_err(|e| ProverError::InvalidHeight(e.to_string()))?;

		let validators_response = self.client.validators(height, Paging::All).await?;

		Ok(validators_response.validators)
	}

	pub async fn next_validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		self.validators(height + 1).await
	}

	/// Get a range of signed headers (ancestry)
	pub async fn signed_headers_range(
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

	pub async fn chain_id(&self) -> Result<String, ProverError> {
		let status = self.client.status().await?;
		Ok(status.node_info.network.to_string())
	}

	pub async fn is_healthy(&self) -> Result<bool, ProverError> {
		match self.client.health().await {
			Ok(_) => Ok(true),
			Err(_) => Ok(false),
		}
	}
}
