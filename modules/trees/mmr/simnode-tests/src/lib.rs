#![cfg(test)]

use codec::{Decode, Encode};
use mmr_primitives::{DataOrHash, FullLeaf};
use pallet_ismp::mmr::Leaf;
use runtime_types::{
    gargantua,
    gargantua::api::runtime_types::{ismp::host::Ethereum, pallet_ismp_demo::pallet::EvmParams},
};
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{crypto::Ss58Codec, keccak_256, offchain::StorageKind, Bytes, H256};
use sp_keyring::sr25519::Keyring;
use sp_mmr_primitives::{mmr_lib::leaf_index_to_pos, utils::NodesUtils, INDEXING_PREFIX};
use sp_runtime::traits::Keccak256;
use std::{env, time::Duration};
use subxt::{
    config::{polkadot::PolkadotExtrinsicParams, substrate::SubstrateHeader, Hasher, Header},
    rpc_params,
    tx::SubmittableExtrinsic,
    utils::{AccountId32, MultiAddress, MultiSignature, H160},
    OnlineClient,
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
    type AccountId = AccountId32;
    type Address = MultiAddress<Self::AccountId, u32>;
    type Signature = MultiSignature;
    type Hasher = RuntimeHasher;
    type Header = SubstrateHeader<u32, RuntimeHasher>;
    type ExtrinsicParams = PolkadotExtrinsicParams<Self>;
}

async fn dispatch_requests() -> Result<(), anyhow::Error> {
    let port = env::var("PORT").unwrap_or("9990".into());
    let client = OnlineClient::<Hyperbridge>::from_url(format!("ws://127.0.0.1:{}", port)).await?;

    // Initialize leaf count by dispatching some leaves
    let params = EvmParams {
        module: H160::random(),
        destination: Ethereum::ExecutionLayer,
        timeout: 0,
        count: 10,
    };
    let call = client
        .tx()
        .call_data(&gargantua::api::tx().ismp_demo().dispatch_to_evm(params))?;
    let extrinsic: Bytes = client
        .rpc()
        .request(
            "simnode_authorExtrinsic",
            // author an extrinsic from alice
            rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
        )
        .await?;
    let submittable = SubmittableExtrinsic::from_bytes(client.clone(), extrinsic.0);
    submittable.submit().await?;
    tokio::time::sleep(Duration::from_secs(10)).await;

    let created_block = client
        .rpc()
        .request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
        .await?;

    let mut last_finalized = created_block.hash;
    let _ = client
        .rpc()
        .request::<bool>("engine_finalizeBlock", rpc_params![last_finalized])
        .await?;
    for _ in 0..3 {
        let created_block = client
            .rpc()
            .request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
            .await?;
        last_finalized = created_block.hash;
    }

    let _ = client
        .rpc()
        .request::<bool>("engine_finalizeBlock", rpc_params![last_finalized])
        .await?;

    // Dispatch some requests

    let mut chain_a = vec![];
    let mut chain_b = vec![];

    let mut chain_a_commitments = vec![];
    let mut chain_b_commitments = vec![];

    // Fork A
    {
        let mut parent_hash = last_finalized;
        for _ in 0..3 {
            let params = EvmParams {
                module: H160::random(),
                destination: Ethereum::ExecutionLayer,
                timeout: 0,
                count: 10,
            };
            let call = client
                .tx()
                .call_data(&gargantua::api::tx().ismp_demo().dispatch_to_evm(params))?;
            let extrinsic: Bytes = client
                .rpc()
                .request(
                    "simnode_authorExtrinsic",
                    // author an extrinsic from alice
                    rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
                )
                .await?;
            let submittable = SubmittableExtrinsic::from_bytes(client.clone(), extrinsic.0);
            submittable.submit().await?;
            tokio::time::sleep(Duration::from_secs(10)).await;

            let created_block = client
                .rpc()
                .request::<CreatedBlock<H256>>(
                    "engine_createBlock",
                    rpc_params![true, false, Some(parent_hash)],
                )
                .await?;

            let events = client.events().at(created_block.hash).await?;

            let events = events
                .iter()
                .filter_map(|ev| {
                    ev.ok().and_then(|ev| {
                        ev.as_event::<gargantua::api::ismp::events::Request>()
                            .ok()
                            .flatten()
                            .and_then(|ev| Some((parent_hash, ev.commitment)))
                    })
                })
                .collect::<Vec<_>>();

            chain_a_commitments.extend(events);

            parent_hash = created_block.hash;
            chain_a.push(parent_hash);
        }
    }

    println!("Finished creating Fork A");

    println!("Creating Fork B");

    // Fork B
    {
        let mut parent_hash = last_finalized;
        for _ in 0..3 {
            let params = EvmParams {
                module: H160::random(),
                destination: Ethereum::ExecutionLayer,
                timeout: 0,
                count: 10,
            };
            let call = client
                .tx()
                .call_data(&gargantua::api::tx().ismp_demo().dispatch_to_evm(params))?;
            let extrinsic: Bytes = client
                .rpc()
                .request(
                    "simnode_authorExtrinsic",
                    // author an extrinsic from alice
                    rpc_params![Bytes::from(call), Keyring::Alice.to_account_id().to_ss58check()],
                )
                .await?;
            let submittable = SubmittableExtrinsic::from_bytes(client.clone(), extrinsic.0);
            submittable.submit().await?;
            tokio::time::sleep(Duration::from_secs(10)).await;
            let created_block = client
                .rpc()
                .request::<CreatedBlock<H256>>(
                    "engine_createBlock",
                    rpc_params![true, false, parent_hash],
                )
                .await?;

            let events = client.events().at(created_block.hash).await?;

            let events = events
                .iter()
                .filter_map(|ev| {
                    ev.ok().and_then(|ev| {
                        ev.as_event::<gargantua::api::ismp::events::Request>()
                            .ok()
                            .flatten()
                            .and_then(|ev| Some((parent_hash, ev.commitment)))
                    })
                })
                .collect::<Vec<_>>();

            chain_b_commitments.extend(events);

            parent_hash = created_block.hash;
            chain_b.push(parent_hash);
        }
    }

    assert_eq!(chain_a_commitments.len(), 30);
    assert_eq!(chain_b_commitments.len(), 30);

    println!("Finished creating Fork B");

    // Finalize fork b
    let res = client
        .rpc()
        .request::<bool>("engine_finalizeBlock", rpc_params![chain_a.last().cloned().unwrap()])
        .await?;
    assert!(res);
    // Import some more blocks
    for _ in 0..10 {
        let _ = client
            .rpc()
            .request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
            .await?;
    }

    // Wait for some time for the async worker to complete
    tokio::time::sleep(Duration::from_secs(60)).await;

    // All Non canonical keys should no longer exist in storage
    let indexing_prefix = INDEXING_PREFIX.to_vec();

    for (idx, (parent_hash, _)) in chain_a_commitments.into_iter().enumerate() {
        let pos = leaf_index_to_pos(idx as u64);
        let non_canon_key = NodesUtils::node_temp_offchain_key::<
            sp_runtime::generic::Header<u32, Keccak256>,
        >(&indexing_prefix, pos, parent_hash);
        let value = client
            .rpc()
            .request::<Option<Bytes>>(
                "offchain_localStorageGet",
                rpc_params![StorageKind::PERSISTENT, Bytes::from(non_canon_key)],
            )
            .await?;
        assert!(value.is_none());
    }

    // Canonical keys should exist and the commitment should match the commitments we have for chain
    // B
    for (idx, (parent_hash, commitment)) in chain_b_commitments.into_iter().enumerate() {
        let pos = leaf_index_to_pos(10 + idx as u64);
        let non_canon_key = NodesUtils::node_temp_offchain_key::<
            sp_runtime::generic::Header<u32, Keccak256>,
        >(&indexing_prefix, pos, parent_hash);
        let canon_key = NodesUtils::node_canon_offchain_key(&indexing_prefix, pos);
        let value = client
            .rpc()
            .request::<Option<Bytes>>(
                "offchain_localStorageGet",
                rpc_params![StorageKind::PERSISTENT, Bytes::from(non_canon_key)],
            )
            .await?;
        assert!(value.is_none());

        let value = client
            .rpc()
            .request::<Option<Bytes>>(
                "offchain_localStorageGet",
                rpc_params![StorageKind::PERSISTENT, Bytes::from(canon_key)],
            )
            .await?;

        let data = value.unwrap().0;
        let leaf = match DataOrHash::<Keccak256, Leaf>::decode(&mut &*data).unwrap() {
            DataOrHash::Data(leaf) => leaf,
            _ => unreachable!(),
        };
        let request = keccak_256(&leaf.preimage());
        assert_eq!(commitment.0, request);
    }

    Ok(())
}
