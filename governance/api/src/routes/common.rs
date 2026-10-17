use serde_json::{json, Map, Value};
use tokio_postgres::{Client, Row};

use crate::{
	error::{ApiError, ApiResult},
	primitives::{author, iso, parse_post_id, post_id, Indexer, PostKind},
};

/// The forum post, first event, last event and comment columns every on-chain item query selects,
/// so their rows can be turned into forum posts the same way. `{events}` is the item's event
/// table and `{filter}` matches its rows to the item.
pub const POST_COLUMNS: &str = "
	s.block_number AS s_block, s.block_hash AS s_hash, s.extrinsic_index AS s_extrinsic,
	s.event_index AS s_event, s.ms AS s_ms,
	l.name AS l_name, l.args AS l_args, l.block_number AS l_block, l.block_hash AS l_hash,
	l.extrinsic_index AS l_extrinsic, l.event_index AS l_event, l.ms AS l_ms,
	p.title, p.content, p.content_type, p.author, p.labels, p.comment_filter,
	ms(p.updated_at) AS p_updated_ms,
	c.count AS comments_count, c.last_ms AS comment_ms";

pub fn post_joins(events: &str, filter: &str, kind: PostKind, number: &str) -> String {
	let event = |order: &str| {
		format!(
			"CROSS JOIN LATERAL (SELECT name, args, block_number, block_hash, extrinsic_index, event_index,
				ms(timestamp) AS ms
			FROM {events} WHERE {filter} ORDER BY block_number {order}, event_index {order} LIMIT 1)"
		)
	};
	let kind = kind as i16;
	format!(
		"{} s {} l
		LEFT JOIN posts p ON p.kind = {kind} AND p.number = {number}
		CROSS JOIN LATERAL (SELECT count(*) AS count, ms(max(created_at)) AS last_ms
			FROM comments WHERE post_kind = {kind} AND post_number = {number} AND NOT deleted) c",
		event("ASC"),
		event("DESC"),
	)
}

pub fn indexer(row: &Row, prefix: &str) -> Indexer {
	Indexer {
		block_height: row.get(format!("{prefix}_block").as_str()),
		block_hash: row.get(format!("{prefix}_hash").as_str()),
		block_time: row.get(format!("{prefix}_ms").as_str()),
		extrinsic_index: row.get(format!("{prefix}_extrinsic").as_str()),
		event_index: row.get(format!("{prefix}_event").as_str()),
	}
}

/// The fields the forum shows for any post, filled from the forum when someone has written one.
pub fn post_fields(row: &Row, kind: PostKind, number: i64, proposer: &str) -> Map<String, Value> {
	let created = row.get::<_, i64>("s_ms");
	let last_event = row.get::<_, i64>("l_ms");
	let last_comment = row.get::<_, Option<i64>>("comment_ms").unwrap_or_default();
	let writer = row.get::<_, Option<String>>("author").unwrap_or_else(|| proposer.into());
	let content_type = row
		.get::<_, Option<String>>("content_type")
		.unwrap_or_else(|| "markdown".into());

	let Value::Object(fields) = json!({
		"_id": post_id(kind, number),
		"indexer": indexer(row, "s"),
		"proposer": proposer,
		"author": author(&writer),
		"title": row.get::<_, Option<String>>("title"),
		"content": row.get::<_, Option<String>>("content"),
		"contentType": content_type,
		"labels": row.get::<_, Option<Value>>("labels").unwrap_or_else(|| json!([])),
		"createdAt": iso(created),
		"updatedAt": iso(row.get::<_, Option<i64>>("p_updated_ms").unwrap_or(last_event)),
		"lastActivityAt": iso(last_event.max(last_comment)),
		"commentsCount": row.get::<_, i64>("comments_count"),
		"commentFilterSetting": row.get::<_, Option<Value>>("comment_filter"),
		"refToPost": { "postId": post_id(kind, number), "postType": kind.post_type() },
	}) else {
		return Map::new();
	};
	fields
}

/// An item's events in order, as their name, args and position. `filter` picks the item's events
/// and takes its id as `$1`.
pub async fn timeline(
	db: &Client,
	events: &str,
	filter: &str,
	id: String,
) -> ApiResult<Vec<(String, Value, Indexer)>> {
	let rows = db
		.query(
			&format!(
				"SELECT name, args, block_number AS e_block, block_hash AS e_hash, extrinsic_index AS e_extrinsic,
					event_index AS e_event, ms(timestamp) AS e_ms
				FROM {events} WHERE {filter} ORDER BY block_number, event_index"
			),
			&[&id],
		)
		.await?;
	Ok(rows
		.iter()
		.map(|row| (row.get("name"), parse(row.get("args")), indexer(row, "e")))
		.collect())
}

/// An on-chain item's index from a detail route, which receives either the index itself or the post
/// id the frontend uses once a page has loaded.
pub fn item_index(id: &str, kind: PostKind, what: &str) -> ApiResult<i32> {
	match parse_post_id(id) {
		Some((found, number)) if found == kind =>
			i32::try_from(number).map_err(|_| ApiError::not_found(what)),
		_ => id.parse().map_err(|_| ApiError::not_found(what)),
	}
}

/// The item's last event, which the forum shows as its state.
pub fn last_event(row: &Row) -> Value {
	json!({
		"name": row.get::<_, String>("l_name"),
		"indexer": indexer(row, "l"),
		"args": parse(row.get("l_args")),
	})
}

/// A JSON value the indexer stored as text.
pub fn parse(text: Option<String>) -> Value {
	text.and_then(|text| serde_json::from_str(&text).ok()).unwrap_or(Value::Null)
}

/// polkadot.js encodes large integers as hex strings; the forum frontend expects decimal strings.
pub fn decimal(value: &Value) -> Value {
	match value {
		Value::String(hex) if hex.starts_with("0x") => u128::from_str_radix(&hex[2..], 16)
			.map(|n| n.to_string().into())
			.unwrap_or_else(|_| value.clone()),
		Value::Number(n) => n.to_string().into(),
		_ => value.clone(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn turns_chain_integers_into_decimal_strings() {
		assert_eq!(
			decimal(&json!("0x0000000000000000058d15e176280000")),
			json!("400000000000000000")
		);
		assert_eq!(decimal(&json!(42)), json!("42"));
		assert_eq!(decimal(&json!("not hex")), json!("not hex"));
	}
}
