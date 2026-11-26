use axum::{
	extract::{Multipart, Path, State},
	http::header::{
		CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_SECURITY_POLICY, CONTENT_TYPE,
		X_CONTENT_TYPE_OPTIONS,
	},
	response::IntoResponse,
	Json,
};
use serde_json::{json, Value};
use sp_crypto_hashing::sha2_256;
use tokio_postgres::Client;

use super::{auth::Session, AppState};
use crate::error::{ApiError, ApiResult};

pub const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;
/// What one account may upload in a day.
const DAILY_FILES: i64 = 50;
const DAILY_BYTES: i64 = 50 * 1024 * 1024;

/// The image types the forum keeps, with the bytes each starts with.
const IMAGE_TYPES: [(&str, &[u8]); 4] = [
	("image/png", b"\x89PNG\r\n\x1a\n"),
	("image/jpeg", b"\xff\xd8\xff"),
	("image/gif", b"GIF8"),
	("image/webp", b"RIFF"),
];

/// The image type `data` holds, judged by its first bytes rather than what the sender claims.
pub fn image_type(data: &[u8]) -> Option<&'static str> {
	IMAGE_TYPES
		.iter()
		.find(|(kind, magic)| {
			data.starts_with(magic) && (*kind != "image/webp" || data.get(8..12) == Some(b"WEBP"))
		})
		.map(|(kind, _)| *kind)
}

/// Stores an image under `id`, unless `uploader` has used up its daily quota. Uploads use a hash
/// of the content, imports keep the current forum's id so the links in imported posts keep
/// working, and have no uploader. Returns whether the file is stored, including when it already
/// was.
pub async fn store(
	db: &Client,
	id: &str,
	content_type: &str,
	data: &[u8],
	uploader: Option<&str>,
) -> anyhow::Result<bool> {
	let size = i32::try_from(data.len())?;
	let inserted = db
		.execute(
			"INSERT INTO files (id, content_type, data, uploader, size)
			SELECT $1, $2, $3::bytea, $4, $5::int WHERE $4::text IS NULL OR (SELECT count(*) < $6
				AND COALESCE(sum(size), 0) + $5::int <= $7 FROM files
				WHERE uploader = $4 AND created_at > NOW() - INTERVAL '1 day')
			ON CONFLICT (id) DO NOTHING",
			&[&id, &content_type, &data, &uploader, &size, &DAILY_FILES, &DAILY_BYTES],
		)
		.await?;
	if inserted == 1 {
		return Ok(true);
	}
	let renewed = db.execute("UPDATE files SET uploaded_at = NOW() WHERE id = $1", &[&id]).await?;
	Ok(renewed == 1)
}

/// Uploads an image for a post or banner. The frontend shows banners by `cid` and embeds `url` in
/// text.
pub async fn upload(
	State(state): State<AppState>,
	session: Session,
	mut form: Multipart,
) -> ApiResult<Json<Value>> {
	while let Some(field) =
		form.next_field().await.map_err(|_| ApiError::bad_request("Invalid upload"))?
	{
		if field.name() != Some("file") {
			continue;
		}
		let declared = field.content_type().unwrap_or_default().to_owned();
		let data = field
			.bytes()
			.await
			.map_err(|_| ApiError::bad_request("The image is larger than 5MB"))?;
		if image_type(&data) != Some(declared.as_str()) {
			return Err(ApiError::bad_request(
				"Only png, jpeg, gif and webp images can be uploaded",
			));
		}
		let id = hex::encode(sha2_256(&data));
		let db = state.db.get().await?;
		if !store(&db, &id, &declared, &data, Some(&session.address)).await? {
			return Err(ApiError::bad_request("You have uploaded as much as you can today"));
		}
		return Ok(Json(json!({ "cid": id, "url": format!("/api/s3/files/{id}") })));
	}
	Err(ApiError::bad_request("No file in the upload"))
}

/// Serves a stored file. Only images are served as what they are, anything else downloads, and
/// nothing served can run script on the site.
pub async fn get(
	State(state): State<AppState>,
	Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
	let row = state
		.db
		.get()
		.await?
		.query_opt("SELECT content_type, data FROM files WHERE id = $1", &[&id])
		.await?
		.ok_or_else(|| ApiError::not_found("File"))?;
	let stored = row.get::<_, String>(0);
	let (content_type, disposition) = match IMAGE_TYPES.iter().find(|(kind, _)| *kind == stored) {
		Some((kind, _)) => (*kind, "inline"),
		None => ("application/octet-stream", "attachment"),
	};
	Ok((
		[
			(CONTENT_TYPE, content_type),
			(CONTENT_DISPOSITION, disposition),
			(X_CONTENT_TYPE_OPTIONS, "nosniff"),
			(CONTENT_SECURITY_POLICY, "default-src 'none'; sandbox"),
			(CACHE_CONTROL, "public, max-age=31536000, immutable"),
		],
		row.get::<_, Vec<u8>>(1),
	))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn knows_images_by_their_first_bytes() {
		assert_eq!(image_type(b"\x89PNG\r\n\x1a\nrest"), Some("image/png"));
		assert_eq!(image_type(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
		assert_eq!(image_type(b"RIFF\0\0\0\0WAVE"), None);
		assert_eq!(image_type(b"<svg xmlns="), None);
	}
}
