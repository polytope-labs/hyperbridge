//! Preimages, whose bytes are kept after the chain clears them, so a proposal's call can still
//! be shown.

use codec::{Decode, Encode};

use super::{int, key, Context};

pub(super) const METHODS: &[&str] = &["Noted", "Requested", "Cleared"];

pub(super) async fn handle(ev: &Context<'_>) -> anyhow::Result<()> {
	let Some(hash) = ev.arg("hash").as_str() else { return Ok(()) };
	let block = ev.position.block_number;
	ev.db
		.execute(
			"INSERT INTO governance.governance_preimages (id, status, updated_block)
			 VALUES ($1, $2, $3)
			 ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status,
				updated_block = EXCLUDED.updated_block",
			&[&hash, &ev.method, &block],
		)
		.await?;
	if ev.method != "Noted" {
		return Ok(());
	}

	let noted = preimage(ev, hash).await?;
	ev.db
		.execute(
			"UPDATE governance.governance_preimages SET length = $2, data = $3, noted_block = $4
			 WHERE id = $1",
			&[
				&hash,
				&noted.as_ref().map(|(length, _)| *length),
				&noted.as_ref().map(|(_, data)| format!("0x{}", hex::encode(data))),
				&block,
			],
		)
		.await?;
	// A referendum can be submitted before its preimage is noted, so its call is decoded now
	let Some((_, data)) = noted else { return Ok(()) };
	let Some(call) = ev.chain.call_json(&data, ev.block).await? else { return Ok(()) };
	ev.db
		.execute(
			"UPDATE governance.governance_referendums SET call = $2
			 WHERE proposal_hash = $1 AND call IS NULL",
			&[&hash, &call.to_string()],
		)
		.await?;
	Ok(())
}

/**
 * A noted preimage's length and bytes as stored at this block. Its length is part of its
 * storage key, and comes from its request status: `RequestStatusFor`, or `StatusFor` in the
 * runtimes from before it.
 */
pub(super) async fn preimage(
	ev: &Context<'_>,
	hash: &str,
) -> anyhow::Result<Option<(i32, Vec<u8>)>> {
	let Ok(hash) = <[u8; 32]>::try_from(hex::decode(hash.trim_start_matches("0x"))?) else {
		return Ok(None);
	};
	let mut status = None;
	for entry in ["RequestStatusFor", "StatusFor"] {
		status = ev
			.chain
			.storage_json(&[key("Preimage", entry), hash.to_vec()].concat(), ev.block)
			.await?;
		if status.is_some() {
			break;
		}
	}
	// `{unrequested: {len}}`, or `{requested: {maybeLen}}` (`len` before that)
	let length = status
		.as_ref()
		.and_then(|status| status.as_object()?.values().next().cloned())
		.and_then(|fields| int(&fields["len"]).or_else(|| int(&fields["maybeLen"])));
	let Some(length) = length else { return Ok(None) };
	let key = [key("Preimage", "PreimageFor"), (hash, length as u32).encode()].concat();
	let stored = ev.chain.storage(&key, ev.block).await?;
	Ok(Vec::<u8>::decode(&mut &stored[..]).ok().map(|data| (length, data)))
}
