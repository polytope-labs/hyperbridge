use crate::{host::InMemorySigner, parachain, try_sending_with_tip, ParachainClient};
use codec::Encode;
use futures::stream::StreamExt;
use sp_core::Pair;
use std::time::Duration;
use subxt::{
    config::{
        extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
    },
    ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
    utils::AccountId32,
};

impl<T> ParachainClient<T>
where
    T: subxt::Config + Send + Sync + Clone,
    T::Header: Send + Sync,
    <T::ExtrinsicParams as ExtrinsicParams<T::Index, T::Hash>>::OtherParams:
        Default + Send + From<BaseExtrinsicParamsBuilder<T, PlainTip>>,
    T::AccountId: From<sp_core::crypto::AccountId32>
        + Into<T::Address>
        + Encode
        + Clone
        + 'static
        + Send
        + Sync,
    T::Signature: From<MultiSignature> + Send + Sync,
{
    pub async fn balance(&self) -> Result<u128, anyhow::Error> {
        let addr =
            parachain::api::storage().system().account(<sp_core::sr25519::Public as Into<
                AccountId32,
            >>::into(self.signer.public()));
        let account = self
            .parachain
            .storage()
            .at_latest()
            .await?
            .fetch(&addr)
            .await?
            .expect("Account should exist");
        Ok(account.data.free)
    }

    pub async fn timestamp(&self) -> Result<Duration, anyhow::Error> {
        let addr = parachain::api::storage().timestamp().now();
        let timestamp = self
            .parachain
            .storage()
            .at_latest()
            .await?
            .fetch(&addr)
            .await?
            .expect("Timestamp should exist");
        Ok(Duration::from_millis(timestamp))
    }

    pub async fn transfer(
        &self,
        params: ismp_assets::TransferParams<T::AccountId, u128>,
    ) -> Result<(), anyhow::Error> {
        let signer = InMemorySigner {
            account_id: MultiSigner::Sr25519(self.signer.public()).into_account().into(),
            signer: self.signer.clone(),
        };

        let tx = parachain::api::tx()
            .ismp_assets()
            .transfer(codec::Decode::decode(&mut &*params.encode())?);
        let progress = try_sending_with_tip(&self.parachain, signer, tx).await?;
        let tx = progress.wait_for_in_block().await?;

        tx.wait_for_success().await?;

        Ok(())
    }

    pub async fn ismp_assets_events_stream(
        &self,
        count: usize,
    ) -> Result<Vec<parachain::api::ismp_assets::events::BalanceReceived>, anyhow::Error> {
        let subscription = self.parachain.rpc().subscribe_best_block_headers().await?;
        let client = self.parachain.clone();
        let stream = subscription.filter_map(move |header| {
            let client = client.clone();
            async move {
                let events = client.events().at(header.ok()?.hash()).await.ok()?;

                events
                    .find::<parachain::api::ismp_assets::events::BalanceReceived>()
                    .collect::<Result<Vec<_>, _>>()
                    .ok()
            }
        });

        let mut stream = Box::pin(stream);

        let mut total = 0;
        let mut values = vec![];
        while let Some(mut val) = stream.next().await {
            values.append(&mut val);
            total += values.len();
            if total >= count {
                return Ok(values)
            }
        }
        Err(anyhow::Error::msg("Stream ended"))
    }
}
