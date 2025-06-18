use std::sync::Arc;

use crate::ArbHost;
use anyhow::anyhow;

use futures::StreamExt;
use ismp::messaging::CreateConsensusState;
use tesseract_primitives::{IsmpHost, IsmpProvider};

#[async_trait::async_trait]
impl IsmpHost for ArbHost {
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let mut stream = Box::pin(futures::stream::pending::<()>());
		while let Some(_) = stream.next().await {}
		Err(anyhow!(
			"{}-{} consensus task has failed, Please restart relayer",
			self.provider().name(),
			counterparty.name()
		))
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		Ok(None)
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}
