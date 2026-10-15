use chrono::Utc;
use serde::Deserialize;
use serde_json::{value::RawValue, Value};

use crate::{
	crypto::verify,
	error::{ApiError, ApiResult},
};

/// How long a signed request stays valid, so an old signature cannot be replayed later.
const VALID_FOR_MS: i64 = 10 * 60 * 1000;

/// A change an account signed with its wallet instead of a session: the frontend signs
/// `JSON.stringify(entity)`, so the entity is kept exactly as sent to check the signature against.
#[derive(Deserialize)]
pub struct SignedEntity {
	entity: Box<RawValue>,
	pub address: String,
	signature: String,
}

impl SignedEntity {
	/// The entity, once its signature, action and age check out.
	pub fn verify(&self, action: &str) -> ApiResult<Value> {
		if !verify(&self.address, self.entity.get(), &self.signature) {
			return Err(ApiError::bad_request("Invalid signature"));
		}
		let entity = serde_json::from_str::<Value>(self.entity.get())
			.map_err(|_| ApiError::bad_request("Invalid request"))?;
		if entity["action"] != action {
			return Err(ApiError::bad_request("The signed action does not match this request"));
		}
		// Signing as a proxy would need the proxy checked on chain, which is not supported yet
		if !entity["real"].is_null() {
			return Err(ApiError::bad_request("Signing for a proxied account is not supported yet"));
		}
		let age = Utc::now().timestamp_millis() - entity["timestamp"].as_i64().unwrap_or_default();
		if !(0..VALID_FOR_MS).contains(&age.abs()) {
			return Err(ApiError::bad_request(
				"The signed request has expired, please sign it again",
			));
		}
		Ok(entity)
	}
}
