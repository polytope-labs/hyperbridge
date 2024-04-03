use beefy_prover::Prover;
use beefy_verifier_primitives::ConsensusState;
use codec::{Decode, Encode};
use ethers::abi::{AbiDecode, AbiEncode, Token, Uint};
use forge_testsuite::Runner;
use futures::stream::StreamExt;
use hex_literal::hex;
use ismp_solidity_abi::{
    beefy::{BeefyConsensusProof, BeefyConsensusState, IntermediateState},
    local,
};
use primitive_types::H256;
use serde::Deserialize;
use sp_consensus_beefy::{
    ecdsa_crypto::Signature, mmr::MmrLeaf, Commitment, VersionedFinalityProof,
};
use sp_runtime::{generic::Header, traits::BlakeTwo256};
use std::{env, path::PathBuf, str::FromStr};
use subxt::{
    config::{
        polkadot::PolkadotExtrinsicParams,
        substrate::{SubstrateExtrinsicParams, SubstrateHeader},
        Hasher, WithExtrinsicParams,
    },
    rpc::Subscription,
    rpc_params,
    utils::{AccountId32, MultiAddress, MultiSignature},
    PolkadotConfig,
};

type Hyperbridge =
    WithExtrinsicParams<HyperbridgeConfig, PolkadotExtrinsicParams<HyperbridgeConfig>>;

pub struct HyperbridgeConfig {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode)]
pub struct Keccak256;

impl Hasher for Keccak256 {
    type Output = H256;
    fn hash(s: &[u8]) -> Self::Output {
        sp_core::keccak_256(s).into()
    }
}

impl subxt::Config for HyperbridgeConfig {
    type Hash = H256;
    type AccountId = AccountId32;
    type Address = MultiAddress<Self::AccountId, u32>;
    type Signature = MultiSignature;
    type Hasher = Keccak256;
    type Header = SubstrateHeader<u32, Keccak256>;
    type ExtrinsicParams = SubstrateExtrinsicParams<Self>;
}

fn default_para_id() -> u32 {
    2000
}
fn activation_block() -> u32 {
    0
}
fn default_relay_ws_url() -> String {
    "ws://127.0.0.1:9944".to_string()
}
fn default_para_ws_url() -> String {
    "ws://127.0.0.1:9988".to_string()
}
#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default = "default_relay_ws_url")]
    relay_ws_url: String,
    #[serde(default = "default_para_ws_url")]
    para_ws_url: String,
    #[serde(default = "default_para_id")]
    para_id: u32,
    #[serde(default = "activation_block")]
    activation_block: u32,
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn beefy_consensus_client_test() -> Result<(), anyhow::Error> {
    // first compile the project.
    let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
    let mut runner = Runner::new(PathBuf::from(&base_dir));
    let mut contract = runner.deploy("BeefyConsensusClientTest").await;
    let config = envy::from_env::<Config>()?;

    let Config { relay_ws_url, para_ws_url, para_id, activation_block } = config;
    let relay = subxt::client::OnlineClient::<PolkadotConfig>::from_url(relay_ws_url).await?;
    let para = subxt::client::OnlineClient::<Hyperbridge>::from_url(para_ws_url).await?;

    para.blocks()
        .subscribe_best()
        .await
        .unwrap()
        .skip_while(|result| {
            futures::future::ready({
                match result {
                    Ok(block) => block.number() < 5,
                    Err(_) => false,
                }
            })
        })
        .take(1)
        .collect::<Vec<_>>()
        .await;

    println!("Parachains Onboarded");

    let prover =
        Prover { beefy_activation_block: activation_block, relay, para, para_ids: vec![para_id] };
    let initial_state = prover.get_initial_consensus_state().await?;
    let consensus_state: BeefyConsensusState = initial_state.into();
    let subscription: Subscription<String> = prover
        .relay
        .rpc()
        .subscribe(
            "beefy_subscribeJustifications",
            rpc_params![],
            "beefy_unsubscribeJustifications",
        )
        .await?;

    let mut subscription_stream = subscription.take(10).enumerate();
    while let Some((_count, Ok(commitment))) = subscription_stream.next().await {
        let commitment: sp_core::Bytes = FromStr::from_str(&commitment)?;
        let VersionedFinalityProof::V1(signed_commitment) =
            VersionedFinalityProof::<u32, Signature>::decode(&mut &*commitment)?;

        match signed_commitment.commitment.validator_set_id {
            id if id < consensus_state.current_authority_set.id.as_u64() => {
                // If validator set id of signed commitment is less than current validator set id we
                // have Then commitment is outdated and we skip it.
                println!(
                    "Skipping outdated commitment \n Received signed commitmment with validator_set_id: {:?}\n Current authority set id: {:#?}\n Next authority set id: {:?}\n",
                    signed_commitment.commitment.validator_set_id, consensus_state.current_authority_set.id, consensus_state.current_authority_set.id
                );
                continue;
            },
            _ => {},
        };

        let consensus_proof: BeefyConsensusProof =
            prover.consensus_proof(signed_commitment.clone()).await?.into();

        if consensus_proof.relay.signed_commitment.commitment.block_number ==
            consensus_state.latest_height
        {
            continue;
        }

        let (new_state, intermediates) = contract
            .call::<_, (bytes::Bytes, IntermediateState)>(
                "VerifyV1",
                (
                    Token::Bytes(consensus_state.clone().encode()),
                    Token::Bytes(consensus_proof.encode()),
                ),
            )
            .await?;

        let debug_intermediate: local::IntermediateState = intermediates.into();
        dbg!(&debug_intermediate);

        {
            let debug_consensus_state: ConsensusState =
                BeefyConsensusState::decode(new_state)?.into();
            dbg!(&debug_consensus_state);
        }
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_decode_encode() -> Result<(), anyhow::Error> {
    // first compile the project.
    let base_dir = env::current_dir()?.parent().unwrap().display().to_string();
    let mut runner = Runner::new(PathBuf::from(&base_dir));
    let mut contract = runner.deploy("BeefyConsensusClientTest").await;

    let mmr_leaf = hex!("003f1e0000ccaf442e2648d278e87dbca890e532ef9cb7cf2058d023903b49567e2943996f550000000000000006000000a9d36172252f275bc8b7851062dff4a29e018355d8626c941f2ad57dfbabecd008ca13222c83d2a481d7b63c356d95bf9366b2a70e907ca3e38fa52e35731537").to_vec();
    let header = hex!("9a28ac82dd089df2f5215ec55ae8b4933f9d58c8c76bf0c0ca1884f3778af2b7a53ba87a649f925c5093914299f42c78ad997b5f69a2ca5dc9ad3357cd0aeb6fd409566fe009ee37e1bbdc43af58c0be65d195bc3f0a5c98568bb12b709ef0d4f3be0806617572612038b856080000000005617572610101ecb27e1850a572d08ff0f4e94a1a557b0ddd7b12158627e442789802aada1553e65d72ecc3a6c0efb9794fb6c2ebf5878da36d6e5b8295cc0f42810beb64c68a").to_vec();
    let commitment = hex!("046d688088bc15df49c90d1823ac81aa90236815062561ccc4352983576013413e17c25a401e00005400000000000000").to_vec();

    let mmr_leaf = MmrLeaf::<u32, H256, H256, H256>::decode(&mut &*mmr_leaf).unwrap();
    let header = Header::<u32, BlakeTwo256>::decode(&mut &*header).unwrap();
    let commitment = Commitment::<u32>::decode(&mut &*commitment).unwrap();

    {
        type H256Hash = [u8; 32];
        let (parent_hash, number, state_root, extrinsics_root, digests) = contract
            .call::<_, (H256Hash, u32, H256Hash, H256Hash, Vec<Token>)>(
                "DecodeHeader",
                (Token::Bytes(header.encode())),
            )
            .await
            .unwrap();

        assert_eq!(&parent_hash, header.parent_hash.as_fixed_bytes());
        assert_eq!(number, header.number);
        assert_eq!(&state_root, header.state_root.as_fixed_bytes());
        assert_eq!(&extrinsics_root, header.extrinsics_root.as_fixed_bytes());
        assert_eq!(header.digest.logs.len(), digests.len());
    }

    {
        let abi = Token::Tuple(vec![
            Token::Array(vec![Token::Tuple(vec![
                Token::FixedBytes(b"mh".to_vec()),
                Token::Bytes(commitment.payload.get_raw(b"mh").unwrap().clone()),
            ])]),
            Token::Uint(Uint::from(commitment.block_number)),
            Token::Uint(Uint::from(commitment.validator_set_id)),
        ]);
        let encoded = contract.call::<_, Vec<u8>>("EncodeCommitment", (abi,)).await.unwrap();

        assert_eq!(encoded, commitment.encode());
    }

    {
        let abi = Token::Tuple(vec![
            Token::Uint(Uint::from(0)),
            Token::Uint(Uint::from(mmr_leaf.parent_number_and_hash.0)),
            Token::FixedBytes(mmr_leaf.parent_number_and_hash.1.as_bytes().to_vec()),
            Token::Tuple(vec![
                Token::Uint(Uint::from(mmr_leaf.beefy_next_authority_set.id)),
                Token::Uint(Uint::from(mmr_leaf.beefy_next_authority_set.len)),
                Token::FixedBytes(
                    mmr_leaf.beefy_next_authority_set.keyset_commitment.as_bytes().to_vec(),
                ),
            ]),
            Token::FixedBytes(mmr_leaf.leaf_extra.as_bytes().to_vec()),
            Token::Uint(Uint::from(0)),
            Token::Uint(Uint::from(0)),
        ]);

        let encoded = contract.call::<_, Vec<u8>>("EncodeLeaf", (abi,)).await.unwrap();

        assert_eq!(encoded, mmr_leaf.encode());
    }

    Ok(())
}
