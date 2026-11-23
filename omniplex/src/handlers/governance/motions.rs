//! Technical committee motions, and the calls the committee whitelists for the whitelisted
//! caller track.

use serde_json::{json, Value};

use super::{int, key, text, Context};

pub(super) const METHODS: &[&str] =
	&["Proposed", "Voted", "Approved", "Disapproved", "Executed", "Closed", "Killed"];

pub(super) async fn handle(ev: &Context<'_>) -> anyhow::Result<()> {
	let Some(hash) = ev.arg("proposalHash").as_str() else { return Ok(()) };
	let id = if ev.method == "Proposed" {
		propose(ev, hash).await?
	} else {
		let row = ev
			.db
			.query_opt(
				"SELECT id FROM governance.governance_motions WHERE hash = $1 AND NOT is_final
				 LIMIT 1",
				&[&hash],
			)
			.await?;
		match row {
			Some(row) => row.get(0),
			None => return Ok(()),
		}
	};

	// Votes come from the events, since a vote and a close in one block leave no `Voting` in
	// storage at its end. A changed vote moves the member to the other side.
	let voter = (ev.method == "Voted").then(|| ev.arg("account").as_str()).flatten();
	let aye = ev.arg("voted").as_bool().unwrap_or_default();
	let state = (ev.method != "Voted" && ev.method != "Closed").then_some(&ev.method);
	let is_final = ["Disapproved", "Executed", "Killed"].contains(&ev.method.as_str());
	let result = (ev.method == "Executed").then(|| ev.arg("result").to_string());
	ev.db
		.execute(
			"UPDATE governance.governance_motions SET
				ayes = CASE WHEN $2::text IS NULL THEN ayes
					ELSE (ayes - $2::text) || CASE WHEN $3 THEN jsonb_build_array($2::text)
						ELSE '[]'::jsonb END END,
				nays = CASE WHEN $2::text IS NULL THEN nays
					ELSE (nays - $2::text) || CASE WHEN $3 THEN '[]'::jsonb
						ELSE jsonb_build_array($2::text) END END,
				state = coalesce($4, state), is_final = is_final OR $5,
				result = coalesce($6, result), updated_block = $7
			 WHERE id = $1",
			&[&id, &voter, &aye, &state, &is_final, &result, &ev.position.block_number],
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

/** Stores a new motion with its call, returning its id. */
async fn propose(ev: &Context<'_>, hash: &str) -> anyhow::Result<String> {
	let index = int(ev.arg("proposalIndex")).unwrap_or_default();
	let hash_bytes = hex::decode(hash.trim_start_matches("0x"))?;
	let stored = ev
		.chain
		.storage_json(&[key("TechnicalCollective", "ProposalOf"), hash_bytes].concat(), ev.block)
		.await?;
	let call = match stored {
		Some(call) => Some(call),
		None => proposed_call(ev).await?,
	};
	// The motion can be closed from this block plus the motion duration on
	let duration = ev
		.chain
		.constant_json("TechnicalCollective", "MotionDuration", ev.block)
		.await?;
	let end = duration
		.as_ref()
		.and_then(int)
		.map(|duration| ev.position.block_number + duration);
	let p = &ev.position;
	ev.db
		.execute(
			"INSERT INTO governance.governance_motions (id, index, hash, proposer, threshold,
				call, ayes, nays, \"end\", state, is_final, proposed_block, proposed_at,
				updated_block)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $7, $8, 'Proposed', false, $9, $10, $9)
			 ON CONFLICT (id) DO UPDATE SET hash = EXCLUDED.hash, proposer = EXCLUDED.proposer,
				threshold = EXCLUDED.threshold, call = EXCLUDED.call, ayes = EXCLUDED.ayes,
				nays = EXCLUDED.nays, \"end\" = EXCLUDED.\"end\", state = EXCLUDED.state,
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
				&end,
				&p.block_number,
				&p.timestamp,
			],
		)
		.await?;
	Ok(index.to_string())
}

/**
 * The call of a motion that storage no longer holds at the end of its block, because it was
 * closed in that block too. It is taken from the extrinsic that proposed it: the extrinsic's
 * motions, in the order they ran, matched to the motions it has proposed so far.
 */
async fn proposed_call(ev: &Context<'_>) -> anyhow::Result<Option<Value>> {
	let Some(extrinsic) = ev.position.extrinsic_index else { return Ok(None) };
	let Some(call) = ev.chain.extrinsic_json(extrinsic as u32, ev.block).await? else {
		return Ok(None);
	};
	let earlier: i64 = ev
		.db
		.query_one(
			"SELECT count(*) FROM governance.governance_motion_events
			 WHERE block_number = $1 AND extrinsic_index = $2 AND name = 'Proposed'",
			&[&ev.position.block_number, &extrinsic],
		)
		.await?
		.get(0);
	Ok(motions(&call).into_iter().nth(earlier as usize))
}

/** The proposals of the `technicalCollective.propose` calls in a call, nested ones included,
 * in the order they run. A threshold below two executes at once and makes no motion. */
fn motions(value: &Value) -> Vec<Value> {
	let arg = |args: &Value, name: &str| {
		args.as_array()
			.and_then(|args| args.iter().find(|arg| arg["name"] == name))
			.map(|arg| arg["value"].clone())
	};
	match value {
		Value::Object(call)
			if call.get("section") == Some(&json!("technicalCollective")) &&
				call.get("method") == Some(&json!("propose")) =>
		{
			let threshold = arg(&call["args"], "threshold").as_ref().and_then(int);
			match (threshold, arg(&call["args"], "proposal")) {
				(Some(threshold), Some(proposal)) if threshold >= 2 => vec![proposal],
				_ => vec![],
			}
		},
		Value::Object(fields) => fields.values().flat_map(motions).collect(),
		Value::Array(items) => items.iter().flat_map(motions).collect(),
		_ => vec![],
	}
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

#[cfg(test)]
mod tests {
	use super::*;

	fn propose(threshold: u64, method: &str) -> Value {
		json!({ "section": "technicalCollective", "method": "propose", "args": [
			{ "name": "threshold", "value": threshold },
			{ "name": "proposal", "value": { "section": "whitelist", "method": method, "args": [] } },
			{ "name": "lengthBound", "value": 40 },
		]})
	}

	#[test]
	fn a_batch_yields_its_motions_in_order() {
		let batch = json!({ "section": "utility", "method": "batchAll", "args": [
			{ "name": "calls", "value": [propose(2, "whitelistCall"), propose(1, "now"), propose(3, "removeWhitelistedCall")] },
		]});
		let methods: Vec<_> = motions(&batch).iter().map(|call| call["method"].clone()).collect();
		assert_eq!(methods, [json!("whitelistCall"), json!("removeWhitelistedCall")]);
		assert!(motions(&json!({ "section": "system", "method": "remark", "args": [] })).is_empty());
	}
}
