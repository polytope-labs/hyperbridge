//! EcdsaBeefy consensus tests.
//!
//! `test_decode_encode` exercises the Beefy Codec with fixed test vectors — runs under
//! plain `cargo test`.
//!
//! `test_beefy_consensus_client` is a live integration test against a running Polkadot
//! relay chain + Hyperbridge parachain. It is marked `#[ignore]` so it is skipped by
//! default; run with `cargo test -- --ignored test_beefy_consensus_client`. Configure
//! endpoints with env vars: `RELAY_WS_URL`, `PARA_WS_URL`, `PARA_ID`.

use super::utils::*;
use alloy_primitives::{Bytes, FixedBytes, U256 as AlloyU256};
use alloy_sol_types::{SolCall, SolValue};
use anyhow::anyhow;
use beefy_verifier_primitives::ConsensusState;
use codec::{Decode, Encode};
use futures::stream::StreamExt;
use hex_literal::hex;
use ismp_abi::ecdsa_beefy::{BeefyConsensusProof, BeefyConsensusState};
use pallet_ismp::{ConsensusDigest, ISMP_ID};
use polkadot_sdk::*;
use primitive_types::H256;
use serde::Deserialize;
use sp_consensus_beefy::{
	ecdsa_crypto::Signature, mmr::MmrLeaf, Commitment as BeefyCommitment, VersionedFinalityProof,
};
use sp_runtime::{generic::Header, traits::BlakeTwo256};
use subxt::{
	backend::legacy::LegacyRpcMethods,
	config::{Hasher, Header as SubxtHeader},
	ext::subxt_rpcs::{client::RpcSubscription, rpc_params},
	PolkadotConfig,
};
use subxt_utils::Hyperbridge;

alloy_sol_macro::sol! {
	struct DigestLog {
		uint8 kind;
		bytes4 id;
		bytes data;
	}

	struct HeaderOut {
		bytes32 parentHash;
		uint256 number;
		bytes32 stateRoot;
		bytes32 extrinsicRoot;
		DigestLog[] digests;
	}

	struct PayloadItem {
		bytes2 id;
		bytes data;
	}

	struct SolCommitment {
		PayloadItem[] payload;
		uint256 blockNumber;
		uint256 validatorSetId;
	}

	struct AuthoritySetCommitment {
		uint256 id;
		uint256 len;
		bytes32 root;
	}

	struct PartialBeefyMmrLeaf {
		uint256 version;
		uint256 parentNumber;
		bytes32 parentHash;
		AuthoritySetCommitment nextAuthoritySet;
		bytes32 extra;
	}

	struct IntermediateState {
		uint256 stateMachineId;
		uint256 height;
		StateCommitment commitment;
	}

	struct StateCommitment {
		uint256 timestamp;
		bytes32 overlayRoot;
		bytes32 stateRoot;
	}

	function DecodeHeader(bytes encoded) external pure returns (HeaderOut);
	function EncodeCommitment(SolCommitment commitment) external pure returns (bytes);
	function EncodeLeaf(PartialBeefyMmrLeaf leaf) external pure returns (bytes);

	// EcdsaBeefy.verify — the canonical entrypoint used by the consensus router.
	// Returns (new encoded state, finalized intermediate states, next authority set id).
	function verify(bytes previousState, bytes proof)
		external
		pure
		returns (bytes, IntermediateState[], uint256);
}

fn deploy_beefy_test(env: &mut TestEnv) -> alloy_primitives::Address {
	let out_dir = env.evm_out_dir_public();
	env.deploy_named(&out_dir, "BeefyConsensusClientTest")
}

fn deploy_beefy_v1(env: &mut TestEnv) -> alloy_primitives::Address {
	let out_dir = env.evm_out_dir_public();
	env.deploy_named(&out_dir, "EcdsaBeefy")
}

#[test]
fn test_decode_encode() {
	let mut env = TestEnv::new();
	let beefy_test = deploy_beefy_test(&mut env);

	let header_bytes = hex!("9a28ac82dd089df2f5215ec55ae8b4933f9d58c8c76bf0c0ca1884f3778af2b7a53ba87a649f925c5093914299f42c78ad997b5f69a2ca5dc9ad3357cd0aeb6fd409566fe009ee37e1bbdc43af58c0be65d195bc3f0a5c98568bb12b709ef0d4f3be0806617572612038b856080000000005617572610101ecb27e1850a572d08ff0f4e94a1a557b0ddd7b12158627e442789802aada1553e65d72ecc3a6c0efb9794fb6c2ebf5878da36d6e5b8295cc0f42810beb64c68a").to_vec();
	let commitment_bytes = hex!("046d688088bc15df49c90d1823ac81aa90236815062561ccc4352983576013413e17c25a401e00005400000000000000").to_vec();
	let mmr_leaf_bytes = hex!("003f1e0000ccaf442e2648d278e87dbca890e532ef9cb7cf2058d023903b49567e2943996f550000000000000006000000a9d36172252f275bc8b7851062dff4a29e018355d8626c941f2ad57dfbabecd008ca13222c83d2a481d7b63c356d95bf9366b2a70e907ca3e38fa52e35731537").to_vec();

	let header = Header::<u32, BlakeTwo256>::decode(&mut &*header_bytes).unwrap();
	let commitment = BeefyCommitment::<u32>::decode(&mut &*commitment_bytes).unwrap();
	let mmr_leaf = MmrLeaf::<u32, H256, H256, H256>::decode(&mut &*mmr_leaf_bytes).unwrap();

	// --- DecodeHeader ---
	let call = DecodeHeaderCall { encoded: Bytes::from(header.encode()) };
	let result = env.call(beefy_test, call.abi_encode());
	let out = <DecodeHeaderCall as SolCall>::abi_decode_returns(&result).unwrap();
	assert_eq!(out.parentHash.0, header.parent_hash.0);
	assert_eq!(out.number, AlloyU256::from(header.number));
	assert_eq!(out.stateRoot.0, header.state_root.0);
	assert_eq!(out.extrinsicRoot.0, header.extrinsics_root.0);
	assert_eq!(out.digests.len(), header.digest.logs.len());

	// --- EncodeCommitment ---
	let mh_payload = commitment.payload.get_raw(b"mh").unwrap().clone();
	let sol_commitment = SolCommitment {
		payload: vec![PayloadItem { id: FixedBytes(*b"mh"), data: Bytes::from(mh_payload) }],
		blockNumber: AlloyU256::from(commitment.block_number),
		validatorSetId: AlloyU256::from(commitment.validator_set_id),
	};
	let call = EncodeCommitmentCall { commitment: sol_commitment };
	let result = env.call(beefy_test, call.abi_encode());
	let encoded = <EncodeCommitmentCall as SolCall>::abi_decode_returns(&result).unwrap();
	assert_eq!(encoded.to_vec(), commitment.encode());

	// --- EncodeLeaf ---
	let sol_leaf = PartialBeefyMmrLeaf {
		version: AlloyU256::ZERO,
		parentNumber: AlloyU256::from(mmr_leaf.parent_number_and_hash.0),
		parentHash: FixedBytes(mmr_leaf.parent_number_and_hash.1 .0),
		nextAuthoritySet: AuthoritySetCommitment {
			id: AlloyU256::from(mmr_leaf.beefy_next_authority_set.id),
			len: AlloyU256::from(mmr_leaf.beefy_next_authority_set.len),
			root: FixedBytes(mmr_leaf.beefy_next_authority_set.keyset_commitment.0),
		},
		extra: FixedBytes(mmr_leaf.leaf_extra.0),
	};
	let call = EncodeLeafCall { leaf: sol_leaf };
	let result = env.call(beefy_test, call.abi_encode());
	let encoded = <EncodeLeafCall as SolCall>::abi_decode_returns(&result).unwrap();
	assert_eq!(encoded.to_vec(), mmr_leaf.encode());

	// Sanity: signature decoding works (used by the consensus flow)
	let _ = Signature::decode(&mut &[0u8; 65][..]);
}

// ---------------------------------------------------------------------------
// Live integration test (ignored by default — requires running nodes)
// ---------------------------------------------------------------------------

fn default_para_id() -> u32 {
	2000
}
fn default_relay_ws_url() -> String {
	"ws://127.0.0.1:9922".to_string()
}
fn default_para_ws_url() -> String {
	"ws://127.0.0.1:9990".to_string()
}

#[derive(Deserialize, Debug)]
struct LiveConfig {
	#[serde(default = "default_relay_ws_url")]
	relay_ws_url: String,
	#[serde(default = "default_para_ws_url")]
	para_ws_url: String,
	#[serde(default = "default_para_id")]
	para_id: u32,
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running relay chain + parachain — set RELAY_WS_URL / PARA_WS_URL and run with --ignored"]
async fn test_beefy_consensus_client() -> Result<(), anyhow::Error> {
	let config = envy::from_env::<LiveConfig>()?;
	let LiveConfig { relay_ws_url, para_ws_url, para_id } = config;

	let (relay, relay_rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, u32::MAX).await?;
	let (para, para_rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&para_ws_url, u32::MAX).await?;

	let relay_rpc = LegacyRpcMethods::<PolkadotConfig>::new(relay_rpc_client.clone());
	let para_rpc = LegacyRpcMethods::<Hyperbridge>::new(para_rpc_client.clone());

	// Compute the beefy activation block from the storage value
	let metadata = relay.metadata();
	let hasher = <PolkadotConfig as subxt::Config>::Hasher::new(&metadata);
	let header = relay_rpc
		.chain_get_header(None)
		.await?
		.ok_or_else(|| anyhow!("No blocks on the relay chain?"))?;
	let header_hash = header.hash_with(hasher);
	let leaves = relay_rpc
		.state_get_storage(
			hex!("a8c65209d47ee80f56b0011e8fd91f508156209906244f2341137c136774c91d").as_slice(),
			Some(header_hash),
		)
		.await?
		.map(|data| u64::decode(&mut data.as_ref()))
		.transpose()?
		.ok_or_else(|| anyhow!("Couldn't fetch latest beefy authority set"))?;
	let activation_block = header.number.saturating_sub(leaves as u32);

	// Wait until the parachain is producing blocks
	para.blocks()
		.subscribe_best()
		.await
		.unwrap()
		.skip_while(|r| {
			futures::future::ready(match r {
				Ok(b) => b.number() < 5,
				Err(_) => false,
			})
		})
		.take(1)
		.collect::<Vec<_>>()
		.await;

	// Build the prover
	let prover = beefy_prover::Prover {
		beefy_activation_block: activation_block,
		relay: relay.clone(),
		relay_rpc: relay_rpc.clone(),
		relay_rpc_client: relay_rpc_client.clone(),
		para,
		para_rpc,
		para_rpc_client,
		para_ids: vec![para_id],
		query_batch_size: None,
	};

	// Initial consensus state
	let initial_state: ConsensusState = prover.get_initial_consensus_state(None).await?;
	let mut consensus_state: BeefyConsensusState = initial_state.into();

	// Deploy EcdsaBeefy directly. (The BeefyConsensusClientTest wrapper's `beefy` state
	// variable is only initialized in forge's setUp() which revm does not invoke.)
	let mut env = TestEnv::new();
	let beefy_v1 = deploy_beefy_v1(&mut env);

	// Subscribe to beefy justifications
	let subscription: RpcSubscription<String> = relay_rpc_client
		.subscribe(
			"beefy_subscribeJustifications",
			rpc_params![],
			"beefy_unsubscribeJustifications",
		)
		.await?;

	let mut subscription = subscription.take(5);
	while let Some(Ok(commitment_hex)) = subscription.next().await {
		let raw = if let Some(stripped) = commitment_hex.strip_prefix("0x") {
			hex::decode(stripped)?
		} else {
			hex::decode(&commitment_hex)?
		};
		let VersionedFinalityProof::V1(signed_commitment) =
			VersionedFinalityProof::<u32, Signature>::decode(&mut &*raw)?;

		// Skip commitments for outdated validator sets
		if signed_commitment.commitment.validator_set_id <
			consensus_state.currentAuthoritySet.id.try_into().unwrap_or(u64::MAX)
		{
			continue;
		}

		let proof: BeefyConsensusProof =
			prover.consensus_proof(signed_commitment.clone()).await?.into();

		if proof.relay.signedCommitment.commitment.blockNumber == consensus_state.latestHeight {
			continue;
		}

		println!(
			"verifying commitment @ block={} validatorSetId={} votes={}",
			proof.relay.signedCommitment.commitment.blockNumber,
			proof.relay.signedCommitment.commitment.validatorSetId,
			proof.relay.signedCommitment.votes.len(),
		);

		// EcdsaBeefy.verify expects proof bytes to decode as (RelayChainProof, ParachainProof)
		// — abi.decode(bytes, (A, B)) — which is tuple-params encoding, NOT a wrapper struct.
		let proof_bytes =
			SolValue::abi_encode_params(&(proof.relay.clone(), proof.parachain.clone()));
		let call = verifyCall {
			previousState: Bytes::from(consensus_state.abi_encode()),
			proof: Bytes::from(proof_bytes),
		};
		match env.call_as_may_revert(env.sender, beefy_v1, call.abi_encode()) {
			Ok(result) => {
				let ret = <verifyCall as SolCall>::abi_decode_returns(&result).unwrap();
				let new_state = <BeefyConsensusState as SolValue>::abi_decode(&ret._0).unwrap();
				assert!(
					new_state.latestHeight > consensus_state.latestHeight,
					"latestHeight should advance: old={} new={}",
					consensus_state.latestHeight,
					new_state.latestHeight,
				);
				println!(
					"  ✓ verified: new latestHeight={} nextAuthoritySetId={}",
					new_state.latestHeight, ret._2,
				);

				// Cross-check every IntermediateState against the live parachain header
				// at that height. The contract derives commitment fields from the header
				// digests (HeaderImpl.stateCommitment); stateRoot must match exactly.
				let intermediates = &ret._1;
				assert!(!intermediates.is_empty(), "verify returned no IntermediateStates",);
				println!("  intermediate states: {}", intermediates.len());
				for inter in intermediates {
					let height_u32: u32 = inter
						.height
						.try_into()
						.map_err(|_| anyhow!("intermediate height overflows u32"))?;
					let block_hash = prover
						.para_rpc
						.chain_get_block_hash(Some(height_u32.into()))
						.await?
						.ok_or_else(|| anyhow!("no parachain block at height {}", height_u32))?;
					let para_header = prover
						.para_rpc
						.chain_get_header(Some(block_hash))
						.await?
						.ok_or_else(|| anyhow!("no parachain header at {:?}", block_hash))?;
					// The contract derives the commitment from the header's ISMP consensus
					// digest (HeaderImpl.stateCommitment in src/consensus/Types.sol):
					//   overlayRoot = ConsensusDigest.mmr_root        (digest data [0..32])
					//   stateRoot   = ConsensusDigest.child_trie_root (digest data [32..])
					use subxt::config::substrate::DigestItem as SubxtDigestItem;
					let ismp_digest = para_header
						.digest
						.logs
						.iter()
						.find_map(|d| match d {
							SubxtDigestItem::Consensus(id, value) if *id == ISMP_ID =>
								Some(value.clone()),
							_ => None,
						})
						.ok_or_else(|| {
							anyhow!("no ISMP consensus digest in header at {}", height_u32)
						})?;
					let decoded = ConsensusDigest::decode(&mut &ismp_digest[..])
						.map_err(|e| anyhow!("decode ConsensusDigest at {}: {e}", height_u32))?;
					assert_eq!(
						inter.commitment.overlayRoot.0, decoded.mmr_root.0,
						"overlayRoot (mmr_root) mismatch at height {}: contract={:?} header={:?}",
						height_u32, inter.commitment.overlayRoot, decoded.mmr_root,
					);
					assert_eq!(
						inter.commitment.stateRoot.0, decoded.child_trie_root.0,
						"stateRoot (child_trie_root) mismatch at height {}: contract={:?} header={:?}",
						height_u32, inter.commitment.stateRoot, decoded.child_trie_root,
					);
					assert!(
						inter.commitment.timestamp > AlloyU256::ZERO,
						"timestamp must be non-zero at height {}",
						height_u32,
					);
					println!(
						"    ✓ height={} stateMachineId={} ts={} overlayRoot+stateRoot match ISMP digest",
						height_u32, inter.stateMachineId, inter.commitment.timestamp,
					);
				}

				consensus_state = new_state;
			},
			Err(revert) => {
				panic!(
					"EcdsaBeefy.verify reverted with output 0x{} at block {}",
					hex::encode(&revert),
					proof.relay.signedCommitment.commitment.blockNumber
				);
			},
		}
	}

	Ok(())
}
