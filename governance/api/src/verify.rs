use serde_json::Value;

use crate::subsquare::SubSquareApi;

/// Fields of a referendum that come from the chain. Off-chain fields such as titles are left to the
/// import, and the timeline is compared event by event.
const REFERENDUM_FIELDS: &[&str] = &[
	"referendumIndex",
	"indexer",
	"proposer",
	"track",
	"trackInfo",
	"state.name",
	"state.indexer",
	"state.args",
	"isFinal",
	"onchainData.proposal.call",
	"onchainData.proposalHash",
	"onchainData.info",
	"onchainData.tally.ayes",
	"onchainData.tally.nays",
	"onchainData.tally.support",
];

const MOTION_FIELDS: &[&str] = &[
	"motionIndex",
	"hash",
	"height",
	"proposer",
	"state",
	"indexer",
	"onchainData.threshold",
	"onchainData.proposal",
	"onchainData.voting",
	"onchainData.tally",
	"onchainData.state.state",
];

#[derive(Default)]
struct Report {
	checked: usize,
	missing: Vec<String>,
	differences: Vec<String>,
}

impl Report {
	fn compare(&mut self, item: &str, field: &str, ours: &Value, theirs: &Value) {
		self.checked += 1;
		let (ours, theirs) = (at(ours, field), at(theirs, field));
		if !matches(&normalized(&ours), &normalized(&theirs)) {
			self.differences
				.push(format!("{item} {field}\n    ours:   {ours}\n    theirs: {theirs}"));
		}
	}
}

/// A nested field by its dotted path, where a number picks an array element.
fn at(value: &Value, path: &str) -> Value {
	path.split('.')
		.try_fold(value, |value, key| match key.parse::<usize>() {
			Ok(index) if value.is_array() => value.get(index),
			_ => value.get(key),
		})
		.cloned()
		.unwrap_or(Value::Null)
}

/// Whether ours has every value theirs has. Ours may carry extra fields, the frontend ignores them.
fn matches(ours: &Value, theirs: &Value) -> bool {
	match (ours, theirs) {
		(Value::Object(ours), Value::Object(theirs)) => theirs
			.iter()
			.all(|(key, value)| matches(ours.get(key).unwrap_or(&Value::Null), value)),
		(Value::Array(ours), Value::Array(theirs)) =>
			ours.len() == theirs.len() &&
				ours.iter().zip(theirs).all(|(ours, theirs)| matches(ours, theirs)),
		_ => ours == theirs,
	}
}

/// Amounts appear as numbers, decimal strings or hex strings depending on the source, so they are
/// compared as decimal strings.
fn normalized(value: &Value) -> Value {
	match value {
		Value::String(hex) if hex.starts_with("0x") && hex.len() <= 34 =>
			u128::from_str_radix(&hex[2..], 16)
				.map(|n| n.to_string().into())
				.unwrap_or(value.clone()),
		Value::String(text) if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) =>
			text.trim_start_matches('0').to_owned().into(),
		Value::Number(n) if n.is_u64() => n.to_string().trim_start_matches('0').to_owned().into(),
		Value::Array(items) => items.iter().map(normalized).collect(),
		Value::Object(fields) => fields.iter().map(|(k, v)| (k.clone(), normalized(v))).collect(),
		_ => value.clone(),
	}
}

/// Compares what this API serves with a SubSquare site for every referendum and motion the site
/// knows, and fails if any on-chain field differs.
pub async fn run(api: &str, site: &str) -> anyhow::Result<()> {
	let (ours, theirs) = (SubSquareApi::new(api), SubSquareApi::site(site));
	let mut report = Report::default();

	for item in theirs.all("gov2/referendums").await? {
		let index = item["referendumIndex"].as_i64().unwrap_or_default();
		let name = format!("referendum {index}");
		let Some(our) = ours.find(&format!("gov2/referendums/{index}")).await? else {
			report.missing.push(name);
			continue;
		};
		let their = theirs.get(&format!("gov2/referendums/{index}")).await?;
		for field in REFERENDUM_FIELDS {
			report.compare(&name, field, &our, &their);
		}
		compare_timeline(&mut report, &name, "name", &our, &their);

		let (our_stats, their_stats) = (
			ours.get(&format!("gov2/referendums/{index}/vote-stats")).await?,
			theirs.get(&format!("gov2/referendums/{index}/vote-stats")).await?,
		);
		for field in their_stats
			.as_object()
			.into_iter()
			.flat_map(|f| f.keys())
			.filter(|k| *k != "_id")
		{
			report.compare(
				&name,
				&format!("vote stats {field}"),
				&our_stats[field],
				&their_stats[field],
			);
		}

		let our_votes = ours.get(&format!("gov2/referenda/{index}/votes")).await?;
		let their_votes = theirs.get(&format!("gov2/referenda/{index}/votes")).await?;
		for vote in their_votes.as_array().into_iter().flatten() {
			let account = vote["account"].as_str().unwrap_or_default();
			let our_vote = our_votes
				.as_array()
				.and_then(|votes| votes.iter().find(|v| v["account"] == vote["account"]))
				.cloned()
				.unwrap_or(Value::Null);
			for field in vote.as_object().into_iter().flat_map(|f| f.keys()) {
				report.compare(&format!("{name} vote of {account}"), field, &our_vote, vote);
			}
		}
	}

	for item in theirs.all("tech-comm/motions").await? {
		let index = item["motionIndex"].as_i64().unwrap_or_default();
		let name = format!("motion {index}");
		let Some(our) = ours.find(&format!("tech-comm/motions/{index}")).await? else {
			report.missing.push(name);
			continue;
		};
		let their = theirs
			.get(&format!("tech-comm/motions/{}", item["_id"].as_str().unwrap_or_default()))
			.await?;
		for field in MOTION_FIELDS {
			report.compare(&name, field, &our, &their);
		}
		compare_timeline(&mut report, &name, "method", &our, &their);
	}

	report.differences.iter().for_each(|difference| println!("{difference}"));
	println!(
		"{} fields compared, {} differ; {} items not indexed yet{}",
		report.checked,
		report.differences.len(),
		report.missing.len(),
		if report.missing.is_empty() {
			String::new()
		} else {
			format!(": {}", report.missing.join(", "))
		}
	);
	anyhow::ensure!(report.differences.is_empty(), "the API differs from {site}");
	Ok(())
}

fn compare_timeline(
	report: &mut Report,
	item: &str,
	name_field: &str,
	ours: &Value,
	theirs: &Value,
) {
	let events = theirs["onchainData"]["timeline"].as_array().map(Vec::len).unwrap_or_default();
	for (index, field) in (0..events).flat_map(|i| [name_field, "indexer", "args"].map(|f| (i, f)))
	{
		report.compare(item, &format!("onchainData.timeline.{index}.{field}"), ours, theirs);
	}
}
