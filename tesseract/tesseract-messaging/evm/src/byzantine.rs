use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Error};
use ethers::providers::Middleware;
use futures::FutureExt;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::{Event, StateMachineUpdated},
	host::StateMachine,
};
use tesseract_primitives::{BoxStream, ByzantineHandler, IsmpProvider};

use crate::EvmClient;

#[async_trait::async_trait]
impl ByzantineHandler for EvmClient {
	async fn check_for_byzantine_attack(
		&self,
		_coprocessor: StateMachine,
		counterparty: Arc<dyn IsmpProvider>,
		event: StateMachineUpdated,
	) -> Result<(), anyhow::Error> {
		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: self.state_machine,
				consensus_state_id: self.consensus_state_id,
			},
			height: event.latest_height,
		};
		let Some(header) = self.client.get_block(event.latest_height).await? else {
			// If block header is not found veto the state commitment
			log::info!(
				"Vetoing State Machine Update for {} on {}",
				self.state_machine,
				counterparty.state_machine_id().state_id
			);
			counterparty.veto_state_commitment(height).await?;
			return Ok(());
		};

		let state_machine_commitment = counterparty.query_state_machine_commitment(height).await?;
		if header.state_root.0 != state_machine_commitment.state_root.0 {
			log::info!(
				"Vetoing State Machine Update for {} on {}",
				self.state_machine,
				counterparty.state_machine_id().state_id
			);
			counterparty.veto_state_commitment(height).await?;
		}

		Ok(())
	}

	async fn state_machine_updates(
		&self,
		_counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<Vec<StateMachineUpdated>>, Error> {
		use futures::StreamExt;
		let (tx, recv) = tokio::sync::broadcast::channel(512);

		let initial_height = self.client.get_block_number().await?.low_u64();
		let client = self.clone();
		let poll_interval = 5;
		tokio::spawn(async move {
				let mut latest_height = initial_height;
				let state_machine = client.state_machine;
				loop {
					tokio::time::sleep(Duration::from_secs(poll_interval)).await;
					// wait for an update with a greater height
					let block_number = match client.client.get_block_number().await {
						Ok(number) => number.low_u64(),
						Err(err) => {
							if let Err(err) = tx
								.send(Err(anyhow!(
									"Error fetching latest block height on {state_machine:?} {err:?}"
								).into()))
							{
								log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
								return
							}
							continue;
						},
					};

					if block_number <= latest_height {
						continue;
					}

					let event = StateMachineUpdated {
						state_machine_id: client.state_machine_id(),
						latest_height: block_number,
					};

					let events = match client.query_ismp_events(latest_height, event).await {
						Ok(events) => events,
						Err(err) => {
							if let Err(err) = tx
								.send(Err(anyhow!(
									"Error encountered while querying ismp events {err:?}"
								).into()))
							{
								log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
								return
							}
							latest_height = block_number;
							continue;
						},
					};

					let events = events
						.into_iter()
						.filter_map(|ev| match ev {
							Event::StateMachineUpdated(update) => Some(update),
							_ => None,
						}).collect::<Vec<_>>();

					if !events.is_empty() {
						if let Err(err) = tx
									.send(Ok(events))
								{
									log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
									return
								}
					}
					latest_height = block_number;
				}
			}.boxed());

		let stream = tokio_stream::wrappers::BroadcastStream::new(recv).filter_map(|res| async {
			match res {
				Ok(res) => Some(res),
				Err(err) => Some(Err(anyhow!("{err:?}").into())),
			}
		});

		Ok(Box::pin(stream))
	}
}
