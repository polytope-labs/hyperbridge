//! The GraphQL queries of the site's block, extrinsic, event and account pages. Statescan
//! answered them from the chain on every view; these read the same tables as the REST routes.
//! Each answer holds every field those queries select, and `__typename` for Apollo's cache.
//! `docs` stays null: the site prepends it to an event's arguments and fails when they are
//! null, as they are for arguments too large to keep.

use axum::{extract::State, Json};
use serde_json::{json, Value};

use super::{normalize, Api, ApiResult};

const EVENT: &str = "jsonb_build_object('__typename', 'Event', 'args', ev.args, 'docs', NULL,
	'indexer', jsonb_build_object('__typename', 'EventIndexer', 'blockHash', b.hash,
		'blockHeight', ev.height, 'blockTime', ev.time, 'eventIndex', ev.index,
		'extrinsicIndex', ev.extrinsic_index),
	'isExtrinsic', ev.extrinsic_index IS NOT NULL,
	'isExtrinsicResult', ev.section = 'system'
		AND ev.method IN ('ExtrinsicSuccess', 'ExtrinsicFailed'),
	'method', ev.method, 'section', ev.section)";

/** An extrinsic with its call and its events. Only the outer call is known, so `calls` is
 * empty, and the fields Statescan read from the signature are null. */
fn extrinsic() -> String {
	format!(
		"jsonb_build_object('__typename', 'Extrinsic',
			'call', jsonb_build_object('__typename', 'Call', 'args', e.args, 'callIndex', NULL,
				'method', e.method, 'section', e.section),
			'calls', '[]'::jsonb, 'callsCount', 0,
			'events', (SELECT coalesce(jsonb_agg({EVENT} ORDER BY ev.index), '[]')
				FROM explorer.events ev WHERE ev.height = e.height AND ev.extrinsic_index = e.index),
			'eventsCount', e.events_count, 'hash', e.hash,
			'indexer', jsonb_build_object('__typename', 'ExtrinsicIndexer', 'blockHash', b.hash,
				'blockHeight', e.height, 'blockTime', e.time, 'extrinsicIndex', e.index),
			'isSigned', e.signed, 'isSuccess', e.success, 'lifetime', NULL, 'nonce', NULL,
			'signature', NULL, 'signer', e.signer, 'tip', NULL, 'version', 4, 'error', NULL)"
	)
}

fn block() -> String {
	format!(
		"jsonb_build_object('__typename', 'Block', 'digest', NULL,
			'events', (SELECT coalesce(jsonb_agg({EVENT} ORDER BY ev.index), '[]')
				FROM explorer.events ev WHERE ev.height = b.height),
			'eventsCount', b.events_count,
			'extrinsics', (SELECT coalesce(jsonb_agg({} ORDER BY e.index), '[]')
				FROM explorer.extrinsics e WHERE e.height = b.height),
			'extrinsicsCount', b.extrinsics_count, 'extrinsicsRoot', b.extrinsics_root,
			'hash', b.hash, 'height', b.height, 'parentHash', b.parent_hash,
			'stateRoot', b.state_root, 'time', b.time, 'validator', NULL)",
		extrinsic()
	)
}

const ACCOUNT: &str = "jsonb_build_object('__typename', 'Account',
	'data', jsonb_build_object('__typename', 'AccountData', 'free', a.free::text,
		'reserved', a.reserved::text, 'feeFrozen', a.frozen::text, 'miscFrozen', a.frozen::text,
		'total', a.total::text,
		'transferrable', (a.free - greatest(a.frozen - a.reserved, 0))::text,
		'lockedBalance', a.frozen::text, 'lockedBreakdown', '[]'::jsonb,
		'reservedBreakdown', '[]'::jsonb),
	'detail', jsonb_build_object('__typename', 'AccountDetail', 'nonce', a.nonce,
		'consumers', a.consumers, 'providers', a.providers,
		'data', jsonb_build_object('__typename', 'AccountDetailData', 'free', a.free::text,
			'reserved', a.reserved::text, 'miscFrozen', a.frozen::text,
			'feeFrozen', a.frozen::text)))";

/** Answers a query by the field it asks for. A query for something else gets a GraphQL error,
 * which the site treats as missing data. */
pub(super) async fn handle(State(api): State<Api>, Json(request): Json<Value>) -> ApiResult {
	let query = request["query"].as_str().unwrap_or_default();
	let variables = &request["variables"];
	let int = |name: &str| variables[name].as_i64();

	let (field, value) = if query.contains("chainBlock") {
		let sql = format!("SELECT {} FROM explorer.blocks b", block());
		let value = match &variables["blockHeightOrHash"] {
			Value::String(hash) =>
				api.one(&format!("{sql} WHERE b.hash = $1"), &[&hash.to_lowercase()]).await?,
			height => match height.as_i64() {
				Some(height) => api.one(&format!("{sql} WHERE b.height = $1"), &[&height]).await?,
				None => None,
			},
		};
		("chainBlock", value)
	} else if query.contains("chainExtrinsic") {
		let sql = format!(
			"SELECT {} FROM explorer.extrinsics e JOIN explorer.blocks b ON b.height = e.height
			 WHERE e.height = $1 AND e.index = $2",
			extrinsic()
		);
		let (height, index) = (int("blockHeight"), int("extrinsicIndex").map(|i| i as i32));
		("chainExtrinsic", api.one(&sql, &[&height, &index]).await?)
	} else if query.contains("chainEvent") {
		let sql = format!(
			"SELECT {EVENT} FROM explorer.events ev JOIN explorer.blocks b ON b.height = ev.height
			 WHERE ev.height = $1 AND ev.index = $2"
		);
		let (height, index) = (int("blockHeight"), int("eventIndex").map(|i| i as i32));
		("chainEvent", api.one(&sql, &[&height, &index]).await?)
	} else if query.contains("chainAccount") {
		let address = variables["address"].as_str().and_then(normalize);
		let sql = format!("SELECT {ACCOUNT} FROM explorer.accounts a WHERE a.address = $1");
		("chainAccount", api.one(&sql, &[&address]).await?)
	} else if query.contains("accountForeignAssets") {
		let empty = json!({
			"__typename": "PagedForeignAssetHolder",
			"limit": variables["limit"],
			"offset": variables["offset"],
			"total": 0,
			"holders": [],
		});
		("accountForeignAssets", Some(empty))
	} else {
		return Ok(Json(json!({ "errors": [{ "message": "unsupported query" }] })));
	};
	Ok(Json(json!({ "data": { field: value } })))
}
