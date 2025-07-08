use cometbft::{block::Height, validator::Info as Validator};
use cometbft_rpc::{Client, HttpClient, Paging, Url};

use crate::{error::ProverError, peppermint_rpc::PeppermintRpcClient, SignedHeader};

pub struct TendermintRpcClient {
	client: HttpClient,
	peppermint_client: PeppermintRpcClient,
}

impl TendermintRpcClient {
	pub async fn new(url: &str) -> Result<Self, ProverError> {
		let client_url = url
			.parse::<Url>()
			.map_err(|e| ProverError::ConversionError(format!("Invalid URL: {}", e)))?;
		let client =
			HttpClient::new(client_url).map_err(|e| ProverError::NetworkError(e.to_string()))?;

		let peppermint_client = PeppermintRpcClient::new(url);

		Ok(Self { client, peppermint_client })
	}

	pub async fn latest_height(&self) -> Result<u64, ProverError> {
		match self.client.status().await {
			Ok(status) => Ok(status.sync_info.latest_block_height.value()),
			Err(_) => self.peppermint_client.latest_height().await,
		}
	}

	pub async fn signed_header(&self, height: u64) -> Result<SignedHeader, ProverError> {
		let height =
			Height::try_from(height).map_err(|e| ProverError::InvalidHeight(e.to_string()))?;

		match self.client.commit(height).await {
			Ok(commit_response) => Ok(commit_response.signed_header),
			Err(_) => self.peppermint_client.signed_header(height.value()).await,
		}
	}

	pub async fn validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		let height =
			Height::try_from(height).map_err(|e| ProverError::InvalidHeight(e.to_string()))?;

		match self.client.validators(height, Paging::All).await {
			Ok(validators_response) => Ok(validators_response.validators),
			Err(_) => self.peppermint_client.validators(height.value()).await,
		}
	}

	pub async fn next_validators(&self, height: u64) -> Result<Vec<Validator>, ProverError> {
		self.validators(height + 1).await
	}

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
		match self.client.status().await {
			Ok(status) => Ok(status.node_info.network.to_string()),
			Err(_) => self.peppermint_client.chain_id().await,
		}
	}

	pub async fn is_healthy(&self) -> Result<bool, ProverError> {
		match self.client.health().await {
			Ok(_) => Ok(true),
			Err(_) => self.peppermint_client.is_healthy().await,
		}
	}
}
