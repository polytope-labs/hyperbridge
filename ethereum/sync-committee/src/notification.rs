use crate::SyncCommitteeHost;
use codec::Decode;
use consensus_client::types::{BeaconClientUpdate, ConsensusState};
use ismp::host::{Ethereum, StateMachine};
use primitives::{consensus_types::Checkpoint, constants::Root};
use std::collections::BTreeMap;
use tesseract_primitives::{IsmpHost, IsmpProvider};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EventResponse {
    pub block: Root,
    pub state: Root,
    pub epoch: String,
    pub execution_optimistic: bool,
}

pub async fn consensus_notification<C>(
    client: &SyncCommitteeHost,
    counterparty: C,
    checkpoint: Checkpoint,
) -> Result<Option<BeaconClientUpdate>, anyhow::Error>
where
    C: IsmpHost + IsmpProvider + 'static,
{
    let consensus_state =
        counterparty.query_consensus_state(None, client.consensus_state_id).await?;
    let consensus_state = ConsensusState::decode(&mut &*consensus_state)?;
    let light_client_state = consensus_state.light_client_state;

    let consensus_update = if let Some(update) = client
        .prover
        .fetch_light_client_update(light_client_state.clone(), checkpoint, "tesseract")
        .await?
    {
        update
    } else {
        return Ok(None)
    };

    let arbitrum_payload = if let Some(arb_client) = client.arbitrum_client.as_ref() {
        let latest_event = arb_client
            .latest_event(
                light_client_state.finalized_header.slot,
                consensus_update.finalized_header.slot,
            )
            .await?;
        if let Some(event) = latest_event {
            arb_client
                .fetch_arbitrum_payload(consensus_update.finalized_header.slot, event)
                .await
                .ok()
        } else {
            None
        }
    } else {
        None
    };

    let mut op_stack_payload = BTreeMap::new();
    if let Some(op_client) = client.optimism_client.as_ref() {
        let latest_event = op_client
            .latest_event(
                light_client_state.finalized_header.slot,
                consensus_update.finalized_header.slot,
            )
            .await?;
        if let Some(event) = latest_event {
            let payload =
                op_client.fetch_op_payload(consensus_update.finalized_header.slot, event).await?;
            op_stack_payload.insert(StateMachine::Ethereum(Ethereum::Optimism), payload);
        }
    }

    if let Some(base_client) = client.base_client.as_ref() {
        let latest_event = base_client
            .latest_event(
                light_client_state.finalized_header.slot,
                consensus_update.finalized_header.slot,
            )
            .await?;
        if let Some(event) = latest_event {
            let payload =
                base_client.fetch_op_payload(consensus_update.finalized_header.slot, event).await?;
            op_stack_payload.insert(StateMachine::Ethereum(Ethereum::Base), payload);
        }
    }

    let message = BeaconClientUpdate { consensus_update, op_stack_payload, arbitrum_payload };

    Ok(Some(message))
}
