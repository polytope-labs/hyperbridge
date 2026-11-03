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
 * Messages a node answers an `eth_call` with when the execution itself failed, which it does
 * again on every retry. Only a revert has its own code (3). geth sends the other halts as
 * -32000 with the VM error, and reth as "EVM error …".
 */
const EXECUTION_FAILURES: &[&str] = &[
	"revert",
	"invalid opcode",
	"out of gas",
	"invalid jump",
	"stack underflow",
	"stack limit",
	"write protection",
	"return data out of bounds",
	"max call depth",
	"execution aborted",
	"evm error",
];

/** JSON-RPC's "method not found": the node doesn't serve the method. */
const METHOD_NOT_FOUND: i64 = -32601;

/**
 * Whether an error is transient: a database error, which aborts the batch's transaction, or an
 * RPC that failed to answer. A failed execution, an undecodable answer or bad data is not:
 * retrying gives the same answer.
 */
pub fn is_transient(err: &anyhow::Error) -> bool {
	fn transport(err: &TransportError) -> bool {
		match err {
			RpcError::ErrorResp(payload) => {
				let message = payload.message.to_lowercase();
				let deterministic = payload.code == 3 ||
					payload.code == METHOD_NOT_FOUND ||
					EXECUTION_FAILURES.iter().any(|failure| message.contains(failure));
				!deterministic
			},
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

#[cfg(test)]
mod tests {
	use alloy::{
		contract::Error as ContractError,
		sol_types::Error as AbiError,
		transports::{TransportError, TransportErrorKind},
	};
	use serde_json::json;

	use super::*;

	fn node_error(code: i64, message: &str) -> anyhow::Error {
		let payload = serde_json::from_value(json!({ "code": code, "message": message })).unwrap();
		ContractError::TransportError(TransportError::ErrorResp(payload)).into()
	}

	#[test]
	fn failed_executions_are_not_transient() {
		for (code, message) in [
			(3, "execution reverted"),
			(-32000, "execution reverted: ERC20: no symbol"),
			(-32000, "invalid opcode: INVALID"),
			(-32000, "invalid opcode: opcode 0xfe not defined"),
			(-32000, "out of gas"),
			(-32000, "invalid jump destination"),
			(-32000, "stack underflow (0 <=> 1)"),
			(-32000, "execution aborted (timeout = 5s)"),
			(-32000, "EVM error InvalidFEOpcode"),
			(-32601, "the method debug_traceTransaction does not exist/is not available"),
		] {
			assert!(!is_transient(&node_error(code, message)), "{code} {message}");
		}
	}

	#[test]
	fn unanswered_calls_are_transient() {
		for (code, message) in [
			(-32000, "header not found"),
			(-32005, "rate limit exceeded"),
			(-32603, "internal error"),
			(429, "Too Many Requests"),
		] {
			assert!(is_transient(&node_error(code, message)), "{code} {message}");
		}
		let timeout: anyhow::Error =
			ContractError::TransportError(TransportErrorKind::custom_str("request timed out"))
				.into();
		assert!(is_transient(&timeout));
		assert!(is_transient(&anyhow::Error::new(Retry("no receipt".into()))));
	}

	#[test]
	fn missing_code_and_bad_answers_are_not_transient() {
		let zero_data = ContractError::ZeroData("symbol".into(), AbiError::Other("empty".into()).into());
		assert!(!is_transient(&anyhow::Error::new(zero_data).context("token symbol()")));
		let undecodable: anyhow::Error = ContractError::from(AbiError::Other("bad".into())).into();
		assert!(!is_transient(&undecodable));
	}
}
