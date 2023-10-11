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

use crate::ParachainHost;
use futures::stream;
use ismp::messaging::ConsensusMessage;
use tesseract_primitives::{BoxStream, IsmpHost, IsmpProvider};

#[async_trait::async_trait]
impl IsmpHost for ParachainHost {
	async fn consensus_notification<C>(
		&self,
		_counterparty: C,
	) -> Result<BoxStream<ConsensusMessage>, anyhow::Error>
	where
		C: IsmpHost + IsmpProvider + 'static,
	{
		// use the inherent provider
		Ok(Box::pin(stream::pending()))
	}
}
