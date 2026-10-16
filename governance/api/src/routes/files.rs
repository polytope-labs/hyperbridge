use axum::{
	extract::{Multipart, Path, State},
	http::header::{CACHE_CONTROL, CONTENT_TYPE},
	response::IntoResponse,
	Json,
};
use serde_json::{json, Value};
use sp_crypto_hashing::sha2_256;
use tokio_postgres::Client;

use super::{auth::Session, AppState};
use crate::error::{ApiError, ApiResult};

pub const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;
const IMAGE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];

/// Stores an image under `id`. Uploads use a hash of the content, imports keep the current forum's
/// id so the links in imported posts keep working.
pub async fn store(db: &Client, id: &str, content_type: &str, data: &[u8]) -> anyhow::Result<()> {
	db.execute(
		"INSERT INTO files (id, content_type, data) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
		&[&id, &content_type, &data],
	)
	.await?;
	Ok(())
}

/// Uploads an image for a post or banner. The frontend shows banners by `cid` and embeds `url` in
/// text.
pub async fn upload(
	State(state): State<AppState>,
	_: Session,
	mut form: Multipart,
) -> ApiResult<Json<Value>> {
	while let Some(field) =
		form.next_field().await.map_err(|_| ApiError::bad_request("Invalid upload"))?
	{
		if field.name() != Some("file") {
			continue;
		}
		let content_type = field.content_type().unwrap_or_default().to_owned();
		if !IMAGE_TYPES.contains(&content_type.as_str()) {
			return Err(ApiError::bad_request(
				"Only png, jpeg, gif and webp images can be uploaded",
			));
		}
		let data = field
			.bytes()
			.await
			.map_err(|_| ApiError::bad_request("The image is larger than 5MB"))?;
		let id = hex::encode(sha2_256(&data));
		store(&state.db, &id, &content_type, &data).await?;
		return Ok(Json(json!({ "cid": id, "url": format!("/api/s3/files/{id}") })));
	}
	Err(ApiError::bad_request("No file in the upload"))
}

pub async fn get(
	State(state): State<AppState>,
	Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
	let row = state
		.db
		.query_opt("SELECT content_type, data FROM files WHERE id = $1", &[&id])
		.await?
		.ok_or_else(|| ApiError::not_found("File"))?;
	Ok((
		[
			(CONTENT_TYPE, row.get::<_, String>(0)),
			(CACHE_CONTROL, "public, max-age=31536000, immutable".into()),
		],
		row.get::<_, Vec<u8>>(1),
	))
}
