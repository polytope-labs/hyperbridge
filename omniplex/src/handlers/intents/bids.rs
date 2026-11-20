//! `intentsCoprocessor.BidPlaced` on Hyperbridge: one `filler_bids` row per bid, with the bid's
//! user operation recovered from the `place_bid` call that raised it.

use alloy::primitives::{keccak256, B256};
use codec::{Compact, Decode};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
	chains::Chain,
	registry::LogMeta,
	substrate::{ss58, SubstrateRpc},
};

/** Hyperbridge's SS58 prefix, on Nexus and Gargantua alike. */
pub const SS58_PREFIX: u8 = 0;
/** `place_bid`'s call index in the intents coprocessor pallet. */
const PLACE_BID_CALL_INDEX: u8 = 0;

/**
 * The `user_op` of the `place_bid(commitment, bid, user_op)` call inside a SCALE-encoded
 * extrinsic, wherever the call is nested (batches, proxies, sudo). The call is found by its
 * arguments: the 64 bytes of `commitment ‖ bid` behind `place_bid`'s call index, then the
 * compact-prefixed `user_op`. The commitment and bid make the match unambiguous without
 * decoding the rest of the extrinsic against runtime metadata.
 */
pub fn extract_user_op(extrinsic: &[u8], commitment: &B256, bid: &B256) -> Option<Vec<u8>> {
	let mut needle = [0u8; 64];
	needle[..32].copy_from_slice(commitment.as_slice());
	needle[32..].copy_from_slice(bid.as_slice());
	(2..extrinsic.len().saturating_sub(63)).find_map(|start| {
		if extrinsic[start..start + 64] != needle || extrinsic[start - 1] != PLACE_BID_CALL_INDEX {
			return None;
		}
		let mut rest = &extrinsic[start + 64..];
		let Compact(length) = Compact::<u32>::decode(&mut rest).ok()?;
		rest.get(..length as usize).map(<[u8]>::to_vec)
	})
}

/** What the filler bid row needs beyond the log. */
pub struct BidSource {
	/** The bid's user operation, hex. */
	pub bid_data: Option<String>,
	/** The extrinsic's hash, or `None` when the event wasn't raised by an extrinsic. */
	pub extrinsic_hash: Option<String>,
}

#[derive(Deserialize)]
struct RpcBid {
	filler: Option<String>,
	bid: Option<String>,
	user_op: Option<String>,
}

/**
 * The extrinsics of the log's block, or `None` when the node pruned the block's body. The bid's
 * data then comes from `intents_getBidsForOrder`.
 *
 * A node also answers null for a block it hasn't imported. So after a null, the body is read
 * again once a node has the block, from that node.
 */
async fn block_extrinsics(rpc: &SubstrateRpc, log: &LogMeta) -> anyhow::Result<Option<Vec<Value>>> {
	let read_body = || async {
		let block: Value = rpc.call("chain_getBlock", json!([log.block_hash])).await?;
		anyhow::Ok(block.pointer("/block/extrinsics").and_then(Value::as_array).cloned())
	};
	if let Some(body) = read_body().await? {
		return Ok(Some(body));
	}
	rpc.ensure_imported(log.block_number).await?;
	read_body().await
}

/**
 * `resolveBidData`: the extrinsic from `chain_getBlock`, else the node's
 * `intents_getBidsForOrder`, which only knows bids that are still live. Failing to read the
 * block fails the event, so a transient RPC error retries rather than storing no payload.
 */
pub async fn resolve_bid_source(
	chain: &Chain,
	log: &LogMeta,
	filler: &B256,
	commitment: &B256,
	bid: &B256,
) -> anyhow::Result<BidSource> {
	let rpc = chain.substrate()?;
	let extrinsics: Vec<Vec<u8>> = match block_extrinsics(rpc, log).await? {
		Some(extrinsics) => extrinsics
			.iter()
			.map(|e| hex::decode(e.as_str().unwrap_or_default().trim_start_matches("0x")))
			.collect::<Result<_, _>>()?,
		None => {
			tracing::warn!(block = %log.block_hash, "chain_getBlock returned no body; bid data from the node's live bids");
			vec![]
		},
	};
	// The facade hashes extrinsics with the chain's hasher (keccak on Hyperbridge); prefer the
	// extrinsic with the log's hash and fall back to its index.
	let extrinsic = extrinsics
		.iter()
		.find(|e| keccak256(e) == log.transaction_hash)
		.or_else(|| extrinsics.get(log.transaction_index as usize));
	let extrinsic_hash = extrinsic.map(|_| log.transaction_hash.to_string());
	if let Some(user_op) = extrinsic.and_then(|e| extract_user_op(e, commitment, bid)) {
		return Ok(BidSource {
			bid_data: Some(format!("0x{}", hex::encode(user_op))),
			extrinsic_hash,
		});
	}

	let filler_hex = filler.to_string();
	let bid_hex = bid.to_string();
	let bid_data =
		match rpc.call::<Vec<RpcBid>>("intents_getBidsForOrder", json!([commitment])).await {
			Ok(bids) => bids
				.into_iter()
				.find(|entry| {
					entry.filler.as_deref().is_some_and(|f| f.eq_ignore_ascii_case(&filler_hex)) &&
						entry.bid.as_deref().is_some_and(|b| b.eq_ignore_ascii_case(&bid_hex))
				})
				.and_then(|entry| entry.user_op)
				.filter(|user_op| !user_op.is_empty()),
			Err(err) if crate::errors::is_transient(&err) => return Err(err),
			Err(err) => {
				tracing::warn!(%commitment, "intents_getBidsForOrder failed: {err:#}");
				None
			},
		};
	Ok(BidSource { bid_data, extrinsic_hash })
}

/** `filler_bids` id: a filler may bid on an order again, so the block and event are in it. */
pub fn bid_id(commitment: &B256, filler: &B256, block_number: u64, log_index: u64) -> String {
	format!("{commitment}-{}-{block_number}-{log_index}", ss58(&filler.0, SS58_PREFIX))
}

#[cfg(test)]
mod tests {
	use super::*;
	use codec::Encode;

	#[tokio::test]
	async fn a_block_one_node_is_behind_is_read_from_the_node_that_has_it() {
		use crate::failover::tests::{MockRpc, Reply};
		let log = LogMeta {
			address: Default::default(),
			block_number: 7,
			block_hash: B256::repeat_byte(7),
			block_timestamp: 0,
			transaction_hash: B256::ZERO,
			transaction_index: 0,
			log_index: 0,
		};
		let behind = MockRpc::start(Reply::Null).await;
		let ahead =
			MockRpc::start(Reply::Body(r#"{"result":{"block":{"extrinsics":["0x01"]}}}"#)).await;
		let rpc = SubstrateRpc::new(&[&behind.url, &ahead.url], reqwest::Client::new());
		assert_eq!(block_extrinsics(&rpc, &log).await.unwrap(), Some(vec![json!("0x01")]));
		// With every node behind, the bid is retried rather than stored without its extrinsic.
		let rpc = SubstrateRpc::new(&[&behind.url], reqwest::Client::new());
		let err = block_extrinsics(&rpc, &log).await.unwrap_err();
		assert!(crate::errors::is_transient(&err), "{err:#}");
		// A node that has the block and still answers without a body has pruned it.
		let pruned = MockRpc::start(Reply::Body(r#"{"result":"0x07"}"#)).await;
		let rpc = SubstrateRpc::new(&[&pruned.url], reqwest::Client::new());
		assert_eq!(block_extrinsics(&rpc, &log).await.unwrap(), None);
	}

	fn place_bid(pallet: u8, commitment: B256, bid: B256, user_op: &[u8]) -> Vec<u8> {
		let mut call = vec![pallet, PLACE_BID_CALL_INDEX];
		call.extend_from_slice(commitment.as_slice());
		call.extend_from_slice(bid.as_slice());
		call.extend(user_op.to_vec().encode());
		call
	}

	#[test]
	fn finds_the_matching_bid_wherever_it_is_nested() {
		let commitment = B256::repeat_byte(0x11);
		let bid = B256::repeat_byte(0x22);
		let other = B256::repeat_byte(0x33);
		// A signed extrinsic's header, then the call.
		let header = [0x84u8, 0x00, 0xaa, 0xbb];
		let direct = [&header[..], &place_bid(94, commitment, bid, b"op")].concat();
		assert_eq!(extract_user_op(&direct, &commitment, &bid), Some(b"op".to_vec()));

		// utility.batch(vec![place_bid(other order), place_bid(this), place_bid(same order, other
		// bid)])
		let mut batch = vec![0x28, 0x00];
		batch.extend(Compact(3u32).encode());
		batch.extend(place_bid(94, other, bid, b"first"));
		batch.extend(place_bid(94, commitment, bid, b"second"));
		batch.extend(place_bid(94, commitment, other, b"third"));
		let batched = [&header[..], &batch].concat();
		assert_eq!(extract_user_op(&batched, &commitment, &bid), Some(b"second".to_vec()));
		assert_eq!(extract_user_op(&batched, &commitment, &other), Some(b"third".to_vec()));
		assert_eq!(extract_user_op(&batched, &other, &other), None);

		// The same bytes behind another call index aren't a place_bid.
		let mut wrong_call = place_bid(94, commitment, bid, b"op");
		wrong_call[1] = 1;
		assert_eq!(extract_user_op(&[&header[..], &wrong_call].concat(), &commitment, &bid), None);
		// A truncated payload is no match.
		let truncated = &direct[..direct.len() - 1];
		assert_eq!(extract_user_op(truncated, &commitment, &bid), None);
	}

	#[test]
	fn bid_ids_use_the_ss58_filler() {
		let commitment: B256 = "0x1e5aac55b5069b58d6a053f332b737a26d4b7d3837cdcecc5a4485f460927307"
			.parse()
			.unwrap();
		let alice: B256 = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
			.parse()
			.unwrap();
		assert_eq!(
			bid_id(&commitment, &alice, 12597100, 7),
			"0x1e5aac55b5069b58d6a053f332b737a26d4b7d3837cdcecc5a4485f460927307-15oF4uVJwmo4TdGW7VfQxNLavjCXviqxT9S1MgbjMNHr6Sp5-12597100-7"
		);
	}
}
