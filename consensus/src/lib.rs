// Copyright (C) Polytope Labs Ltd.
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

//! Consensus message relay

use futures::StreamExt;
use ismp::messaging::Message;
use tesseract_primitives::IsmpHost;

pub async fn relay<A, B>(chain_a: A, chain_b: B) -> Result<(), anyhow::Error>
where
    A: IsmpHost,
    B: IsmpHost,
{
    let mut consensus_a = chain_a.consensus_notification().await;
    let mut consensus_b = chain_b.consensus_notification().await;

    loop {
        tokio::select! {
            result = consensus_a.next() =>  {
                match result {
                    None => break,
                    Some(consensus_message) => {
                        chain_b.submit(vec![Message::Consensus(consensus_message)]).await?;
                        log::info!(
                            target: "hyper-relay",
                            "Submitting consensus update message from {} to {}",
                            chain_a.name(), chain_b.name()
                        );
                    }
                }
            }

            result = consensus_b.next() =>  {
                 match result {
                    None => break,
                    Some(consensus_message) => {
                         chain_a.submit(vec![Message::Consensus(consensus_message)]).await?;
                         log::info!(
                            target: "hyper-relay",
                            "Submitting consensus update message from {} to {}",
                            chain_b.name(), chain_a.name()
                         );
                    }
                }
            }
        }
    }

    Ok(())
}
