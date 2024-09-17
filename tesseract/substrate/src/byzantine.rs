use std::sync::Arc;

use anyhow::anyhow;
use codec::{Decode, Encode};
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::StateMachineUpdated,
};
use sp_core::H256;
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip,
		substrate::SubstrateHeader, ExtrinsicParams,
	},
	ext::sp_runtime::{AccountId32, MultiSignature},
};
use tesseract_primitives::{ByzantineHandler, IsmpProvider};

use crate::SubstrateClient;

#[async_trait::async_trait]
impl<C> ByzantineHandler for SubstrateClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::AccountId: From<AccountId32> + Into<C::Address> + Clone + Send + Sync,
	C::Signature: From<MultiSignature> + Send + Sync,
	H256: From<<C as subxt::Config>::Hash>,
{
	async fn check_for_byzantine_attack(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
		event: StateMachineUpdated,
	) -> Result<(), anyhow::Error> {
		let is_syncing = self.client.rpc().system_health().await?.is_syncing;
		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: self.state_machine_id().state_id,
				consensus_state_id: self.state_machine_id().consensus_state_id,
			},
			height: event.latest_height,
		};

		let Some(block_hash) =
			self.client.rpc().block_hash(Some(event.latest_height.into())).await?
		else {
			// If block header is not found and node is fully synced, veto the state commitment
			if !is_syncing {
				log::info!(
					"Vetoing state commitment for {} on {}",
					self.state_machine_id().state_id,
					counterparty.state_machine_id().state_id
				);
				counterparty.veto_state_commitment(height).await?;
				return Ok(())
			} else {
				Err(anyhow!("Node is still syncing, cannot fetch finalized block"))?
			}
		};
		let header = self
			.client
			.rpc()
			.header(Some(block_hash))
			.await?
			.ok_or_else(|| anyhow!("Failed to get block header in byzantine handler"))?;

		let header = SubstrateHeader::<u32, C::Hasher>::decode(&mut &*header.encode())?;

		let finalized_state_commitment =
			counterparty.query_state_machine_commitment(height).await?;

		if finalized_state_commitment.state_root != header.state_root.into() {
			log::info!(
				"Vetoing state commitment for {} on {}",
				self.state_machine_id().state_id,
				counterparty.state_machine_id().state_id
			);
			counterparty.veto_state_commitment(height).await?;
		}

		Ok(())
	}
}
