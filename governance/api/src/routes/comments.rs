use axum::{
	extract::{Path, RawQuery, State},
	Json,
};
use serde::Deserialize;
use serde_json::{Map, Value};
use tokio_postgres::Row;

use super::{auth::Session, posts::reactions, AppState, Params};
use crate::{
	error::{ApiError, ApiResult},
	primitives::{
		author, comment_id, iso, parse_comment_id, parse_post_id, post_id, Page, PostKind,
	},
};

const COMMENTS: &str = "SELECT id, post_kind, post_number, reply_to, author, content, content_type,
	ms(created_at) AS created_ms, ms(updated_at) AS updated_ms FROM comments";
const MAX_PAGE_SIZE: i64 = 200;

/// A comment as SubSquare returns it. `height` is its position among the post's comments, which the
/// frontend shows and links to.
fn comment(row: &Row, height: i64, reactions: Value, replies: Vec<Value>) -> Value {
	let kind =
		PostKind::from_code(row.get::<_, i16>("post_kind") as u8).unwrap_or(PostKind::Discussion);
	let fields = [
		("_id", comment_id(row.get("id")).into()),
		(kind.post_type(), post_id(kind, row.get("post_number")).into()),
		("replyToComment", row.get::<_, Option<i64>>("reply_to").map(comment_id).into()),
		("content", row.get::<_, String>("content").into()),
		("contentType", row.get::<_, String>("content_type").into()),
		("contentVersion", "2".into()),
		("author", author(&row.get::<_, String>("author"))),
		("height", height.into()),
		("createdAt", iso(row.get("created_ms")).into()),
		("updatedAt", iso(row.get("updated_ms")).into()),
		("reactions", reactions),
		("replies", replies.into()),
	];
	Value::Object(
		fields
			.into_iter()
			.map(|(key, value)| (key.to_owned(), value))
			.collect::<Map<_, _>>(),
	)
}

/// The post a comment route addresses, which must exist and be visible.
async fn commented_post(state: &AppState, id: &str) -> ApiResult<(PostKind, i64)> {
	let (kind, number) = parse_post_id(id).ok_or_else(|| ApiError::not_found("Post"))?;
	let query = match kind {
		PostKind::Discussion => "SELECT 1 FROM posts WHERE kind = 0 AND number = $1 AND NOT hidden",
		PostKind::Referendum =>
			"SELECT 1 FROM governance_referendums WHERE index = $1::bigint::int",
		PostKind::Motion => "SELECT 1 FROM governance_motions WHERE index = $1::bigint::int",
		PostKind::TreasurySpend =>
			"SELECT 1 FROM governance_treasury_spends WHERE index = $1::bigint::int",
	};
	state
		.db
		.query_opt(query, &[&number])
		.await?
		.ok_or_else(|| ApiError::not_found("Post"))?;
	Ok((kind, number))
}

/// A post's comments with their replies nested, a page of top level comments at a time. The
/// frontend asks for the `last` page first.
pub async fn list(
	State(state): State<AppState>,
	Path(id): Path<String>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	let (kind, number) = commented_post(&state, &id).await?;
	let params = Params::new(query);
	let page_size = params
		.get("page_size")
		.and_then(|size| size.parse().ok())
		.unwrap_or(MAX_PAGE_SIZE)
		.clamp(1, MAX_PAGE_SIZE);
	let total = state
		.db
		.query_one(
			"SELECT count(*) FROM comments WHERE post_kind = $1 AND post_number = $2 AND reply_to IS NULL AND NOT deleted",
			&[&(kind as i16), &number],
		)
		.await?
		.get::<_, i64>(0);
	let last_page = ((total + page_size - 1) / page_size).max(1);
	let page = match params.get("page") {
		Some("last") | None => last_page,
		Some(page) => page.parse::<i64>().unwrap_or(1).clamp(1, last_page),
	};
	let offset = (page - 1) * page_size;

	let roots = state
		.db
		.query(
			&format!(
				"{COMMENTS} WHERE post_kind = $1 AND post_number = $2 AND reply_to IS NULL AND NOT deleted
				ORDER BY id LIMIT {page_size} OFFSET {offset}"
			),
			&[&(kind as i16), &number],
		)
		.await?;
	let root_ids = roots.iter().map(|row| row.get::<_, i64>("id")).collect::<Vec<_>>();
	let replies = state
		.db
		.query(
			&format!("{COMMENTS} WHERE reply_to = ANY($1) AND NOT deleted ORDER BY id"),
			&[&root_ids],
		)
		.await?;
	let reactions = reactions(
		&state,
		&roots
			.iter()
			.chain(&replies)
			.map(|row| comment_id(row.get("id")))
			.collect::<Vec<_>>(),
	)
	.await?;
	let reactions_of = |row: &Row| {
		Value::from(reactions.get(&comment_id(row.get("id"))).cloned().unwrap_or_default())
	};

	let items = roots
		.iter()
		.enumerate()
		.map(|(position, root)| {
			let nested = replies
				.iter()
				.filter(|reply| reply.get::<_, Option<i64>>("reply_to") == Some(root.get("id")))
				.map(|reply| comment(reply, 0, reactions_of(reply), Vec::new()))
				.collect();
			comment(root, offset + position as i64 + 1, reactions_of(root), nested)
		})
		.collect();
	Ok(Json(Page { items, page, page_size, total }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentInput {
	content: String,
	#[serde(default = "markdown")]
	content_type: String,
}

fn markdown() -> String {
	"markdown".into()
}

impl CommentInput {
	fn validate(&self) -> ApiResult<()> {
		if self.content.trim().is_empty() || self.content.len() > 65_536 {
			return Err(ApiError::bad_request("The comment must be between 1 and 65536 characters"));
		}
		if !matches!(self.content_type.as_str(), "markdown" | "html") {
			return Err(ApiError::bad_request("Unknown content type"));
		}
		Ok(())
	}
}

async fn insert(
	state: &AppState,
	session: &Session,
	(kind, number): (PostKind, i64),
	reply_to: Option<i64>,
	input: CommentInput,
) -> ApiResult<Json<Value>> {
	input.validate()?;
	let id = state
		.db
		.query_one(
			"INSERT INTO comments (post_kind, post_number, reply_to, author, content, content_type)
			VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
			&[
				&(kind as i16),
				&number,
				&reply_to,
				&session.address,
				&input.content,
				&input.content_type,
			],
		)
		.await?
		.get::<_, i64>(0);
	let row = find(state, &comment_id(id)).await?;
	Ok(Json(comment(&row, 0, Value::Array(Vec::new()), Vec::new())))
}

pub async fn create(
	State(state): State<AppState>,
	session: Session,
	Path(id): Path<String>,
	Json(input): Json<CommentInput>,
) -> ApiResult<Json<Value>> {
	let post = commented_post(&state, &id).await?;
	insert(&state, &session, post, None, input).await
}

/// Replies to a top level comment on the same post.
pub async fn reply(
	State(state): State<AppState>,
	session: Session,
	Path((id, parent)): Path<(String, String)>,
	Json(input): Json<CommentInput>,
) -> ApiResult<Json<Value>> {
	let (kind, number) = commented_post(&state, &id).await?;
	let parent = parse_comment_id(&parent).ok_or_else(|| ApiError::not_found("Comment"))?;
	state
		.db
		.query_opt(
			"SELECT 1 FROM comments WHERE id = $1 AND post_kind = $2 AND post_number = $3 AND reply_to IS NULL AND NOT deleted",
			&[&parent, &(kind as i16), &number],
		)
		.await?
		.ok_or_else(|| ApiError::not_found("Comment"))?;
	insert(&state, &session, (kind, number), Some(parent), input).await
}

async fn find(state: &AppState, id: &str) -> ApiResult<Row> {
	let id = parse_comment_id(id).ok_or_else(|| ApiError::not_found("Comment"))?;
	state
		.db
		.query_opt(&format!("{COMMENTS} WHERE id = $1 AND NOT deleted"), &[&id])
		.await?
		.ok_or_else(|| ApiError::not_found("Comment"))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
	let row = find(&state, &id).await?;
	let mut reactions = reactions(&state, &[id.clone()]).await?;
	Ok(Json(comment(&row, 0, reactions.remove(&id).unwrap_or_default().into(), Vec::new())))
}

pub async fn edit(
	State(state): State<AppState>,
	session: Session,
	Path(id): Path<String>,
	Json(input): Json<CommentInput>,
) -> ApiResult<Json<bool>> {
	input.validate()?;
	let row = find(&state, &id).await?;
	if row.get::<_, String>("author") != session.address {
		return Err(ApiError::forbidden());
	}
	state
		.db
		.execute(
			"UPDATE comments SET content = $2, content_type = $3, updated_at = NOW() WHERE id = $1",
			&[&row.get::<_, i64>("id"), &input.content, &input.content_type],
		)
		.await?;
	Ok(Json(true))
}

pub async fn delete(
	State(state): State<AppState>,
	session: Session,
	Path(id): Path<String>,
) -> ApiResult<Json<bool>> {
	let row = find(&state, &id).await?;
	if row.get::<_, String>("author") != session.address && !state.admins.contains(&session.address)
	{
		return Err(ApiError::forbidden());
	}
	state
		.db
		.execute("UPDATE comments SET deleted = TRUE WHERE id = $1", &[&row.get::<_, i64>("id")])
		.await?;
	Ok(Json(true))
}
