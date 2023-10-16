//! Functions for updating configuration on pallets

use crate::{
	extrinsic::{send_extrinsic, Extrinsic, InMemorySigner},
	SubstrateClient,
};
use codec::Encode;
use ismp::messaging::CreateConsensusState;
use primitives::IsmpHost;
use sp_core::Pair;
use subxt::{
	config::{extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams},
	ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
	tx::TxPayload,
};

impl<T, C> SubstrateClient<T, C>
where
	T: IsmpHost + Send + Sync + Clone,
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::AccountId: From<sp_core::crypto::AccountId32>
		+ Into<C::Address>
		+ Encode
		+ Clone
		+ 'static
		+ Send
		+ Sync,
	C::Signature: From<MultiSignature> + Send + Sync,
{
	pub async fn create_consensus_state(
		&self,
		message: CreateConsensusState,
	) -> Result<(), anyhow::Error> {
		let signer = InMemorySigner {
			account_id: MultiSigner::Sr25519(self.signer.public()).into_account().into(),
			signer: self.signer.clone(),
		};

		let call = message.encode();
		let call = Extrinsic::new("Ismp", "create_consensus_client", call)
			.encode_call_data(&self.client.metadata())?;
		let tx = Extrinsic::new("Sudo", "sudo", call);
		let nonce = self.get_nonce().await?;
		send_extrinsic(&self.client, signer, tx, nonce).await?;

		Ok(())
	}
}
