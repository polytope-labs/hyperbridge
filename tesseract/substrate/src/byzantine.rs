use std::sync::Arc;

use anyhow::anyhow;
use codec::{Decode, Encode};
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::StateMachineUpdated,
	host::StateMachine,
};
use sp_core::H256;
use substrate_state_machine::fetch_overlay_root_and_timestamp;
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
			self.client.rpc().block_hash(Some(event.latest_height.into())).await?
		else {
			// If block header is not found veto the state commitment
			log::info!(
				"Vetoing state commitment for {} on {}: block header not found for {}",
				self.state_machine_id().state_id,
				counterparty.state_machine_id().state_id,
				event.latest_height
			);
			counterparty.veto_state_commitment(height).await?;
			return Ok(())
		};
		let header = self
			.client
			.rpc()
			.header(Some(block_hash))
			.await?
			.ok_or_else(|| anyhow!("Failed to get block header in byzantine handler"))?;

		let header = SubstrateHeader::<u32, C::Hasher>::decode(&mut &*header.encode())?;

		let digest = sp_runtime::generic::Digest::decode(&mut &*header.digest.encode())?;
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
}
