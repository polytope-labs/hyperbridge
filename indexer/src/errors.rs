//! Which errors a batch should fail on. rindexer retries a failed batch forever, so only an
//! error that can go away on a retry may fail one; anything else would stop the chain.

use alloy::transports::{RpcError, TransportError};

/** An error worth failing the batch for: it should go away on a retry. */
#[derive(Debug)]
pub struct Retry(pub String);

impl std::fmt::Display for Retry {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.0)
	}
}

impl std::error::Error for Retry {}

/**
 * Whether an error is transient: a database error, which aborts the batch's transaction, or an
 * RPC that failed to answer. A revert, an undecodable answer or bad data is not: retrying gives
 * the same answer.
 */
pub fn is_transient(err: &anyhow::Error) -> bool {
	fn transport(err: &TransportError) -> bool {
		match err {
			RpcError::ErrorResp(payload) =>
				!(payload.code == 3 || payload.message.to_lowercase().contains("revert")),
			RpcError::UnsupportedFeature(_) | RpcError::LocalUsageError(_) => false,
			_ => true,
		}
	}
	err.chain().any(|cause| {
		if cause.is::<tokio_postgres::Error>() ||
			cause.is::<Retry>() ||
			cause.is::<reqwest::Error>()
		{
			return true;
		}
		if let Some(alloy::contract::Error::TransportError(err)) =
			cause.downcast_ref::<alloy::contract::Error>()
		{
			return transport(err);
		}
		cause.downcast_ref::<TransportError>().is_some_and(transport)
	})
}
