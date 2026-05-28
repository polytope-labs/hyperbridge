//! Simnode coverage for the runtime base call filter (`IsmpCallFilter`).
//!
//! The filter blocks `Ismp::fund_message` outright and blocks `Ismp::handle_unsigned`
//! when the batch carries a BEEFY consensus update. We submit each call as a *signed*
//! extrinsic on purpose: a signed extrinsic skips `validate_unsigned` and goes straight
//! to dispatch, where the filter runs before the call body. A blocked call comes back as
//! `System::CallFiltered`; a call the filter lets through reaches the body and trips
//! `ensure_none` with `BadOrigin`. Telling those two apart is the whole test, and it lets
//! us exercise the filter without building real BEEFY proofs.
//!
//! The consensus-state to client mappings are seeded straight into storage with
//! `System::set_storage`, since the filter only reads `Ismp::ConsensusStateClient` and
//! never the consensus state itself.

#![cfg(test)]

use std::env;

use anyhow::anyhow;
use codec::Encode;
use polkadot_sdk::{
	sp_io::hashing::{blake2_128, twox_128},
	*,
};
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{crypto::Ss58Codec, Bytes};
use sp_keyring::sr25519::Keyring;

use ismp::messaging::{ConsensusMessage, Message};
use primitive_types::H256;
use subxt::{
	dynamic::Value,
	error::DispatchError,
	ext::{
		scale_value::Composite,
		subxt_rpcs::{rpc_params, RpcClient},
	},
	tx::{DynamicPayload, SubmittableTransaction},
	OnlineClient,
};
use subxt_utils::{
	values::{messages_to_value, storage_kv_list_to_value},
	Hyperbridge,
};

/// BEEFY consensus client id (`b"BEEF"`), duplicated here rather than pulling in
/// `ismp-beefy` for one constant, as `pallet_beefy_consensus_proofs` also does.
const BEEFY_CONSENSUS_ID: [u8; 4] = *b"BEEF";
/// ISMP parachain consensus client id (`b"PARA"`), standing in for any non-BEEFY client.
const PARACHAIN_CONSENSUS_ID: [u8; 4] = *b"PARA";

/// Synthetic consensus state ids the test seeds itself, chosen so they can't collide with
/// state a running node already tracks. One is bound to BEEFY, one to the parachain client,
/// and one is left unmapped so `consensus_client_id` returns `None`.
const BEEFY_STATE_ID: [u8; 4] = *b"TBF1";
const OTHER_STATE_ID: [u8; 4] = *b"TBF2";
const UNMAPPED_STATE_ID: [u8; 4] = *b"TBF3";

/// Storage key for `Ismp::ConsensusStateClient`, a `Blake2_128Concat` map keyed by
/// `ConsensusStateId`.
fn consensus_state_client_key(state_id: &[u8; 4]) -> Vec<u8> {
	[
		twox_128(b"Ismp").as_slice(),
		twox_128(b"ConsensusStateClient").as_slice(),
		blake2_128(state_id).as_slice(),
		state_id.as_slice(),
	]
	.concat()
}

/// A consensus update for `state_id`. The proof and signer are empty because the filter
/// only inspects `consensus_state_id` and the call never reaches its body.
fn consensus_message(state_id: [u8; 4]) -> Message {
	Message::Consensus(ConsensusMessage {
		consensus_proof: vec![],
		consensus_state_id: state_id,
		signer: vec![],
	})
}

/// A `handle_unsigned` payload whose batch is a single consensus update for `state_id`.
fn consensus_batch(state_id: [u8; 4]) -> DynamicPayload {
	subxt::dynamic::tx(
		"Ismp",
		"handle_unsigned",
		vec![messages_to_value(vec![consensus_message(state_id)])],
	)
}

/// A `fund_message` payload. Contents are arbitrary since the filter rejects it before the
/// commitment is ever looked up; we only need it to decode.
fn fund_message() -> DynamicPayload {
	let commitment = Value::variant(
		"Request",
		Composite::unnamed(vec![Value::unnamed_composite(vec![Value::from_bytes([0u8; 32])])]),
	);
	let params =
		Value::named_composite(vec![("commitment", commitment), ("amount", Value::u128(0))]);
	subxt::dynamic::tx("Ismp", "fund_message", vec![params])
}

/// Submit a sudo-wrapped call signed by Alice and wait for finalization.
async fn submit_sudo(
	client: &OnlineClient<Hyperbridge>,
	rpc_client: &RpcClient,
	inner: DynamicPayload,
) -> Result<(), anyhow::Error> {
	let sudo = subxt::dynamic::tx("Sudo", "sudo", vec![inner.into_value()]);
	let call_data = client.tx().call_data(&sudo)?;
	let extrinsic: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call_data), Keyring::Alice.to_account_id().to_ss58check()],
		)
		.await
		.map_err(|err| anyhow!("simnode_authorExtrinsic failed: {err:?}"))?;
	let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
	let progress = submittable.submit_and_watch().await?;
	let block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;
	let finalized = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;
	assert!(finalized);
	progress.wait_for_finalized_success().await?;
	Ok(())
}

/// Author `call` signed by `signer`, seal and finalize a block, and return the dispatch
/// error. Every call in this test is expected to fail at dispatch (filtered or `BadOrigin`),
/// so a successful dispatch is itself a test failure.
async fn dispatch_error(
	client: &OnlineClient<Hyperbridge>,
	rpc_client: &RpcClient,
	call: DynamicPayload,
	signer: Keyring,
) -> Result<DispatchError, anyhow::Error> {
	let call_data = client.tx().call_data(&call)?;
	let extrinsic: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call_data), signer.to_account_id().to_ss58check()],
		)
		.await
		.map_err(|err| anyhow!("simnode_authorExtrinsic failed: {err:?}"))?;
	let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
	let progress = submittable.submit_and_watch().await?;
	let block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;
	let finalized = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;
	assert!(finalized);
	match progress.wait_for_finalized_success().await {
		Ok(_) => Err(anyhow!("call dispatched successfully, expected it to fail")),
		Err(subxt::Error::Runtime(err)) => Ok(err),
		Err(other) => Err(anyhow!("expected a runtime dispatch error, got: {other:?}")),
	}
}

/// True when the base call filter rejected the call before its body ran.
fn is_call_filtered(err: &DispatchError) -> bool {
	let DispatchError::Module(module) = err else { return false };
	matches!(
		module.details(),
		Ok(details)
			if details.pallet.name() == "System" && details.variant.name.as_str() == "CallFiltered"
	)
}

/// True when the call passed the filter and reached the body, where `ensure_none` rejected
/// the signed origin.
fn is_bad_origin(err: &DispatchError) -> bool {
	matches!(err, DispatchError::BadOrigin)
}

#[tokio::test]
#[ignore]
async fn ismp_call_filter_blocks_only_beefy_consensus_and_fund_message() -> Result<(), anyhow::Error>
{
	let port = env::var("PORT").unwrap_or_else(|_| "9990".into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	// Bind one state to BEEFY and one to the parachain client. `UNMAPPED_STATE_ID` is left out
	// so it resolves to no client at all.
	let kv_list: Vec<(Vec<u8>, Vec<u8>)> = vec![
		(consensus_state_client_key(&BEEFY_STATE_ID), BEEFY_CONSENSUS_ID.encode()),
		(consensus_state_client_key(&OTHER_STATE_ID), PARACHAIN_CONSENSUS_ID.encode()),
	];
	let set_storage =
		subxt::dynamic::tx("System", "set_storage", vec![storage_kv_list_to_value(&kv_list)]);
	submit_sudo(&client, &rpc_client, set_storage).await?;

	// fund_message is always filtered.
	let err = dispatch_error(&client, &rpc_client, fund_message(), Keyring::Bob).await?;
	assert!(is_call_filtered(&err), "fund_message must be filtered, got {err:?}");

	// handle_unsigned carrying a BEEFY consensus update is filtered.
	let err =
		dispatch_error(&client, &rpc_client, consensus_batch(BEEFY_STATE_ID), Keyring::Bob).await?;
	assert!(
		is_call_filtered(&err),
		"beefy consensus handle_unsigned must be filtered, got {err:?}"
	);

	// A batch is filtered as long as it carries a BEEFY update, even mixed with others.
	let mixed = subxt::dynamic::tx(
		"Ismp",
		"handle_unsigned",
		vec![messages_to_value(vec![
			consensus_message(OTHER_STATE_ID),
			consensus_message(BEEFY_STATE_ID),
		])],
	);
	let err = dispatch_error(&client, &rpc_client, mixed, Keyring::Bob).await?;
	assert!(
		is_call_filtered(&err),
		"mixed batch with a beefy update must be filtered, got {err:?}"
	);

	// A consensus update for a non-BEEFY client passes the filter and reaches the body.
	let err =
		dispatch_error(&client, &rpc_client, consensus_batch(OTHER_STATE_ID), Keyring::Bob).await?;
	assert!(
		is_bad_origin(&err),
		"non-beefy consensus handle_unsigned must pass the filter, got {err:?}"
	);

	// An unmapped consensus state resolves to no client, so it is not treated as BEEFY.
	let err =
		dispatch_error(&client, &rpc_client, consensus_batch(UNMAPPED_STATE_ID), Keyring::Bob)
			.await?;
	assert!(
		is_bad_origin(&err),
		"unmapped consensus handle_unsigned must pass the filter, got {err:?}"
	);

	Ok(())
}

#[tokio::test]
#[ignore]
async fn collator_selection_leave_intent_is_filtered() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or_else(|_| "9990".into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	// The filter runs before the call body, so the caller doesn't need to be a candidate.
	let leave_intent =
		subxt::dynamic::tx("CollatorSelection", "leave_intent", Composite::unnamed(vec![]));
	let err = dispatch_error(&client, &rpc_client, leave_intent, Keyring::Bob).await?;
	assert!(
		is_call_filtered(&err),
		"CollatorSelection::leave_intent must be blocked by the base call filter, got {err:?}"
	);

	Ok(())
}
