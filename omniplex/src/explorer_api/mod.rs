//! The API the Statescan site reads, served from the `explorer` tables: the REST routes it calls
//! for Hyperbridge, in the shapes Statescan's server gave them, and the GraphQL queries of its
//! block, extrinsic, event and account pages. Postgres builds each item's JSON.

mod graphql;

use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;
use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::{IntoResponse, Response},
	routing::{get, post},
	Json, Router,
};
use rindexer::PostgresClient;
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tokio_postgres::types::ToSql;
use tower_http::cors::CorsLayer;

use crate::substrate::{account_id, encode_address};

/** The site polls the overview every 12 seconds; the server refreshes it twice as often. */
const OVERVIEW_REFRESH: Duration = Duration::from_secs(6);

const MAX_PAGE_SIZE: i64 = 100;

/** Inherents in every block, which Statescan left out of its lists. */
const LISTED_EXTRINSICS: &str = "NOT (e.section = 'timestamp' AND e.method = 'set')
	AND NOT (e.section = 'parachainSystem' AND e.method = 'setValidationData')";

/** Every extrinsic's own result event, which Statescan left out of its lists. */
const LISTED_EVENTS: &str = "NOT (ev.section = 'system'
	AND ev.method IN ('ExtrinsicSuccess', 'ExtrinsicFailed'))";

const BLOCK: &str = "jsonb_build_object('height', b.height, 'hash', b.hash, 'time', b.time,
	'validator', NULL, 'parentHash', b.parent_hash, 'stateRoot', b.state_root,
	'extrinsicsRoot', b.extrinsics_root, 'eventsCount', b.events_count,
	'extrinsicsCount', b.extrinsics_count, 'isFinalized', true)";

const EXTRINSIC: &str = "jsonb_build_object('indexer', jsonb_build_object('blockHeight', e.height,
	'blockHash', b.hash, 'blockTime', e.time, 'extrinsicIndex', e.index), 'hash', e.hash,
	'isSuccess', e.success, 'isSigned', e.signed, 'signer', e.signer, 'section', e.section,
	'method', e.method, 'args', e.args, 'eventsCount', e.events_count, 'callsCount', 0,
	'isFinalized', true)";

const EVENT: &str = "jsonb_build_object('indexer', jsonb_build_object('blockHeight', ev.height,
	'blockHash', b.hash, 'blockTime', ev.time, 'eventIndex', ev.index,
	'extrinsicIndex', ev.extrinsic_index), 'isExtrinsic', ev.extrinsic_index IS NOT NULL,
	'section', ev.section, 'method', ev.method, 'args', ev.args)";

const TRANSFER: &str = "jsonb_build_object('indexer', jsonb_build_object('blockHeight', t.height,
	'blockHash', b.hash, 'blockTime', t.time, 'eventIndex', t.event_index,
	'extrinsicIndex', t.extrinsic_index), 'from', t.\"from\", 'to', t.\"to\",
	'balance', t.amount::text, 'isSigned', coalesce(e.signed, false), 'assetModule', 0,
	'isNativeAsset', true, 'isFinalized', true)";

const TRANSFERS_FROM: &str = "explorer.transfers t JOIN explorer.blocks b ON b.height = t.height
	LEFT JOIN explorer.extrinsics e ON e.height = t.height AND e.index = t.extrinsic_index";

const ACCOUNT: &str = "jsonb_build_object('address', a.address, 'data', jsonb_build_object(
	'total', a.total::text, 'free', a.free::text, 'reserved', a.reserved::text,
	'miscFrozen', a.frozen::text, 'feeFrozen', a.frozen::text), 'detail', jsonb_build_object(
	'nonce', a.nonce, 'consumers', a.consumers, 'providers', a.providers,
	'sufficients', a.sufficients, 'data', jsonb_build_object('free', a.free::text,
	'reserved', a.reserved::text, 'frozen', a.frozen::text, 'flags', '0')))";

#[derive(Clone)]
struct Api {
	db: Arc<PostgresClient>,
	overview: Arc<RwLock<Value>>,
}

/** Serves the API on `listen` until shutdown. */
pub async fn serve(listen: SocketAddr) -> anyhow::Result<()> {
	let db = Arc::new(PostgresClient::new().await.context("connecting to DATABASE_URL")?);
	let api = Api { db, overview: Arc::new(RwLock::new(json!({}))) };
	tokio::spawn(refresh_overview(api.clone()));

	let router = Router::new()
		.route("/overview", get(overview))
		.route("/latest-blocks", get(latest_blocks))
		.route("/latest-transfers", get(latest_transfers))
		.route("/search/hints", get(search_hints))
		.route("/blocks", get(blocks))
		.route("/extrinsics", get(extrinsics))
		.route("/extrinsics/{id}", get(extrinsic))
		.route("/events", get(events))
		.route("/calls", get(calls))
		.route("/calls/{id}", get(call))
		.route("/transfers", get(transfers))
		.route("/accounts", get(accounts))
		.route("/accounts/{address}/summary", get(account_summary))
		.route("/accounts/{address}/extrinsics", get(account_extrinsics))
		.route("/accounts/{address}/transfers", get(account_transfers))
		.route("/specs/filter", get(spec_filter))
		.route("/graphql", post(graphql::handle))
		.layer(CorsLayer::permissive())
		.with_state(api);

	let listener = tokio::net::TcpListener::bind(listen).await?;
	tracing::info!(%listen, "explorer API listening");
	axum::serve(listener, router).await?;
	Ok(())
}

type Params = Query<HashMap<String, String>>;
type ApiResult = Result<Json<Value>, ApiError>;

/** An error as Statescan's server sent it: the status, with a message for a 404. */
enum ApiError {
	NotFound(&'static str),
	Internal(anyhow::Error),
}

impl IntoResponse for ApiError {
	fn into_response(self) -> Response {
		match self {
			ApiError::NotFound(message) =>
				(StatusCode::NOT_FOUND, Json(json!({ "message": message }))).into_response(),
			ApiError::Internal(error) => {
				tracing::warn!("explorer API: {error:#}");
				StatusCode::INTERNAL_SERVER_ERROR.into_response()
			},
		}
	}
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
	fn from(error: E) -> Self {
		ApiError::Internal(error.into())
	}
}

impl Api {
	/** The JSON in the first column of each row. */
	async fn rows(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> anyhow::Result<Vec<Value>> {
		let conn = self.db.raw_connection().await?;
		Ok(conn.query(sql, params).await?.iter().map(|row| row.get(0)).collect())
	}

	async fn one(
		&self,
		sql: &str,
		params: &[&(dyn ToSql + Sync)],
	) -> anyhow::Result<Option<Value>> {
		Ok(self.rows(sql, params).await?.into_iter().next())
	}

	async fn count(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> anyhow::Result<i64> {
		let conn = self.db.raw_connection().await?;
		Ok(conn.query_one(sql, params).await?.get(0))
	}
}

/** A page of a list: `page` from 0 and `page_size`, as the site sends them. */
struct Page {
	page: i64,
	size: i64,
}

impl Page {
	fn from(params: &HashMap<String, String>) -> Self {
		let number = |key: &str| params.get(key).and_then(|v| v.parse::<i64>().ok());
		let size = number("page_size").unwrap_or(10).clamp(1, MAX_PAGE_SIZE);
		Self { page: number("page").unwrap_or(0).max(0), size }
	}

	fn sql(&self) -> String {
		format!("LIMIT {} OFFSET {}", self.size, self.page * self.size)
	}

	fn body(&self, items: Vec<Value>, total: Option<i64>) -> Json<Value> {
		let mut body = json!({ "items": items, "page": self.page, "pageSize": self.size });
		if let Some(total) = total {
			body["total"] = json!(total);
		}
		Json(body)
	}
}

/** `WHERE` clauses and the values they bind, numbered as they are added. */
struct Filter {
	clauses: Vec<String>,
	values: Vec<Box<dyn ToSql + Sync + Send>>,
}

impl Filter {
	fn new(fixed: &str) -> Self {
		Self { clauses: vec![fixed.to_string()], values: vec![] }
	}

	/** Adds `clause`, where `$` stands for the value's placeholder. */
	fn add(&mut self, clause: &str, value: impl ToSql + Sync + Send + 'static) {
		self.values.push(Box::new(value));
		self.clauses.push(clause.replace('$', &format!("${}", self.values.len())));
	}

	fn sql(&self) -> String {
		format!("WHERE {}", self.clauses.join(" AND "))
	}

	fn params(&self) -> Vec<&(dyn ToSql + Sync)> {
		self.values.iter().map(|v| v.as_ref() as &(dyn ToSql + Sync)).collect()
	}

	/** The block or date range of the site's time filter, over `table`'s height and time. */
	fn time_range(&mut self, params: &HashMap<String, String>, table: &str) {
		let number = |key: &str| params.get(key).and_then(|v| v.parse::<i64>().ok());
		let (column, start, end) = match params.get("time_dimension").map(String::as_str) {
			Some("date") => ("time", number("date_start"), number("date_end")),
			_ => ("height", number("block_start"), number("block_end")),
		};
		if let Some(start) = start {
			self.add(&format!("{table}.{column} >= $"), start);
		}
		if let Some(end) = end {
			self.add(&format!("{table}.{column} <= $"), end);
		}
	}
}

/** An address in any SS58 prefix, as the prefix 0 form the tables hold. */
fn normalize(address: &str) -> Option<String> {
	encode_address(&account_id(address)?, 0)
}

async fn refresh_overview(api: Api) {
	let mut interval = tokio::time::interval(OVERVIEW_REFRESH);
	loop {
		interval.tick().await;
		match read_overview(&api).await {
			Ok(overview) => *api.overview.write().await = overview,
			Err(error) => tracing::warn!("explorer API overview: {error:#}"),
		}
	}
}

async fn read_overview(api: &Api) -> anyhow::Result<Value> {
	let overview = api
		.one(
			"SELECT jsonb_build_object(
				'latestHeight', (SELECT coalesce(max(height), 0) FROM explorer.blocks),
				'finalizedHeight', (SELECT coalesce(max(height), 0) FROM explorer.blocks),
				'totalIssuance', (SELECT value FROM explorer.status WHERE name = 'totalIssuance'),
				'signedExtrinsics', (SELECT count(*) FROM explorer.extrinsics WHERE signed),
				'transfers', (SELECT CASE WHEN reltuples < 0
					THEN (SELECT count(*) FROM explorer.transfers) ELSE reltuples::bigint END
					FROM pg_class WHERE oid = 'explorer.transfers'::regclass),
				'accounts', (SELECT count(*) FROM explorer.accounts))",
			&[],
		)
		.await?;
	Ok(overview.unwrap_or_default())
}

async fn overview(State(api): State<Api>) -> Json<Value> {
	Json(api.overview.read().await.clone())
}

async fn latest_blocks(State(api): State<Api>) -> ApiResult {
	let sql = format!("SELECT {BLOCK} FROM explorer.blocks b ORDER BY b.height DESC LIMIT 5");
	Ok(Json(Value::Array(api.rows(&sql, &[]).await?)))
}

async fn latest_transfers(State(api): State<Api>) -> ApiResult {
	let sql = format!(
		"SELECT {TRANSFER} FROM {TRANSFERS_FROM} WHERE e.signed
		 ORDER BY t.height DESC, t.event_index LIMIT 5"
	);
	Ok(Json(Value::Array(api.rows(&sql, &[]).await?)))
}

async fn search_hints(State(api): State<Api>, Query(params): Params) -> ApiResult {
	let term = params.get("term").map(|t| t.trim()).unwrap_or_default();
	if let Ok(height) = term.parse::<i64>() {
		let exists = api
			.count("SELECT count(*) FROM explorer.blocks WHERE height = $1", &[&height])
			.await?;
		return Ok(Json(if exists > 0 { json!({ "block": height }) } else { json!({}) }));
	}
	let hash = term.to_lowercase();
	if hash.len() == 66 && hash.starts_with("0x") {
		let block = "SELECT count(*) FROM explorer.blocks WHERE hash = $1";
		if api.count(block, &[&hash]).await? > 0 {
			return Ok(Json(json!({ "block": hash })));
		}
		let extrinsic = "SELECT count(*) FROM explorer.extrinsics WHERE hash = $1";
		if api.count(extrinsic, &[&hash]).await? > 0 {
			return Ok(Json(json!({ "extrinsic": hash })));
		}
	}
	Ok(Json(normalize(term).map_or(json!({}), |account| json!({ "account": account }))))
}

async fn blocks(State(api): State<Api>, Query(params): Params) -> ApiResult {
	let page = Page::from(&params);
	let sql =
		format!("SELECT {BLOCK} FROM explorer.blocks b ORDER BY b.height DESC {}", page.sql());
	let total = api.count("SELECT coalesce(max(height), 0) FROM explorer.blocks", &[]).await?;
	Ok(page.body(api.rows(&sql, &[]).await?, Some(total)))
}

async fn extrinsics(State(api): State<Api>, Query(params): Params) -> ApiResult {
	let page = Page::from(&params);
	let mut filter = Filter::new(LISTED_EXTRINSICS);
	if let Some(section) = params.get("section") {
		filter.add("e.section = $", section.clone());
	}
	if let Some(method) = params.get("method") {
		filter.add("e.method = $", method.clone());
	}
	if params.get("signed_only").is_some_and(|v| v == "true") {
		filter.clauses.push("e.signed".into());
	}
	filter.time_range(&params, "e");
	let sql = format!(
		"SELECT {EXTRINSIC} FROM explorer.extrinsics e JOIN explorer.blocks b ON b.height = e.height
		 {} ORDER BY e.height DESC, e.index {}",
		filter.sql(),
		page.sql()
	);
	Ok(page.body(api.rows(&sql, &filter.params()).await?, None))
}

async fn extrinsic(State(api): State<Api>, Path(id): Path<String>) -> ApiResult {
	let from = "explorer.extrinsics e JOIN explorer.blocks b ON b.height = e.height";
	let found = match id.split_once('-').map(|(h, i)| (h.parse::<i64>(), i.parse::<i32>())) {
		Some((Ok(height), Ok(index))) => {
			let sql =
				format!("SELECT {EXTRINSIC} FROM {from} WHERE e.height = $1 AND e.index = $2");
			api.one(&sql, &[&height, &index]).await?
		},
		_ => {
			let sql = format!("SELECT {EXTRINSIC} FROM {from} WHERE e.hash = $1 LIMIT 1");
			api.one(&sql, &[&id.to_lowercase()]).await?
		},
	};
	found.map(Json).ok_or(ApiError::NotFound("extrinsic not found"))
}

async fn events(State(api): State<Api>, Query(params): Params) -> ApiResult {
	let page = Page::from(&params);
	let mut filter = Filter::new(LISTED_EVENTS);
	if let Some(section) = params.get("section") {
		filter.add("ev.section = $", section.clone());
	}
	if let Some(method) = params.get("method") {
		filter.add("ev.method = $", method.clone());
	}
	if params.get("is_extrinsic").is_some_and(|v| v == "true") {
		filter.clauses.push("ev.extrinsic_index IS NOT NULL".into());
	}
	filter.time_range(&params, "ev");
	let sql = format!(
		"SELECT {EVENT} FROM explorer.events ev JOIN explorer.blocks b ON b.height = ev.height
		 {} ORDER BY ev.height DESC, ev.index {}",
		filter.sql(),
		page.sql()
	);
	Ok(page.body(api.rows(&sql, &filter.params()).await?, None))
}

/** Statescan keeps no calls for Hyperbridge, so its call pages are always empty. */
async fn calls(Query(params): Params) -> Json<Value> {
	Page::from(&params).body(vec![], None)
}

async fn call() -> ApiError {
	ApiError::NotFound("call not found")
}

async fn transfers(State(api): State<Api>, Query(params): Params) -> ApiResult {
	let page = Page::from(&params);
	let mut filter = Filter::new("true");
	if params.get("signed_only").is_some_and(|v| v == "true") {
		filter.clauses.push("e.signed".into());
	}
	filter.time_range(&params, "t");
	let sql = format!(
		"SELECT {TRANSFER} FROM {TRANSFERS_FROM} {} ORDER BY t.height DESC, t.event_index {}",
		filter.sql(),
		page.sql()
	);
	Ok(page.body(api.rows(&sql, &filter.params()).await?, None))
}

async fn accounts(State(api): State<Api>, Query(params): Params) -> ApiResult {
	let page = Page::from(&params);
	let sql =
		format!("SELECT {ACCOUNT} FROM explorer.accounts a ORDER BY a.total DESC {}", page.sql());
	let total = api.count("SELECT count(*) FROM explorer.accounts", &[]).await?;
	Ok(page.body(api.rows(&sql, &[]).await?, Some(total)))
}

async fn account_summary(State(api): State<Api>, Path(address): Path<String>) -> ApiResult {
	let address = normalize(&address).unwrap_or(address);
	let transfers = api
		.count(
			"SELECT count(*) FROM explorer.transfers WHERE \"from\" = $1 OR \"to\" = $1",
			&[&address],
		)
		.await?;
	let extrinsics = api
		.count("SELECT count(*) FROM explorer.extrinsics WHERE signer = $1", &[&address])
		.await?;
	Ok(Json(json!({ "transfersCount": transfers, "extrinsicsCount": extrinsics })))
}

async fn account_extrinsics(
	State(api): State<Api>,
	Path(address): Path<String>,
	Query(params): Params,
) -> ApiResult {
	let page = Page::from(&params);
	let address = normalize(&address).unwrap_or(address);
	let sql = format!(
		"SELECT {EXTRINSIC} FROM explorer.extrinsics e JOIN explorer.blocks b ON b.height = e.height
		 WHERE e.signer = $1 ORDER BY e.height DESC, e.index {}",
		page.sql()
	);
	let total = api
		.count("SELECT count(*) FROM explorer.extrinsics WHERE signer = $1", &[&address])
		.await?;
	Ok(page.body(api.rows(&sql, &[&address]).await?, Some(total)))
}

async fn account_transfers(
	State(api): State<Api>,
	Path(address): Path<String>,
	Query(params): Params,
) -> ApiResult {
	let page = Page::from(&params);
	let address = normalize(&address).unwrap_or(address);
	let sql = format!(
		"SELECT {TRANSFER} FROM {TRANSFERS_FROM} WHERE t.\"from\" = $1 OR t.\"to\" = $1
		 ORDER BY t.height DESC, t.event_index {}",
		page.sql()
	);
	let total = api
		.count(
			"SELECT count(*) FROM explorer.transfers WHERE \"from\" = $1 OR \"to\" = $1",
			&[&address],
		)
		.await?;
	Ok(page.body(api.rows(&sql, &[&address]).await?, Some(total)))
}

/** The section and method filters: for each runtime, newest first, every call and event name
 * seen up to it. */
async fn spec_filter(State(api): State<Api>) -> ApiResult {
	let specs = api
		.rows(
			"SELECT jsonb_build_object('specVersion', s.spec_version, 'pallets', (
				SELECT coalesce(jsonb_agg(jsonb_build_object('name', p.section,
					'calls', p.calls, 'events', p.events) ORDER BY p.section), '[]')
				FROM (SELECT section,
					coalesce(jsonb_agg(DISTINCT method) FILTER (WHERE kind = 'call'), '[]') AS calls,
					coalesce(jsonb_agg(DISTINCT method) FILTER (WHERE kind = 'event'), '[]') AS events
					FROM explorer.call_names WHERE spec_version <= s.spec_version
					GROUP BY section) p))
			 FROM (SELECT DISTINCT spec_version FROM explorer.call_names) s
			 ORDER BY s.spec_version DESC",
			&[],
		)
		.await?;
	Ok(Json(Value::Array(specs)))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn filters_number_their_placeholders_in_order() {
		let mut filter = Filter::new("true");
		filter.add("e.section = $", "balances".to_string());
		filter.add("e.height >= $", 5i64);
		assert_eq!(filter.sql(), "WHERE true AND e.section = $1 AND e.height >= $2");
		assert_eq!(filter.params().len(), 2);
	}

	#[test]
	fn pages_follow_the_site() {
		let params =
			HashMap::from([("page".into(), "2".into()), ("page_size".into(), "25".into())]);
		assert_eq!(Page::from(&params).sql(), "LIMIT 25 OFFSET 50");
		let huge = HashMap::from([("page_size".into(), "100000".into())]);
		assert_eq!(Page::from(&huge).sql(), "LIMIT 100 OFFSET 0");
	}
}
