// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::{anyhow, Error};
use ismp::messaging::CreateConsensusState;
use std::sync::Arc;

use crate::EvmHost;
use tesseract_primitives::{IsmpHost, IsmpProvider};

#[async_trait::async_trait]
impl IsmpHost for EvmHost {
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		log::info!(
			target: "tesseract",
			"Starting EVM Host dummy consensus client for {} -> {}",
			self.provider().name(),
			counterparty.name()
		);

		log::info!(
			target: "tesseract",
			"⚠️  EVM Host is a dummy consensus client - it will not produce any consensus updates"
		);

		// This is a dummy consensus client that never produces updates
		// We just wait indefinitely
		let forever = std::future::pending::<()>();
		forever.await;

		// This should never be reached, but if it is, return an error
		Err(anyhow!(
			"{}-{} EVM Host consensus task has unexpectedly terminated",
			self.provider().name(),
			counterparty.name()
		))
	}

	async fn query_initial_consensus_state(&self) -> Result<Option<CreateConsensusState>, Error> {
		log::info!(
			target: "tesseract",
			"EVM Host is a dummy consensus client - no initial consensus state available"
		);
		
		// Dummy consensus client does not provide an initial consensus state
		// The consensus is managed elsewhere (e.g., by a relay chain)
		// and does not need to be initialized on counterparty chains
		Ok(None)
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}