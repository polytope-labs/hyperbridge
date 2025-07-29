use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Error};
use codec::{Decode, Encode};
use futures::FutureExt;
use subxt::{
	config::{substrate::SubstrateHeader, ExtrinsicParams, HashFor, Header},
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature, H256},
};

use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::{Event, StateMachineUpdated},
	host::StateMachine,
};
use substrate_state_machine::fetch_overlay_root_and_timestamp;
use tesseract_primitives::{BoxStream, ByzantineHandler, IsmpProvider};

use crate::SubstrateClient;

#[async_trait::async_trait]
impl<C> ByzantineHandler for SubstrateClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	C::AccountId: From<AccountId32> + Into<C::Address> + Clone + Send + Sync,
	C::Signature: From<MultiSignature> + Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C>>::Params: Send + Sync + DefaultParams,
	H256: From<HashFor<C>>,
{
	async fn check_for_byzantine_attack(
		&self,
		coprocessor: StateMachine,
		counterparty: Arc<dyn IsmpProvider>,
		event: StateMachineUpdated,
	) -> Result<(), anyhow::Error> {
		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: self.state_machine_id().state_id,
				consensus_state_id: self.state_machine_id().consensus_state_id,
			},
			height: event.latest_height,
		};

		let Some(block_hash) =
			self.rpc.chain_get_block_hash(Some(event.latest_height.into())).await?
		else {
			// If block header is not found veto the state commitment

			log::info!(
				"Vetoing state commitment for {} on {}: block header not found for {}",
				self.state_machine_id().state_id,
				counterparty.state_machine_id().state_id,
				event.latest_height
			);
			counterparty.veto_state_commitment(height).await?;

			return Ok(());
		};
		let header = self
			.rpc
			.chain_get_header(Some(block_hash))
			.await?
			.ok_or_else(|| anyhow!("Failed to get block header in byzantine handler"))?;

		let header = SubstrateHeader::<u32, C::Hasher>::decode(&mut &*header.encode())?;

		let digest =
			polkadot_sdk::sp_runtime::generic::Digest::decode(&mut &*header.digest.encode())?;
		let digest_result = fetch_overlay_root_and_timestamp(&digest, Default::default())
			.map_err(|_| anyhow!("Failed to extract disgest logs in byzantine handler"))?;

		let state_root = if self.state_machine_id().state_id == coprocessor {
			digest_result.ismp_digest.child_trie_root
		} else {
			header.state_root.into()
		};
		let finalized_state_commitment =
			counterparty.query_state_machine_commitment(height).await?;

		if finalized_state_commitment.state_root != state_root.into() {
			log::info!(
				"Vetoing state commitment for {} on {}, state commitment mismatch",
				self.state_machine_id().state_id,
				counterparty.state_machine_id().state_id
			);
			counterparty.veto_state_commitment(height).await?;
		}

		Ok(())
	}

	async fn state_machine_updates(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<Vec<StateMachineUpdated>>, Error> {
		use futures::StreamExt;
		let client = self.clone();
		let (tx, recv) = tokio::sync::broadcast::channel(512);
		let latest_height = client.query_finalized_height().await?;

		tokio::task::spawn(async move {
				let mut latest_height = latest_height;
				let state_machine = client.state_machine;
				loop {
					tokio::time::sleep(Duration::from_secs(3)).await;
					let header = match client.rpc.chain_get_header(None).await {
						Ok(Some(header)) => header,
						_ => {
							if let Err(err) = tx
								.send(Err(anyhow!(
									"Error encountered while fetching finalized head"
								).into()))
							{
								log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
								return
							}
							continue;
						},
					};

					if header.number().into() <= latest_height {
						continue;
					}

					let event = StateMachineUpdated {
						state_machine_id: client.state_machine_id(),
						latest_height: header.number().into(),
					};

					let events = match client.query_ismp_events(latest_height, event).await {
						Ok(e) => e,
						Err(err) => {
							if let Err(err) = tx
								.send(Err(anyhow!(
									"Error encountered while querying ismp events {err:?}"
								).into()))
							{
								log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
								return
							}
							latest_height = header.number().into();
							continue;
						},
					};

					let events = events
						.into_iter()
						.filter_map(|event| match event {
							Event::StateMachineUpdated(e)
								if e.state_machine_id == counterparty_state_id =>
								Some(e),
							_ => None,
						})
						.collect::<Vec<_>>();

					if !events.is_empty() {
						if let Err(err) = tx
										.send(Ok(events))
									{
										log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
										return
									}
					}

					latest_height = header.number().into();
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
