use anyhow::Context;
use serde_json::Value;
use tokio_postgres::Client as Db;

use crate::{
	primitives::{comment_id, post_id, PostKind},
	subsquare::SubSquareApi,
};

/// Where each kind of post lives in SubSquare's API, and the field holding its number.
const SECTIONS: [(PostKind, &str, &str); 4] = [
	(PostKind::Referendum, "gov2/referendums", "referendumIndex"),
	(PostKind::Motion, "tech-comm/motions", "motionIndex"),
	(PostKind::Discussion, "posts", "postUid"),
	(PostKind::TreasurySpend, "treasury/spends", "index"),
];

/// Copies the off-chain content of a SubSquare site into the forum: post titles and descriptions,
/// discussions, comments and reactions. Running it again updates what it brought over before, so it
/// can be repeated right before switching over.
pub async fn run(db: &Db, site: &str) -> anyhow::Result<()> {
	let source = SubSquareApi::site(site);

	for (kind, route, number_field) in SECTIONS {
		let (mut posts, mut comments) = (0, 0);
		for item in source.all(route).await? {
			let number = match &item[number_field] {
				Value::String(number) => number.parse::<i64>().ok(),
				number => number.as_i64(),
			}
			.context("post without a number")?;
			// Motions are only found by SubSquare's own id, everything else by its number
			let key = match kind {
				PostKind::Motion => item["_id"].as_str().unwrap_or_default().to_owned(),
				_ => number.to_string(),
			};
			let detail = source.get(&format!("{route}/{key}")).await?;

			if detail["title"].is_string() || detail["content"].is_string() {
				import_post(db, kind, number, &detail).await?;
				posts += 1;
			}
			import_reactions(db, &post_id(kind, number), &detail["reactions"]).await?;

			if item["commentsCount"].as_i64().unwrap_or_default() > 0 {
				let path = format!("{route}/{}/comments", item["_id"].as_str().unwrap_or_default());
				for root in source.all(&path).await? {
					let Some(id) = import_comment(db, kind, number, None, &root).await? else {
						continue
					};
					comments += 1;
					for reply in root["replies"].as_array().into_iter().flatten() {
						comments += import_comment(db, kind, number, Some(id), reply)
							.await?
							.is_some() as usize;
					}
				}
			}
		}
		tracing::info!(target: crate::LOG_TARGET, "Imported {posts} posts and {comments} comments from {route}");
	}

	// New discussions continue after the highest imported number
	db.execute(
		"SELECT setval('discussion_number', GREATEST((SELECT max(number) FROM posts WHERE kind = 0), 1))",
		&[],
	)
	.await?;
	Ok(())
}

async fn import_post(db: &Db, kind: PostKind, number: i64, post: &Value) -> anyhow::Result<()> {
	db.execute(
		"INSERT INTO posts (kind, number, title, content, content_type, author, labels, banner_cid, created_at, updated_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::text::timestamptz, $10::text::timestamptz)
		ON CONFLICT (kind, number) DO UPDATE SET title = $3, content = $4, content_type = $5, author = $6,
			labels = $7, banner_cid = $8, created_at = $9::text::timestamptz, updated_at = $10::text::timestamptz",
		&[
			&(kind as i16),
			&number,
			&post["title"].as_str(),
			&post["content"].as_str(),
			&post["contentType"].as_str().unwrap_or("markdown"),
			&post["author"]["address"].as_str(),
			&post.get("labels").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
			&post["bannerCid"].as_str(),
			&post["createdAt"].as_str(),
			&post["updatedAt"].as_str(),
		],
	)
	.await?;
	Ok(())
}

/// Imports a comment and its reactions, returning its new id. Comments from accounts without an
/// address, which SubSquare allows for email sign ups, are left out.
async fn import_comment(
	db: &Db,
	kind: PostKind,
	number: i64,
	reply_to: Option<i64>,
	comment: &Value,
) -> anyhow::Result<Option<i64>> {
	let Some(author) = comment["author"]["address"].as_str() else { return Ok(None) };
	let row = db
		.query_one(
			"INSERT INTO comments (post_kind, post_number, reply_to, author, content, content_type, imported_from,
				created_at, updated_at)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8::text::timestamptz, $9::text::timestamptz)
			ON CONFLICT (imported_from) DO UPDATE SET content = $5, content_type = $6, updated_at = $9::text::timestamptz
			RETURNING id",
			&[
				&(kind as i16),
				&number,
				&reply_to,
				&author,
				&comment["content"].as_str().unwrap_or_default(),
				&comment["contentType"].as_str().unwrap_or("markdown"),
				&comment["_id"].as_str(),
				&comment["createdAt"].as_str(),
				&comment["updatedAt"].as_str(),
			],
		)
		.await?;
	let id = row.get::<_, i64>(0);
	import_reactions(db, &comment_id(id), &comment["reactions"]).await?;
	Ok(Some(id))
}

async fn import_reactions(db: &Db, target: &str, reactions: &Value) -> anyhow::Result<()> {
	for reaction in reactions.as_array().into_iter().flatten() {
		let Some(address) = reaction["user"]["address"].as_str().or(reaction["proposer"].as_str())
		else {
			continue
		};
		db.execute(
			"INSERT INTO reactions (target, address, reaction) VALUES ($1, $2, $3)
			ON CONFLICT (target, address) DO UPDATE SET reaction = $3",
			&[&target, &address, &(reaction["reaction"].as_i64().unwrap_or(1) as i16)],
		)
		.await?;
	}
	Ok(())
}
