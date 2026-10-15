use axum::{
	http::StatusCode,
	response::{IntoResponse, Response},
	Json,
};
use serde_json::json;

/// An error in the shape the SubSquare frontend reads, `{ message }` with the HTTP status.
pub struct ApiError {
	status: StatusCode,
	message: String,
}

impl ApiError {
	pub fn not_found(what: &str) -> Self {
		Self { status: StatusCode::NOT_FOUND, message: format!("{what} not found") }
	}
}

impl IntoResponse for ApiError {
	fn into_response(self) -> Response {
		(self.status, Json(json!({ "message": self.message }))).into_response()
	}
}

// Anything unexpected becomes a 500 and is logged, so internals never reach the client.
impl<E: Into<anyhow::Error>> From<E> for ApiError {
	fn from(err: E) -> Self {
		tracing::error!(target: crate::LOG_TARGET, "{:?}", err.into());
		Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: "Internal server error".into() }
	}
}

pub type ApiResult<T> = Result<T, ApiError>;
