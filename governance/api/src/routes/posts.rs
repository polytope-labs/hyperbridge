use std::collections::HashMap;

use axum::{
	extract::{Path, RawQuery, State},
	Json,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use tokio_postgres::Row;

use super::{auth::Session, AppState, Params};
use crate::{
	error::{ApiError, ApiResult},
	primitives::{author, iso, parse_post_id, post_id, Page, PostKind},
};

const DISCUSSIONS: &str = "
	SELECT p.number, p.title, p.content, p.content_type, p.author, p.labels, p.banner_cid,
		ms(p.created_at) AS created_ms, ms(p.updated_at) AS updated_ms,
		c.count AS comments_count, c.last_ms AS comment_ms
	FROM posts p
	CROSS JOIN LATERAL (SELECT count(*) AS count, ms(max(created_at)) AS last_ms
		FROM comments WHERE post_kind = 0 AND post_number = p.number AND NOT deleted) c
	WHERE p.kind = 0 AND NOT p.hidden";

fn discussion(row: &Row) -> Value {
	let number = row.get::<_, i64>("number");
	let created = row.get::<_, i64>("created_ms");
	let last_comment = row.get::<_, Option<i64>>("comment_ms").unwrap_or_default();
	json!({
		"_id": post_id(PostKind::Discussion, number),
		"postUid": number.to_string(),
		"title": row.get::<_, Option<String>>("title"),
		"content": row.get::<_, Option<String>>("content"),
		"contentType": row.get::<_, String>("content_type"),
		"author": author(&row.get::<_, Option<String>>("author").unwrap_or_default()),
		"labels": row.get::<_, Value>("labels"),
		"bannerCid": row.get::<_, Option<String>>("banner_cid"),
		"poll": null,
		"createdAt": iso(created),
		"updatedAt": iso(row.get("updated_ms")),
		"lastActivityAt": iso(created.max(last_comment)),
		"commentsCount": row.get::<_, i64>("comments_count"),
		"refToPost": { "postId": post_id(PostKind::Discussion, number), "postType": PostKind::Discussion.post_type() },
	})
}

pub async fn list(
	State(state): State<AppState>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	let params = Params::new(query);
	let paging = params.paging(10);
	let labels = Value::from(params.get("label").map(|label| vec![label]).unwrap_or_default());
	// Active discussions are those started or commented on in the last week
	let active =
		if params.flag("active") { "AND last_activity > NOW() - INTERVAL '7 days'" } else { "" };
	let author = params.get("author").map(str::to_owned);
	let discussions = format!(
		"SELECT * FROM ({DISCUSSIONS} AND p.labels @> $1 AND ($2::text IS NULL OR p.author = $2)) d
		CROSS JOIN LATERAL (SELECT GREATEST(to_timestamp(d.created_ms / 1000.0),
			to_timestamp(d.comment_ms / 1000.0)) AS last_activity) a WHERE TRUE {active}"
	);

	let rows = state
		.db
		.query(
			&format!(
				"{discussions} ORDER BY last_activity DESC LIMIT {} OFFSET {}",
				paging.page_size,
				paging.offset()
			),
			&[&labels, &author],
		)
		.await?;
	let total = state
		.db
		.query_one(&format!("SELECT count(*) FROM ({discussions}) items"), &[&labels, &author])
		.await?
		.get::<_, i64>(0);
	Ok(Json(paging.page(rows.iter().map(discussion).collect(), total)))
}

/// A discussion by its number, which its page URL carries, or by its post id.
pub async fn detail(
	State(state): State<AppState>,
	Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
	let number = match parse_post_id(&id) {
		Some((PostKind::Discussion, number)) => number,
		_ => id.parse().map_err(|_| ApiError::not_found("Post"))?,
	};
	let row = state
		.db
		.query_opt(&format!("{DISCUSSIONS} AND p.number = $1"), &[&number])
		.await?
		.ok_or_else(|| ApiError::not_found("Post"))?;

	let mut post = discussion(&row);
	post["authors"] = json!([post["author"]["address"]]);
	post["reactions"] = post_reactions(&state, post_id(PostKind::Discussion, number)).await?;
	Ok(Json(post))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostInput {
	title: String,
	content: String,
	#[serde(default = "markdown")]
	content_type: String,
	banner_cid: Option<String>,
	#[serde(default)]
	labels: Vec<String>,
}

fn markdown() -> String {
	"markdown".into()
}

impl PostInput {
	fn validate(&self) -> ApiResult<()> {
		let title = self.title.trim();
		if title.is_empty() || title.chars().count() > 256 {
			return Err(ApiError::bad_request("The title must be between 1 and 256 characters"));
		}
		if self.content.trim().is_empty() || self.content.len() > 100_000 {
			return Err(ApiError::bad_request(
				"The content must be between 1 and 100000 characters",
			));
		}
		if !matches!(self.content_type.as_str(), "markdown" | "html") {
			return Err(ApiError::bad_request("Unknown content type"));
		}
		Ok(())
	}
}

/// Starts a discussion and returns its number, which the frontend opens next.
pub async fn create(
	State(state): State<AppState>,
	session: Session,
	Json(input): Json<PostInput>,
) -> ApiResult<Json<i64>> {
	input.validate()?;
	let row = state
		.db
		.query_one(
			"INSERT INTO posts (kind, number, title, content, content_type, author, labels, banner_cid)
			VALUES (0, nextval('discussion_number'), $1, $2, $3, $4, $5, $6) RETURNING number",
			&[
				&input.title.trim(),
				&input.content,
				&input.content_type,
				&session.address,
				&Value::from(input.labels),
				&input.banner_cid,
			],
		)
		.await?;
	Ok(Json(row.get(0)))
}

/// Who may write a post: a discussion's author, or the proposer of the on-chain item it describes.
pub async fn owner(state: &AppState, kind: PostKind, number: i64) -> ApiResult<Option<String>> {
	let query = match kind {
		PostKind::Discussion => "SELECT author FROM posts WHERE kind = 0 AND number = $1",
		PostKind::Referendum =>
			"SELECT proposer FROM governance_referendums WHERE index = $1::bigint::int",
		PostKind::Motion => "SELECT proposer FROM governance_motions WHERE index = $1::bigint::int",
		PostKind::TreasurySpend => return Ok(None),
	};
	Ok(state.db.query_opt(query, &[&number]).await?.and_then(|row| row.get(0)))
}

/// The post a write route addresses, checked to be one the signed in account may manage.
async fn managed_post(state: &AppState, session: &Session, id: &str) -> ApiResult<(PostKind, i64)> {
	let (kind, number) = parse_post_id(id).ok_or_else(|| ApiError::not_found("Post"))?;
	let owner = owner(state, kind, number).await?;
	if owner.as_deref() != Some(session.address.as_str()) &&
		!state.admins.contains(&session.address)
	{
		return Err(ApiError::forbidden());
	}
	Ok((kind, number))
}

/// Edits the title and content of a discussion or of the post describing an on-chain item.
pub async fn edit(
	State(state): State<AppState>,
	session: Session,
	Path(id): Path<String>,
	Json(input): Json<PostInput>,
) -> ApiResult<Json<bool>> {
	input.validate()?;
	let (kind, number) = managed_post(&state, &session, &id).await?;
	state
		.db
		.execute(
			"INSERT INTO posts (kind, number, title, content, content_type, author, labels, banner_cid)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
			ON CONFLICT (kind, number) DO UPDATE SET title = $3, content = $4, content_type = $5, labels = $7,
				banner_cid = $8, updated_at = NOW()",
			&[
				&(kind as i16),
				&number,
				&input.title.trim(),
				&input.content,
				&input.content_type,
				&session.address,
				&Value::from(input.labels),
				&input.banner_cid,
			],
		)
		.await?;
	Ok(Json(true))
}

pub async fn delete(
	State(state): State<AppState>,
	session: Session,
	Path(id): Path<String>,
) -> ApiResult<Json<bool>> {
	let (kind, number) = managed_post(&state, &session, &id).await?;
	if kind != PostKind::Discussion {
		return Err(ApiError::bad_request("Only discussions can be deleted"));
	}
	state
		.db
		.execute("UPDATE posts SET hidden = TRUE WHERE kind = 0 AND number = $1", &[&number])
		.await?;
	Ok(Json(true))
}

#[derive(Deserialize)]
pub struct HideInput {
	hide: bool,
}

pub async fn hide(
	State(state): State<AppState>,
	session: Session,
	Path(id): Path<String>,
	Json(input): Json<HideInput>,
) -> ApiResult<Json<bool>> {
	if !state.admins.contains(&session.address) {
		return Err(ApiError::forbidden());
	}
	let Some((PostKind::Discussion, number)) = parse_post_id(&id) else {
		return Err(ApiError::not_found("Post"));
	};
	state
		.db
		.execute(
			"UPDATE posts SET hidden = $2 WHERE kind = 0 AND number = $1",
			&[&number, &input.hide],
		)
		.await?;
	Ok(Json(true))
}

/// Reactions on posts or comments, keyed by the post or comment id they are on.
pub async fn reactions(
	state: &AppState,
	targets: &[String],
) -> ApiResult<HashMap<String, Vec<Value>>> {
	let rows = state
		.db
		.query(
			"SELECT target, address, reaction, ms(created_at) FROM reactions WHERE target = ANY($1) ORDER BY created_at",
			&[&targets],
		)
		.await?;
	Ok(rows.iter().fold(HashMap::new(), |mut by_target, row| {
		let address = row.get::<_, String>(1);
		by_target.entry(row.get::<_, String>(0)).or_insert_with(Vec::new).push(json!({
			"reaction": row.get::<_, i16>(2),
			"user": author(&address),
			"createdAt": iso(row.get(3)),
		}));
		by_target
	}))
}

pub async fn post_reactions(state: &AppState, id: String) -> ApiResult<Value> {
	let mut reactions = reactions(state, &[id.clone()]).await?;
	Ok(reactions.remove(&id).unwrap_or_default().into())
}

#[derive(Deserialize)]
pub struct ReactionInput {
	reaction: i16,
}

/// Sets the signed in account's reaction on a post or comment, 1 for up and 0 for down.
pub async fn react(
	State(state): State<AppState>,
	session: Session,
	Path(id): Path<String>,
	Json(input): Json<ReactionInput>,
) -> ApiResult<Json<bool>> {
	if !matches!(input.reaction, 0 | 1) {
		return Err(ApiError::bad_request("Unknown reaction"));
	}
	state
		.db
		.execute(
			"INSERT INTO reactions (target, address, reaction) VALUES ($1, $2, $3)
			ON CONFLICT (target, address) DO UPDATE SET reaction = $3, created_at = NOW()",
			&[&id, &session.address, &input.reaction],
		)
		.await?;
	Ok(Json(true))
}

pub async fn unreact(
	State(state): State<AppState>,
	session: Session,
	Path(id): Path<String>,
) -> ApiResult<Json<bool>> {
	state
		.db
		.execute(
			"DELETE FROM reactions WHERE target = $1 AND address = $2",
			&[&id, &session.address],
		)
		.await?;
	Ok(Json(true))
}

#[derive(Deserialize)]
pub struct CommentFilterInput {
	filter: Map<String, Value>,
}

/// Saves how a referendum's comments are filtered for every visitor, which its proposer or an admin
/// may choose.
pub async fn set_comment_filter(
	State(state): State<AppState>,
	session: Session,
	Path(index): Path<i64>,
	Json(input): Json<CommentFilterInput>,
) -> ApiResult<Json<bool>> {
	let (kind, number) =
		managed_post(&state, &session, &post_id(PostKind::Referendum, index)).await?;
	state
		.db
		.execute(
			"INSERT INTO posts (kind, number, comment_filter) VALUES ($1, $2, $3)
			ON CONFLICT (kind, number) DO UPDATE SET comment_filter = $3",
			&[&(kind as i16), &number, &Value::Object(input.filter)],
		)
		.await?;
	Ok(Json(true))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportInput {
	post_type: String,
	post_id: String,
	reason: String,
}

/// Records a visitor's report of a post for the admins to review.
pub async fn report(
	State(state): State<AppState>,
	session: Session,
	Json(input): Json<ReportInput>,
) -> ApiResult<Json<bool>> {
	if input.reason.trim().is_empty() || input.reason.chars().count() > 1000 {
		return Err(ApiError::bad_request("The reason must be between 1 and 1000 characters"));
	}
	state
		.db
		.execute(
			"INSERT INTO reports (post_type, post_id, reason, reporter) VALUES ($1, $2, $3, $4)",
			&[&input.post_type, &input.post_id, &input.reason.trim(), &session.address],
		)
		.await?;
	Ok(Json(true))
}
