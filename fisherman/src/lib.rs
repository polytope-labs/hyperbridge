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

//! Tesseract Fisherman

use anyhow::anyhow;
use futures::{future::Either, StreamExt, TryFutureExt};
use tesseract_primitives::{IsmpHost, IsmpProvider};

pub async fn fish<A, B>(chain_a: A, chain_b: B) -> Result<(), anyhow::Error>
where
    A: IsmpHost + 'static,
    B: IsmpHost + 'static,
{
    let task_a = {
        let chain_a = chain_a.clone();
        let chain_b = chain_b.clone();
        Box::pin(handle_notification(chain_a, chain_b))
    };

    let task_b = {
        let chain_a = chain_a.clone();
        let chain_b = chain_b.clone();
        Box::pin(handle_notification(chain_b, chain_a))
    };

    // if one task completes, abort the other
    let err = match futures::future::select(task_a, task_b).await {
        Either::Left((res, _task)) => res,
        Either::Right((res, _task)) => res,
    };

    log::error!("{:?}", err);

    Ok(())
}

async fn handle_notification<A, B>(chain_a: A, chain_b: B) -> Result<(), anyhow::Error>
where
    A: IsmpHost + 'static,
    B: IsmpHost + 'static,
{
    let mut state_machine_update_stream = chain_a
        .provider()
        .state_machine_update_notification(chain_b.provider().state_machine_id())
        .await
        .map_err(|err| anyhow!("StateMachineUpdated stream subscription failed: {err:?}"))?;
    let chain_a_name = chain_a.provider().name();
    let chain_b_name = chain_b.provider().name();

    while let Some(item) = state_machine_update_stream.next().await {
        match item {
            Ok(state_machine_update) => {
                let chain_b_clone = chain_b.clone();
                let chain_a_clone = chain_a.clone();
                let fut = chain_b
                    .query_consensus_message(state_machine_update)
                    .and_then(|message| async move {
                        // chain_b_clone.check_for_byzantine_attack(&*chain_a_clone.provider(), message).await
                        Ok(())
                    });

                if let Err(err) = fut.await {
                    log::error!("Failed to check for byzantine behavior: {err:?}")
                }
            }
            Err(e) => {
                log::error!(target: "tesseract","Fisherman task {}-{} encountered an error: {e:?}", chain_a_name, chain_b_name)
            }
        }
    }

    Err(anyhow!(
        "{}-{} fisherman task has failed, Please restart relayer",
        chain_a_name,
        chain_b_name
    ))?
}
