//! Referenda, the tracks they run on, their tally history, and the enactment the scheduler
//! dispatches once one is confirmed.

use chrono::NaiveDateTime;
use codec::Encode;
use serde_json::{json, Value};
use sp_crypto_hashing::blake2_256;
use tokio_postgres::{Row, Transaction};

use super::{amount, blake2_128_concat, int, key, preimages, Context};
use crate::store::{numeric, Numeric};

pub(super) const METHODS: &[&str] = &[
	"Submitted",
	"DecisionDepositPlaced",
	"DecisionDepositRefunded",
	"SubmissionDepositRefunded",
	"DecisionStarted",
	"ConfirmStarted",
	"ConfirmAborted",
	"Confirmed",
	"Approved",
	"Rejected",
	"TimedOut",
	"Cancelled",
	"Killed",
	"MetadataSet",
	"MetadataCleared",
];

/** Deposits are refunded long after a referendum ends, so refunds stay in its history without
 * changing its state. */
const DEPOSIT_REFUNDS: &[&str] = &["DecisionDepositRefunded", "SubmissionDepositRefunded"];

const COLUMNS: &[&str] = &[
	"id",
	"index",
	"track",
	"proposer",
	"proposal",
	"proposal_hash",
	"call",
	"state",
	"is_final",
	"ayes",
	"nays",
	"support",
	"electorate",
	"info",
	"enactment_task",
	"enactment_result",
	"submitted_block",
	"submitted_at",
	"updated_block",
	"track_info",
];

#[derive(Clone, Copy, Default)]
struct Tally {
	ayes: u128,
	nays: u128,
	support: u128,
}

impl Tally {
	fn from_json(value: &Value) -> Option<Self> {
		Some(Self {
			ayes: amount(&value["ayes"])?,
			nays: amount(&value["nays"])?,
			support: amount(&value["support"])?,
		})
	}
}

struct Referendum {
	index: i32,
	track: i32,
	proposer: Option<String>,
	/** The proposal exactly as `Submitted` carries it: a preimage lookup, an inline call or a
	 * legacy hash. */
	proposal: String,
	proposal_hash: Option<String>,
	/** The proposed call, decoded with the runtime of the block it became known in. */
	call: Option<String>,
	/** The name of the last event that changed it, such as `DecisionStarted`. */
	state: String,
	is_final: bool,
	tally: Tally,
	electorate: Option<u128>,
	/** Its details while it was live, kept once it ends, since storage then only holds the
	 * outcome. */
	info: Option<String>,
	enactment_task: Option<String>,
	enactment_result: Option<String>,
	submitted_block: i32,
	submitted_at: NaiveDateTime,
	updated_block: i32,
	/** Its track's definition at its last state change, since runtime upgrades change it. */
	track_info: Option<String>,
}

impl Referendum {
	async fn load(db: &Transaction<'_>, index: i32) -> anyhow::Result<Option<Self>> {
		let sql = format!(
			"SELECT {} FROM governance.governance_referendums WHERE id = $1",
			COLUMNS.join(", ")
		);
		let row = db.query_opt(&sql, &[&index.to_string()]).await?;
		row.map(|row| Self::from_row(&row)).transpose()
	}

	fn from_row(row: &Row) -> anyhow::Result<Self> {
		let big = |column: &str| -> anyhow::Result<u128> {
			Ok(row.get::<_, Numeric>(column).as_str().parse()?)
		};
		let electorate = row.get::<_, Option<Numeric>>("electorate");
		Ok(Self {
			index: row.get("index"),
			track: row.get("track"),
			proposer: row.get("proposer"),
			proposal: row.get("proposal"),
			proposal_hash: row.get("proposal_hash"),
			call: row.get("call"),
			state: row.get("state"),
			is_final: row.get("is_final"),
			tally: Tally { ayes: big("ayes")?, nays: big("nays")?, support: big("support")? },
			electorate: electorate.map(|e| e.as_str().parse()).transpose()?,
			info: row.get("info"),
			enactment_task: row.get("enactment_task"),
			enactment_result: row.get("enactment_result"),
			submitted_block: row.get("submitted_block"),
			submitted_at: row.get("submitted_at"),
			updated_block: row.get("updated_block"),
			track_info: row.get("track_info"),
		})
	}

	async fn save(&self, db: &Transaction<'_>) -> anyhow::Result<()> {
		let placeholders: Vec<String> = (1..=COLUMNS.len()).map(|i| format!("${i}")).collect();
		let updates: Vec<String> = COLUMNS[1..]
			.iter()
			.map(|column| format!("{column} = EXCLUDED.{column}"))
			.collect();
		let sql = format!(
			"INSERT INTO governance.governance_referendums ({}) VALUES ({})
			 ON CONFLICT (id) DO UPDATE SET {}",
			COLUMNS.join(", "),
			placeholders.join(", "),
			updates.join(", ")
		);
		db.execute(
			&sql,
			&[
				&self.index.to_string(),
				&self.index,
				&self.track,
				&self.proposer,
				&self.proposal,
				&self.proposal_hash,
				&self.call,
				&self.state,
				&self.is_final,
				&numeric(self.tally.ayes),
				&numeric(self.tally.nays),
				&numeric(self.tally.support),
				&self.electorate.map(numeric),
				&self.info,
				&self.enactment_task,
				&self.enactment_result,
				&self.submitted_block,
				&self.submitted_at,
				&self.updated_block,
				&self.track_info,
			],
		)
		.await?;
		Ok(())
	}
}

pub(super) async fn handle(ev: &Context<'_>) -> anyhow::Result<()> {
	let Some(index) = int(ev.arg("index")) else { return Ok(()) };
	let info = referendum_info(ev, index, ev.block).await?;
	let ongoing = info.as_ref().and_then(|info| info.get("ongoing"));

	let mut referendum = match Referendum::load(ev.db, index).await? {
		Some(referendum) => referendum,
		None if ev.method == "Submitted" => {
			let proposal = ev.arg("proposal");
			let proposer = ongoing.and_then(|o| o["submissionDeposit"]["who"].as_str());
			sync_tracks(ev).await?;
			Referendum {
				index,
				track: int(ev.arg("track")).unwrap_or_default(),
				proposer: proposer.map(String::from),
				proposal: proposal.to_string(),
				proposal_hash: proposal_hash(proposal),
				call: proposal_call(ev, proposal).await?,
				state: ev.method.clone(),
				is_final: false,
				tally: Tally::default(),
				electorate: None,
				info: None,
				enactment_task: None,
				enactment_result: None,
				submitted_block: ev.position.block_number,
				submitted_at: ev.position.timestamp,
				updated_block: ev.position.block_number,
				track_info: None,
			}
		},
		None => {
			tracing::warn!(index, method = %ev.method, "referendum has no Submitted event before this");
			return Ok(());
		},
	};

	// Ongoing referenda are read from storage, finishing events carry the final tally
	let tally = ongoing
		.and_then(|ongoing| Tally::from_json(&ongoing["tally"]))
		.or_else(|| Tally::from_json(ev.arg("tally")));
	if let Some(tally) = tally {
		referendum.tally = tally;
	}
	// The scheduler moves a referendum's alarm without an event, so when it ends, the block
	// before is the only place its final live details are kept
	let live = match ongoing {
		Some(ongoing) => Some(ongoing.to_string()),
		None if !referendum.is_final => {
			let before = referendum_info(ev, index, ev.block - 1).await?;
			before.and_then(|info| info.get("ongoing").map(Value::to_string))
		},
		None => None,
	};
	if live.is_some() {
		referendum.info = live;
	}
	// Deposit refunds arrive after a referendum ends, and its tally was measured against the
	// electorate when it ended
	if !referendum.is_final {
		referendum.electorate = Some(ev.electorate().await?);
	}
	if ev.method == "Confirmed" {
		referendum.enactment_task = Some(enactment_task(index));
	}
	let changes_state = !DEPOSIT_REFUNDS.contains(&ev.method.as_str());
	if changes_state {
		referendum.state = ev.method.clone();
		referendum.track_info = track_info(ev, referendum.track).await?;
	}
	referendum.is_final = ongoing.is_none();
	referendum.updated_block = ev.position.block_number;
	referendum.save(ev.db).await?;

	save_event(ev, index, &ev.method, &ev.args_text(), changes_state).await?;
	if let Some(tally) = tally {
		let electorate = match referendum.electorate {
			Some(electorate) => electorate,
			None => ev.electorate().await?,
		};
		save_snapshot(ev, index, tally, electorate).await?;
	}
	Ok(())
}

/** Records the referendum's tally after votes on it changed in this block. */
pub(super) async fn refresh_tally(ev: &Context<'_>, index: i32) -> anyhow::Result<()> {
	let info = referendum_info(ev, index, ev.block).await?;
	let Some(ongoing) = info.as_ref().and_then(|info| info.get("ongoing")) else {
		return Ok(());
	};
	let (Some(tally), Some(mut referendum)) =
		(Tally::from_json(&ongoing["tally"]), Referendum::load(ev.db, index).await?)
	else {
		return Ok(());
	};
	let electorate = ev.electorate().await?;
	referendum.tally = tally;
	referendum.info = Some(ongoing.to_string());
	referendum.electorate = Some(electorate);
	referendum.updated_block = ev.position.block_number;
	referendum.save(ev.db).await?;
	save_snapshot(ev, index, tally, electorate).await
}

/** The scheduler's dispatch of a confirmed referendum's call, found by its task name. */
pub(super) async fn dispatched(ev: &Context<'_>) -> anyhow::Result<()> {
	let Some(task) = ev.arg("id").as_str() else { return Ok(()) };
	let row = ev
		.db
		.query_opt(
			"UPDATE governance.governance_referendums
			 SET state = 'Executed', enactment_result = $2, updated_block = $3
			 WHERE id = (SELECT id FROM governance.governance_referendums
				WHERE enactment_task = $1 LIMIT 1)
			 RETURNING index",
			&[&task, &ev.arg("result").to_string(), &ev.position.block_number],
		)
		.await?;
	let Some(row) = row else { return Ok(()) };
	let args = json!({ "result": ev.arg("result") }).to_string();
	save_event(ev, row.get(0), "Executed", &args, true).await
}

/** `Referenda::ReferendumInfoFor` as of `block`. */
async fn referendum_info(
	ev: &Context<'_>,
	index: i32,
	block: u64,
) -> anyhow::Result<Option<Value>> {
	let key = [key("Referenda", "ReferendumInfoFor"), blake2_128_concat(&(index as u32).encode())]
		.concat();
	ev.chain.storage_json(&key, block).await
}

/** The name pallet-referenda gives the scheduled enactment of a referendum, which the
 * scheduler's `Dispatched` event carries. */
fn enactment_task(index: i32) -> String {
	let name = blake2_256(&(*b"assembly", "enactment", index as u32).encode());
	format!("0x{}", hex::encode(name))
}

/** `Referenda::Tracks` in the runtime of this block, as `[id, info]` pairs. */
pub(super) async fn tracks(ev: &Context<'_>) -> anyhow::Result<Vec<(i32, Value)>> {
	let tracks = ev.chain.constant_json("Referenda", "Tracks", ev.block).await?;
	let pairs = tracks.as_ref().and_then(Value::as_array).cloned().unwrap_or_default();
	Ok(pairs
		.into_iter()
		.filter_map(|mut pair| Some((int(&pair[0])?, pair[1].take())))
		.collect())
}

/** Refreshes the track definitions, which only change with a runtime upgrade. */
async fn sync_tracks(ev: &Context<'_>) -> anyhow::Result<()> {
	for (id, info) in tracks(ev).await? {
		let name = info["name"].as_str().unwrap_or_default().trim_end_matches('\0');
		ev.db
			.execute(
				"INSERT INTO governance.governance_tracks (id, name, info, updated_block)
				 VALUES ($1, $2, $3, $4)
				 ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, info = EXCLUDED.info,
					updated_block = EXCLUDED.updated_block",
				&[&id.to_string(), &name, &info.to_string(), &ev.position.block_number],
			)
			.await?;
	}
	Ok(())
}

async fn track_info(ev: &Context<'_>, track: i32) -> anyhow::Result<Option<String>> {
	let tracks = tracks(ev).await?;
	Ok(tracks
		.into_iter()
		.find(|(id, _)| *id == track)
		.map(|(_, info)| info.to_string()))
}

fn proposal_hash(proposal: &Value) -> Option<String> {
	["lookup", "legacy"]
		.iter()
		.find_map(|kind| proposal[kind]["hash"].as_str())
		.map(String::from)
}

/** The proposed call, inline or from its preimage, if the preimage is noted yet. */
async fn proposal_call(ev: &Context<'_>, proposal: &Value) -> anyhow::Result<Option<String>> {
	let bytes = match (proposal["inline"].as_str(), proposal_hash(proposal)) {
		(Some(inline), _) => hex::decode(inline.trim_start_matches("0x")).ok(),
		(None, Some(hash)) => preimages::preimage(ev, &hash).await?.map(|(_, data)| data),
		(None, None) => None,
	};
	let Some(bytes) = bytes else { return Ok(None) };
	Ok(ev.chain.call_json(&bytes, ev.block).await?.map(|call| call.to_string()))
}

async fn save_event(
	ev: &Context<'_>,
	index: i32,
	name: &str,
	args: &str,
	changes_state: bool,
) -> anyhow::Result<()> {
	let p = &ev.position;
	ev.db
		.execute(
			"INSERT INTO governance.governance_referendum_events (id, referendum_id, name, args,
				block_number, block_hash, extrinsic_index, event_index, \"timestamp\",
				changes_state)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) ON CONFLICT (id) DO NOTHING",
			&[
				&p.id,
				&index.to_string(),
				&name,
				&args,
				&p.block_number,
				&p.block_hash,
				&p.extrinsic_index,
				&p.event_index,
				&p.timestamp,
				&changes_state,
			],
		)
		.await?;
	Ok(())
}

async fn save_snapshot(
	ev: &Context<'_>,
	index: i32,
	tally: Tally,
	electorate: u128,
) -> anyhow::Result<()> {
	let p = &ev.position;
	ev.db
		.execute(
			"INSERT INTO governance.governance_tally_snapshots (id, referendum_id, ayes, nays,
				support, electorate, block_number, \"timestamp\")
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
			 ON CONFLICT (id) DO UPDATE SET ayes = EXCLUDED.ayes, nays = EXCLUDED.nays,
				support = EXCLUDED.support, electorate = EXCLUDED.electorate",
			&[
				&format!("{index}-{}", p.block_number),
				&index.to_string(),
				&numeric(tally.ayes),
				&numeric(tally.nays),
				&numeric(tally.support),
				&numeric(electorate),
				&p.block_number,
				&p.timestamp,
			],
		)
		.await?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn enactment_tasks_are_named_as_pallet_referenda_names_them() {
		// The id of the scheduler Dispatched event that enacted Nexus referendum 15 at block
		// 8069993
		assert_eq!(
			enactment_task(15),
			"0x000d200ef3a006678d25617d594246894e0102c39c30b04d6e31eb34c6cef151"
		);
	}

	#[test]
	fn proposals_name_their_preimage() {
		let lookup = json!({ "lookup": { "hash": "0xab", "len": 5 } });
		assert_eq!(proposal_hash(&lookup).as_deref(), Some("0xab"));
		assert_eq!(
			proposal_hash(&json!({ "legacy": { "hash": "0xcd" } })).as_deref(),
			Some("0xcd")
		);
		assert_eq!(proposal_hash(&json!({ "inline": "0x0000" })), None);
	}
}
