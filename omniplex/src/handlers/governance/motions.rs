//! Technical committee motions, and the calls the committee whitelists for the whitelisted
//! caller track.

use alloy::primitives::B256;
use serde_json::{json, Value};

use super::{int, key, text, Context};

pub(super) const METHODS: &[&str] =
	&["Proposed", "Voted", "Approved", "Disapproved", "Executed", "Closed", "Killed"];

pub(super) async fn handle(ev: &Context<'_>) -> anyhow::Result<()> {
	let Some(hash) = ev.arg("proposalHash").as_str() else { return Ok(()) };
	let Ok(hash_bytes) = hex::decode(hash.trim_start_matches("0x")) else { return Ok(()) };
	let (id, updated_block) = if ev.method == "Proposed" {
		(propose(ev, hash, &hash_bytes).await?, ev.position.block_number)
	} else {
		let row = ev
			.db
			.query_opt(
				"SELECT id, updated_block FROM governance.governance_motions
				 WHERE hash = $1 AND NOT is_final LIMIT 1",
				&[&hash],
			)
			.await?;
		match row {
			Some(row) => (row.get(0), row.get::<_, i32>(1)),
			None => return Ok(()),
		}
	};

	// `Voting` holds an open motion's votes and deadline, and loses the votes of members who
	// leave the committee, which emits no event. A close in this block removes it, so the
	// votes start from `Voting` at the end of the block before, when the motion was last
	// updated in an earlier block, and the event's own vote is applied on top, moving a changed
	// vote to the other side.
	let voting_key = [key("TechnicalCollective", "Voting"), hash_bytes].concat();
	let voting = ev.chain.storage_json(&voting_key, ev.block).await?;
	let closed_here = voting.is_none();
	let voting = match voting {
		None if updated_block < ev.position.block_number =>
			ev.chain.storage_json(&voting_key, ev.block - 1).await?,
		voting => voting,
	};
	let voters = |side: &str| voting.as_ref().map(|voting| voting[side].clone());
	let voter = match closed_here {
		true if ev.method == "Voted" => ev.arg("account").as_str(),
		_ => None,
	};
	let aye = ev.arg("voted").as_bool().unwrap_or_default();
	let state = (ev.method != "Voted" && ev.method != "Closed").then_some(&ev.method);
	let is_final = ["Disapproved", "Executed", "Killed"].contains(&ev.method.as_str());
	let result = (ev.method == "Executed").then(|| ev.arg("result").to_string());
	ev.db
		.execute(
			"UPDATE governance.governance_motions SET
				ayes = CASE WHEN $4::text IS NULL THEN coalesce($2::jsonb, ayes)
					ELSE (coalesce($2::jsonb, ayes) - $4::text) || CASE WHEN $5
						THEN jsonb_build_array($4::text) ELSE '[]'::jsonb END END,
				nays = CASE WHEN $4::text IS NULL THEN coalesce($3::jsonb, nays)
					ELSE (coalesce($3::jsonb, nays) - $4::text) || CASE WHEN $5
						THEN '[]'::jsonb ELSE jsonb_build_array($4::text) END END,
				\"end\" = coalesce($6, \"end\"), state = coalesce($7, state),
				is_final = is_final OR $8, result = coalesce($9, result), updated_block = $10
			 WHERE id = $1",
			&[
				&id,
				&voters("ayes"),
				&voters("nays"),
				&voter,
				&aye,
				&voting.as_ref().and_then(|voting| int(&voting["end"])),
				&state,
				&is_final,
				&result,
				&ev.position.block_number,
			],
		)
		.await?;

	let p = &ev.position;
	ev.db
		.execute(
			"INSERT INTO governance.governance_motion_events (id, motion_id, name, args,
				block_number, block_hash, extrinsic_index, event_index, \"timestamp\")
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT (id) DO NOTHING",
			&[
				&p.id,
				&id,
				&ev.method,
				&ev.args_text(),
				&p.block_number,
				&p.block_hash,
				&p.extrinsic_index,
				&p.event_index,
				&p.timestamp,
			],
		)
		.await?;
	Ok(())
}

/** Stores a new motion with its call, returning its id. Its votes and deadline follow from
 * `Voting`, read for every motion event. */
async fn propose(ev: &Context<'_>, hash: &str, hash_bytes: &[u8]) -> anyhow::Result<String> {
	let index = int(ev.arg("proposalIndex")).unwrap_or_default();
	let stored = ev
		.chain
		.storage_json(
			&[key("TechnicalCollective", "ProposalOf"), hash_bytes.to_vec()].concat(),
			ev.block,
		)
		.await?;
	let call = match stored {
		Some(call) => Some(call),
		None => proposed_call(ev, hash_bytes).await?,
	};
	let p = &ev.position;
	ev.db
		.execute(
			"INSERT INTO governance.governance_motions (id, index, hash, proposer, threshold,
				call, ayes, nays, \"end\", state, is_final, proposed_block, proposed_at,
				updated_block)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $7, NULL, 'Proposed', false, $8, $9, $8)
			 ON CONFLICT (id) DO UPDATE SET hash = EXCLUDED.hash, proposer = EXCLUDED.proposer,
				threshold = EXCLUDED.threshold, call = EXCLUDED.call, ayes = EXCLUDED.ayes,
				nays = EXCLUDED.nays, \"end\" = NULL, state = EXCLUDED.state,
				is_final = false, proposed_block = EXCLUDED.proposed_block,
				proposed_at = EXCLUDED.proposed_at, updated_block = EXCLUDED.updated_block",
			&[
				&index.to_string(),
				&index,
				&hash,
				&ev.arg("account").as_str().unwrap_or_default(),
				&int(ev.arg("threshold")).unwrap_or_default(),
				&call.as_ref().map(Value::to_string),
				&json!([]),
				&p.block_number,
				&p.timestamp,
			],
		)
		.await?;
	Ok(index.to_string())
}

/**
 * The call of a motion that storage no longer holds at the end of its block, because it was
 * closed in that block too. It is the call in the proposing extrinsic that hashes to the
 * motion's hash, so a batch's failed proposals can't stand in for it.
 */
async fn proposed_call(ev: &Context<'_>, hash: &[u8]) -> anyhow::Result<Option<Value>> {
	let (Some(extrinsic), Ok(hash)) = (ev.position.extrinsic_index, B256::try_from(hash)) else {
		return Ok(None);
	};
	ev.chain.extrinsic_call_json(extrinsic as u32, hash, ev.block).await
}

pub(super) async fn whitelist(ev: &Context<'_>) -> anyhow::Result<()> {
	let Some(hash) = ev.arg("callHash").as_str() else { return Ok(()) };
	let block = ev.position.block_number;
	// A call first seen being removed or dispatched counts as whitelisted at that block
	let (column, result) = match ev.method.as_str() {
		"CallWhitelisted" => ("whitelisted_block", None),
		"WhitelistedCallRemoved" => ("removed_block", None),
		"WhitelistedCallDispatched" => ("dispatched_block", text(ev.arg("result"))),
		_ => return Ok(()),
	};
	ev.db
		.execute(
			"INSERT INTO governance.governance_whitelisted_calls (id, whitelisted_block)
			 VALUES ($1, $2) ON CONFLICT (id) DO NOTHING",
			&[&hash, &block],
		)
		.await?;
	ev.db
		.execute(
			&format!(
				"UPDATE governance.governance_whitelisted_calls SET {column} = $2,
					dispatch_result = coalesce($3, dispatch_result)
				 WHERE id = $1"
			),
			&[&hash, &block, &result],
		)
		.await?;
	Ok(())
}
