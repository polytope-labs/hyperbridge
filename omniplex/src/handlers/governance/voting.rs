//! Votes and delegations. Nexus vote events carry no referendum index, so each voting event
//! re-reads the accounts it names from `ConvictionVoting::VotingFor`. Only ongoing referenda are
//! touched, so votes on finished ones keep their final state.

use std::collections::{BTreeMap, BTreeSet};

use codec::Encode;
use serde_json::Value;
use tokio_postgres::Row;

use super::{amount, int, key, referenda, twox_64_concat, Context};
use crate::{
	store::{numeric, Numeric},
	substrate::account_id,
};

pub(super) const METHODS: &[&str] = &["Voted", "VoteRemoved", "Delegated", "Undelegated"];

/** `Conviction` in the order of its variants, so a delegation stores 0 for `None` up to 6 for
 * `Locked6x`. */
const CONVICTIONS: &[&str] =
	&["None", "Locked1x", "Locked2x", "Locked3x", "Locked4x", "Locked5x", "Locked6x"];

pub(super) async fn handle(ev: &Context<'_>) -> anyhow::Result<()> {
	let account = |at: usize| ev.values.get(at).and_then(Value::as_str).map(String::from);
	let Some(who) = account(0) else { return Ok(()) };
	let mut accounts = BTreeSet::from([who.clone()]);
	match ev.method.as_str() {
		"Delegated" => accounts.extend(account(1)),
		// Undelegated carries no target, so the delegatees are those of the delegations it ends
		"Undelegated" => accounts
			.extend(active_delegations(ev, &who).await?.into_iter().map(|(_, (_, d))| d.target)),
		_ => {},
	}
	for account in accounts {
		sync(ev, &account).await?;
	}
	Ok(())
}

/** How an account voted on one referendum. */
#[derive(PartialEq)]
struct Choice {
	kind: &'static str,
	aye: Option<bool>,
	conviction: Option<i32>,
	balance: u128,
	aye_balance: Option<u128>,
	nay_balance: Option<u128>,
	abstain_balance: Option<u128>,
}

impl Choice {
	/** An `AccountVote`. A standard vote packs its direction into the top bit of a byte and
	 * its conviction into the rest. */
	fn from_json(vote: &Value) -> Option<Self> {
		let split = |kind, fields: &Value| {
			let (aye, nay) = (amount(&fields["aye"])?, amount(&fields["nay"])?);
			let abstain = (kind == "SplitAbstain").then(|| amount(&fields["abstain"])).flatten();
			Some(Self {
				kind,
				aye: None,
				conviction: None,
				balance: aye + nay + abstain.unwrap_or_default(),
				aye_balance: Some(aye),
				nay_balance: Some(nay),
				abstain_balance: abstain,
			})
		};
		if let Some(standard) = vote.get("standard") {
			let byte = amount(&standard["vote"])? as u8;
			return Some(Self {
				kind: "Standard",
				aye: Some(byte & 0x80 != 0),
				conviction: Some((byte & 0x7f) as i32),
				balance: amount(&standard["balance"])?,
				aye_balance: None,
				nay_balance: None,
				abstain_balance: None,
			});
		}
		match (vote.get("split"), vote.get("splitAbstain")) {
			(Some(fields), _) => split("Split", fields),
			(_, Some(fields)) => split("SplitAbstain", fields),
			_ => None,
		}
	}

	fn from_row(row: &Row) -> anyhow::Result<Self> {
		let kind = match row.get::<_, &str>("kind") {
			"Standard" => "Standard",
			"Split" => "Split",
			_ => "SplitAbstain",
		};
		Ok(Self {
			kind,
			aye: row.get("aye"),
			conviction: row.get("conviction"),
			balance: big(row, "balance")?.unwrap_or_default(),
			aye_balance: big(row, "aye_balance")?,
			nay_balance: big(row, "nay_balance")?,
			abstain_balance: big(row, "abstain_balance")?,
		})
	}
}

/** An account's casting on one track: its votes by referendum, and what is delegated to it. */
struct Casting {
	track: i32,
	votes: Vec<(i32, Value)>,
	delegated_votes: u128,
	delegated_capital: u128,
}

#[derive(PartialEq)]
struct Delegation {
	target: String,
	conviction: i32,
	balance: u128,
}

/**
 * Brings an account's votes and delegations in line with `ConvictionVoting::VotingFor` at this
 * block, then records the tally of every referendum whose votes changed.
 */
async fn sync(ev: &Context<'_>, account: &str) -> anyhow::Result<()> {
	let Some(id) = account_id(account) else { return Ok(()) };
	let mut castings = Vec::new();
	let mut delegating = BTreeMap::new();
	for (track, _) in referenda::tracks(ev).await? {
		let key = [
			key("ConvictionVoting", "VotingFor"),
			twox_64_concat(&id),
			twox_64_concat(&(track as u16).encode()),
		]
		.concat();
		let Some(voting) = ev.chain.storage_json(&key, ev.block).await? else { continue };
		if let Some(casting) = voting.get("casting") {
			let votes = casting["votes"].as_array().cloned().unwrap_or_default();
			castings.push(Casting {
				track,
				votes: votes
					.into_iter()
					.filter_map(|mut pair| Some((int(&pair[0])?, pair[1].take())))
					.collect(),
				delegated_votes: amount(&casting["delegations"]["votes"]).unwrap_or_default(),
				delegated_capital: amount(&casting["delegations"]["capital"]).unwrap_or_default(),
			});
		}
		if let Some(delegation) = voting.get("delegating") {
			let conviction = delegation["conviction"].as_str().unwrap_or_default();
			delegating.insert(
				track,
				Delegation {
					target: delegation["target"].as_str().unwrap_or_default().to_string(),
					conviction: CONVICTIONS.iter().position(|c| *c == conviction).unwrap_or(0)
						as i32,
					balance: amount(&delegation["balance"]).unwrap_or_default(),
				},
			);
		}
	}
	sync_delegations(ev, account, &delegating).await?;

	let current: BTreeMap<i32, (&Casting, &Value)> = castings
		.iter()
		.flat_map(|casting| casting.votes.iter().map(move |(poll, vote)| (*poll, (casting, vote))))
		.collect();
	let ongoing = ev
		.db
		.query("SELECT index FROM governance.governance_referendums WHERE NOT is_final", &[])
		.await?;
	let mut changed = Vec::new();
	for row in ongoing {
		let index: i32 = row.get(0);
		let id = format!("{index}-{account}");
		let existing = ev
			.db
			.query_opt("SELECT * FROM governance.governance_votes WHERE id = $1", &[&id])
			.await?;
		let Some((casting, vote)) = current.get(&index) else {
			if existing.is_some() {
				ev.db
					.execute("DELETE FROM governance.governance_votes WHERE id = $1", &[&id])
					.await?;
				save_vote_call(ev, account, index, None).await?;
				changed.push(index);
			}
			continue;
		};
		let Some(choice) = Choice::from_json(vote) else { continue };
		let (vote_changed, delegated_changed) = match &existing {
			Some(row) => (
				Choice::from_row(row)? != choice,
				big(row, "delegated_votes")? != Some(casting.delegated_votes),
			),
			None => (true, true),
		};
		if !vote_changed && !delegated_changed {
			continue;
		}
		ev.db
			.execute(
				"INSERT INTO governance.governance_votes (id, referendum_index, track, account,
					kind, aye, conviction, balance, aye_balance, nay_balance, abstain_balance,
					delegated_votes, delegated_capital, block_number)
				 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
				 ON CONFLICT (id) DO UPDATE SET track = EXCLUDED.track, kind = EXCLUDED.kind,
					aye = EXCLUDED.aye, conviction = EXCLUDED.conviction,
					balance = EXCLUDED.balance, aye_balance = EXCLUDED.aye_balance,
					nay_balance = EXCLUDED.nay_balance,
					abstain_balance = EXCLUDED.abstain_balance,
					delegated_votes = EXCLUDED.delegated_votes,
					delegated_capital = EXCLUDED.delegated_capital,
					block_number = EXCLUDED.block_number",
				&[
					&id,
					&index,
					&casting.track,
					&account,
					&choice.kind,
					&choice.aye,
					&choice.conviction,
					&numeric(choice.balance),
					&choice.aye_balance.map(numeric),
					&choice.nay_balance.map(numeric),
					&choice.abstain_balance.map(numeric),
					&numeric(casting.delegated_votes),
					&numeric(casting.delegated_capital),
					&ev.position.block_number,
				],
			)
			.await?;
		if vote_changed {
			save_vote_call(ev, account, index, Some(vote)).await?;
		}
		changed.push(index);
	}

	for index in changed {
		referenda::refresh_tally(ev, index).await?;
	}
	Ok(())
}

/** The account's delegations in force, with their ids. */
async fn active_delegations(
	ev: &Context<'_>,
	account: &str,
) -> anyhow::Result<Vec<(String, (i32, Delegation))>> {
	let rows = ev
		.db
		.query(
			"SELECT id, track, target, conviction, balance FROM governance.governance_delegations
			 WHERE delegator = $1 AND active",
			&[&account],
		)
		.await?;
	rows.iter()
		.map(|row| {
			let delegation = Delegation {
				target: row.get("target"),
				conviction: row.get("conviction"),
				balance: big(row, "balance")?.unwrap_or_default(),
			};
			Ok((row.get("id"), (row.get("track"), delegation)))
		})
		.collect()
}

/** Ends the delegations that changed, and starts the ones that are new or changed. */
async fn sync_delegations(
	ev: &Context<'_>,
	account: &str,
	delegating: &BTreeMap<i32, Delegation>,
) -> anyhow::Result<()> {
	let mut unchanged = BTreeSet::new();
	for (id, (track, delegation)) in active_delegations(ev, account).await? {
		if delegating.get(&track) == Some(&delegation) {
			unchanged.insert(track);
			continue;
		}
		ev.db
			.execute(
				"UPDATE governance.governance_delegations SET active = false, to_block = $2
				 WHERE id = $1",
				&[&id, &ev.position.block_number],
			)
			.await?;
	}
	for (track, delegation) in delegating.iter().filter(|(track, _)| !unchanged.contains(track)) {
		let block = ev.position.block_number;
		ev.db
			.execute(
				"INSERT INTO governance.governance_delegations (id, delegator, target, track,
					conviction, balance, active, from_block)
				 VALUES ($1, $2, $3, $4, $5, $6, true, $7)
				 ON CONFLICT (id) DO UPDATE SET target = EXCLUDED.target,
					conviction = EXCLUDED.conviction, balance = EXCLUDED.balance,
					active = true, to_block = NULL",
				&[
					&format!("{account}-{track}-{block}"),
					&account,
					&delegation.target,
					track,
					&delegation.conviction,
					&numeric(delegation.balance),
					&block,
				],
			)
			.await?;
	}
	Ok(())
}

async fn save_vote_call(
	ev: &Context<'_>,
	account: &str,
	index: i32,
	vote: Option<&Value>,
) -> anyhow::Result<()> {
	let p = &ev.position;
	ev.db
		.execute(
			"INSERT INTO governance.governance_vote_calls (id, account, referendum_index, vote,
				removed, block_number, block_hash, extrinsic_index, event_index, \"timestamp\")
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) ON CONFLICT (id) DO NOTHING",
			&[
				&format!("{}-{index}", p.id),
				&account,
				&index,
				&vote.map(Value::to_string),
				&vote.is_none(),
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

fn big(row: &Row, column: &str) -> anyhow::Result<Option<u128>> {
	let value = row.get::<_, Option<Numeric>>(column);
	Ok(value.map(|value| value.as_str().parse()).transpose()?)
}

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;

	#[test]
	fn votes_read_as_conviction_voting_stores_them() {
		let aye = Choice::from_json(&json!({ "standard": { "vote": 0x82, "balance": "70" } }));
		let aye = aye.unwrap();
		assert_eq!(
			(aye.kind, aye.aye, aye.conviction, aye.balance),
			("Standard", Some(true), Some(2), 70)
		);
		let nay = Choice::from_json(&json!({ "standard": { "vote": 1, "balance": 5 } })).unwrap();
		assert_eq!((nay.aye, nay.conviction), (Some(false), Some(1)));
		let split =
			Choice::from_json(&json!({ "splitAbstain": { "aye": 1, "nay": 2, "abstain": 3 } }));
		let split = split.unwrap();
		assert_eq!(
			(split.kind, split.balance, split.abstain_balance),
			("SplitAbstain", 6, Some(3))
		);
		assert!(Choice::from_json(&json!({ "other": {} })).is_none());
	}
}
