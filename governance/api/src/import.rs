use std::collections::BTreeSet;

use anyhow::Context;
use serde_json::Value;
use tokio_postgres::Client as Db;

use crate::{
	crypto::canonical,
	forum_client::ForumClient,
	primitives::{comment_id, post_id, PostKind},
	routes::files,
};

/// The longest file id taken from a link in imported text.
const MAX_FILE_ID: usize = 128;

/// Where each kind of post lives in the current forum's API, and the field holding its number.
const SECTIONS: [(PostKind, &str, &str); 4] = [
	(PostKind::Referendum, "gov2/referendums", "referendumIndex"),
	(PostKind::Motion, "tech-comm/motions", "motionIndex"),
	(PostKind::Discussion, "posts", "postUid"),
	(PostKind::TreasurySpend, "treasury/spends", "index"),
];

/// Copies the off-chain content of the current forum into this one: post titles and descriptions,
/// discussions, comments and reactions. Running it again updates what it brought over before, so it
/// can be repeated right before switching over.
pub async fn run(db: &Db, site: &str) -> anyhow::Result<()> {
	let source = ForumClient::site(site)?;

	for (kind, route, number_field) in SECTIONS {
		let (mut posts, mut comments) = (0, 0);
		for item in source.all(route).await? {
			let number = match &item[number_field] {
				Value::String(number) => number.parse::<i64>().ok(),
				number => number.as_i64(),
			}
			.context("post without a number")?;
			// Motions are only found by the current forum's own id, everything else by its number
			let key = match kind {
				PostKind::Motion => item["_id"].as_str().unwrap_or_default().to_owned(),
				_ => number.to_string(),
			};
			let detail = source.get(&format!("{route}/{key}")).await?;

			if detail["title"].is_string() || detail["content"].is_string() {
				import_post(db, kind, number, &detail).await?;
				import_images(db, &source, &detail).await;
				posts += 1;
			}
			import_reactions(db, &post_id(kind, number), &detail["reactions"]).await?;

			if item["commentsCount"].as_i64().unwrap_or_default() > 0 {
				let path = format!("{route}/{}/comments", item["_id"].as_str().unwrap_or_default());
				for root in source.all(&path).await? {
					let has_replies = root["replies"].as_array().is_some_and(|r| !r.is_empty());
					let Some(id) =
						import_comment(db, kind, number, None, &root, has_replies).await?
					else {
						continue
					};
					comments += 1;
					for reply in root["replies"].as_array().into_iter().flatten() {
						comments += import_comment(db, kind, number, Some(id), reply, false)
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

/// Copies the images a post shows, its banner and any image the current forum hosts in its text,
/// under their existing ids so the post's links keep working. Post text is written by anyone, so a
/// link to a missing, oversized or non image file is logged and left behind rather than stopping
/// the import.
async fn import_images(db: &Db, source: &ForumClient, post: &Value) {
	let in_text = post["content"]
		.as_str()
		.unwrap_or_default()
		.split("/s3/files/")
		.skip(1)
		.map(|rest| rest.chars().take_while(char::is_ascii_alphanumeric).collect::<String>());
	let ids = post["bannerCid"]
		.as_str()
		.map(str::to_owned)
		.into_iter()
		.chain(in_text)
		.filter(|id| {
			(1..=MAX_FILE_ID).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric())
		})
		.collect::<BTreeSet<_>>();
	for id in ids {
		let imported = async {
			let data = source.file(&format!("s3/files/{id}"), files::MAX_FILE_SIZE).await?;
			let content_type = files::image_type(&data).context("not an image")?;
			files::store(db, &id, content_type, &data, None).await
		};
		if let Err(err) = imported.await {
			tracing::warn!(target: crate::LOG_TARGET, "Skipped file {id}: {err:?}");
		}
	}
}

async fn import_post(db: &Db, kind: PostKind, number: i64, post: &Value) -> anyhow::Result<()> {
	db.execute(
		"INSERT INTO posts (kind, number, title, content, content_type, author, labels, banner_cid, created_at, updated_at,
			edited)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::text::timestamptz, $10::text::timestamptz, $11)
		ON CONFLICT (kind, number) DO UPDATE SET title = $3, content = $4, content_type = $5, author = $6,
			labels = $7, banner_cid = $8, created_at = $9::text::timestamptz, updated_at = $10::text::timestamptz,
			edited = $11",
		&[
			&(kind as i16),
			&number,
			&post["title"].as_str(),
			&post["content"].as_str(),
			&post["contentType"].as_str().unwrap_or("markdown"),
			&post["author"]["address"].as_str().map(address),
			&post.get("labels").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
			&post["bannerCid"].as_str(),
			&post["createdAt"].as_str(),
			&post["updatedAt"].as_str(),
			&post["edited"].as_bool().unwrap_or_default(),
		],
	)
	.await?;
	Ok(())
}

/// Imports a comment and its reactions, returning its new id. Comments from accounts without an
/// address, which the current forum allows for email sign ups, are left out.
async fn import_comment(
	db: &Db,
	kind: PostKind,
	number: i64,
	reply_to: Option<i64>,
	comment: &Value,
	has_replies: bool,
) -> anyhow::Result<Option<i64>> {
	// Comments by accounts with no address, such as email accounts, can't belong to anyone here.
	// One with replies stays as a deleted comment, so its replies keep their thread.
	let author = comment["author"]["address"].as_str().map(address);
	if author.is_none() && !has_replies {
		return Ok(None);
	}
	let content = match author {
		Some(_) => comment["content"].as_str().unwrap_or_default(),
		None => "",
	};
	let row = db
		.query_one(
			"INSERT INTO comments (post_kind, post_number, reply_to, author, content, content_type, imported_from,
				created_at, updated_at, deleted)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8::text::timestamptz, $9::text::timestamptz, $10)
			ON CONFLICT (imported_from) DO UPDATE SET content = $5, content_type = $6, updated_at = $9::text::timestamptz,
				deleted = comments.deleted OR $10
			RETURNING id",
			&[
				&(kind as i16),
				&number,
				&reply_to,
				&author.as_deref().unwrap_or_default(),
				&content,
				&comment["contentType"].as_str().unwrap_or("markdown"),
				&comment["_id"].as_str(),
				&comment["createdAt"].as_str(),
				&comment["updatedAt"].as_str(),
				&author.is_none(),
			],
		)
		.await?;
	let id = row.get::<_, i64>(0);
	import_reactions(db, &comment_id(id), &comment["reactions"]).await?;
	Ok(Some(id))
}

/// An imported address in the chain's format, so imported content belongs to the same account as
/// what its author writes here. Anything that is not an address is kept as it was.
fn address(imported: &str) -> String {
	canonical(imported).unwrap_or_else(|| imported.to_owned())
}

async fn import_reactions(db: &Db, target: &str, reactions: &Value) -> anyhow::Result<()> {
	for reaction in reactions.as_array().into_iter().flatten() {
		let Some(address) = reaction["user"]["address"]
			.as_str()
			.or(reaction["proposer"].as_str())
			.map(address)
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
