use crate::{
    codegen::parachain::api::ismp::events::StateMachineUpdated as StateMachineUpdatedEvent,
    ParachainClient,
};
use codec::Encode;
use futures::{Stream, StreamExt};
use ismp::consensus::StateMachineId;
use std::pin::Pin;
use subxt::config::Header;
use tesseract_primitives::StateMachineUpdated;

impl<T> ParachainClient<T>
where
    T: subxt::Config + Send + Sync + Clone,
    T::Header: Send + Sync,
{
    pub async fn state_machine_update_notification_stream(
        &self,
        counterparty_state_id: StateMachineId,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StateMachineUpdated, anyhow::Error>> + Send>>,
        anyhow::Error,
    > {
        let client = self.parachain.clone();

        let subscription = self.parachain.rpc().subscribe_best_block_headers().await?;

        let stream = subscription.filter_map(move |header| {
            let client = client.clone();
            async move {
                let events = client.events().at(header.ok()?.hash()).await.ok()?;

                let event = events
                    .find::<StateMachineUpdatedEvent>()
                    .find(|ev| match ev {
                        Ok(StateMachineUpdatedEvent { state_machine_id, .. }) => {
                            state_machine_id.encode() == counterparty_state_id.encode()
                        }
                        _ => false,
                    })
                    .map(|res| match res {
                        Ok(StateMachineUpdatedEvent {
                            state_machine_id: _, latest_height, ..
                        }) => Ok(StateMachineUpdated {
                            state_machine_id: counterparty_state_id,
                            latest_height,
                        }),
                        Err(e) => Err(e.into()),
                    });
                event
            }
        });

        Ok(Box::pin(stream))
    }
}
