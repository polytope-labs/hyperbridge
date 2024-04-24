#![cfg(test)]

use codec::Encode;
use gargantua_runtime::MultiAddress;
use runtime_types::{
    gargantua,
    gargantua::api::runtime_types::{ismp::host::Ethereum, pallet_ismp_demo::pallet::EvmParams},
};
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{
    crypto::{AccountId32, Ss58Codec},
    Bytes, H256,
};
use sp_keyring::sr25519::Keyring;
use std::env;
use subxt::{
    config::{polkadot::PolkadotExtrinsicParams, substrate::SubstrateHeader, Hasher},
    dynamic::Value,
    ext::{sp_core::keccak_256, sp_runtime::MultiSignature},
    rpc_params,
    tx::SubmittableExtrinsic,
    OnlineClient, SubstrateConfig,
};

#[tokio::test]
async fn test_all_features() -> Result<(), anyhow::Error> {
    dispatch_requests().await?;
    Ok(())
}

#[derive(Clone)]
pub struct Hyperbridge;

/// A type that can hash values using the keccak_256 algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode)]
pub struct RuntimeHasher;

impl Hasher for RuntimeHasher {
    type Output = H256;
    fn hash(s: &[u8]) -> Self::Output {
        keccak_256(s).into()
    }
}

impl subxt::Config for Hyperbridge {
    type Hash = H256;
    type AccountId = subxt::ext::sp_runtime::AccountId32;
    type Address = MultiAddress<Self::AccountId, u32>;
    type Signature = MultiSignature;
    type Hasher = RuntimeHasher;
    type Header = SubstrateHeader<u32, RuntimeHasher>;
    type ExtrinsicParams = PolkadotExtrinsicParams<Self>;
}

async fn dispatch_requests() -> Result<(), anyhow::Error> {
    let port = env::var("PORT").unwrap_or("9944".into());
    let client = OnlineClient::<Hyperbridge>::from_url(format!("ws://127.0.0.1:{}", port)).await?;

    // Dispatch some requests

    let params = EvmParams {
        module: Default::default(),
        destination: Ethereum::ExecutionLayer,
        timeout: 0,
        count: 10,
    };

    // Fork A
    {
        let mut parent_hash = H256::random();
        for _ in 0..3 {
            let call = client
                .tx()
                .call_data(&gargantua::api::tx().ismp_demo().dispatch_to_evm(params.clone()))?;
            let extrinsic: Bytes = client
                .rpc()
                .request(
                    "simnode_authorExtrinsic",
                    // author an extrinsic from alice
                    rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
                )
                .await?;
            let submittable = SubmittableExtrinsic::from_bytes(client.clone(), extrinsic.0);
            let in_block = submittable.submit_and_watch().await?.wait_for_in_block().await?;

            let events = in_block.fetch_events().await?.all_events_in_block().clone();

            let created_block = client
                .rpc()
                .request::<CreatedBlock<H256>>(
                    "engine_createBlock",
                    rpc_params![false, false, Some(parent_hash)],
                )
                .await?;

            parent_hash = created_block.hash;
        }
    }

    Ok(())
}

fn dispatch_demo_requests() {}
