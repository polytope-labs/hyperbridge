//! Which errors a batch should fail on. rindexer retries a failed batch forever, so only an
//! error that can go away on a retry may fail one; anything else would stop the chain.

use alloy::{
	rpc::json_rpc::ErrorPayload,
	transports::{RpcError, TransportError, TransportErrorKind},
};
use anyhow::anyhow;
use serde::Deserialize;

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
 * -32000 with the VM error, reth as "EVM error …", and Polkadot Hub as "failed to run
 * contract".
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
	"gas uint64 overflow",
	"evm error",
	"failed to run contract",
];

/** Messages of a node that couldn't serve a call: it ran out of time, or lacks the block or
 * state. */
const NOT_SERVED: &[&str] = &[
	"timeout",
	"timed out",
	"not found",
	"missing trie node",
	"historical state",
	"state not available",
	"state is not available",
	"unknown block",
	"pruned",
	"unavailable",
	"upstream",
	// The ISMP RPC's answer for a block the node hasn't imported.
	"invalid block number",
	// A Substrate node's answers for state it pruned, and for a block it doesn't have.
	"already discarded",
	"unknownblock",
];

/** JSON-RPC's "method not found": the node doesn't serve the method. */
const METHOD_NOT_FOUND: i64 = -32601;

/** JSON-RPC's "internal error": the node failed, not the call. */
const INTERNAL_ERROR: i64 = -32603;

/** Whether a node's error answer says the execution failed. */
fn failed_execution(answer: &ErrorPayload) -> bool {
	let message = answer.message.to_lowercase();
	answer.code == 3 || EXECUTION_FAILURES.iter().any(|failure| message.contains(failure))
}

/** Whether a transport error is a node's answer that the execution failed. */
pub fn is_failed_execution(err: &TransportError) -> bool {
	error_answer(err).is_some_and(|answer| failed_execution(&answer))
}

/**
 * Whether a node's error answer says it couldn't serve the call: it is rate-limited or busy, or
 * lacks the block or state. Another RPC, or a retry, can answer it. A failed execution never
 * counts, as its revert reason is the contract's own text.
 */
pub fn not_served(answer: &ErrorPayload) -> bool {
	let message = answer.message.to_lowercase();
	!failed_execution(answer) &&
		(answer.is_retry_err() ||
			answer.code == INTERNAL_ERROR ||
			NOT_SERVED.iter().any(|reason| message.contains(reason)))
}

/** Whether the node answered, with a JSON-RPC error or an HTTP error, rather than not at all. */
pub fn is_answer(err: &TransportError) -> bool {
	matches!(err, RpcError::ErrorResp(_) | RpcError::Transport(TransportErrorKind::HttpError(_)))
}

/**
 * Whether an error is transient: a database error, or an RPC that failed to answer. A failed
 * execution, an undecodable answer or bad data is not: retrying gives the same answer. Nor is
 * a database data exception (SQLSTATE class 22), such as a NUL in text or a number out of
 * range, since the same values fail again.
 */
pub fn is_transient(err: &anyhow::Error) -> bool {
	fn transport(err: &TransportError) -> bool {
		match err {
			RpcError::ErrorResp(answer) =>
				!(failed_execution(answer) || answer.code == METHOD_NOT_FOUND),
			RpcError::UnsupportedFeature(_) | RpcError::LocalUsageError(_) => false,
			_ => true,
		}
	}
	err.chain().any(|cause| {
		if let Some(db) = cause.downcast_ref::<tokio_postgres::Error>() {
			return !db.code().is_some_and(|code| code.code().starts_with("22"));
		}
		if cause.is::<Retry>() || cause.is::<reqwest::Error>() {
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

/** The node's error answer in `err`: a JSON-RPC error, or one sent with a 4xx status. */
fn error_answer(err: &TransportError) -> Option<ErrorPayload> {
	#[derive(Deserialize)]
	struct Response {
		error: ErrorPayload,
	}
	match err {
		RpcError::ErrorResp(answer) => Some(answer.clone()),
		RpcError::Transport(TransportErrorKind::HttpError(http))
			if (400..500).contains(&http.status) && !matches!(http.status, 408 | 429) =>
			serde_json::from_str::<Response>(&http.body).ok().map(|response| response.error),
		_ => None,
	}
}

/** The node's error answer to a contract call: see [`error_answer`]. */
fn answer(err: &alloy::contract::Error) -> Option<ErrorPayload> {
	let alloy::contract::Error::TransportError(err) = err else { return None };
	error_answer(err)
}

/**
 * The error of a call into code anyone can deploy, such as a token's `symbol()`. When the node
 * answers with an error, the call ran, and it fails again on a retry. Clients word failures
 * their own way, and a revert reason is the contract's own text, so a list of failures can't
 * be complete. Instead, an answer is final unless it says the node couldn't serve the call.
 * The caller caps the call's gas well below what the node's time limit allows, so a timeout
 * is the node's, not the code's.
 */
pub fn untrusted_call_error(err: alloy::contract::Error) -> anyhow::Error {
	if answer(&err).is_some_and(|answer| !not_served(&answer)) {
		anyhow!("{err}")
	} else {
		err.into()
	}
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
			(-32000, "gas uint64 overflow"),
			(-32000, "EVM error InvalidFEOpcode"),
			(-32000, "failed to run contract: InvalidInstruction"),
			(-32601, "the method debug_traceTransaction does not exist/is not available"),
		] {
			assert!(!is_transient(&node_error(code, message)), "{code} {message}");
		}
	}

	#[test]
	fn unanswered_calls_are_transient() {
		for (code, message) in [
			(-32000, "header not found"),
			// A busy node times out calls to trusted contracts, which never run that long.
			(-32000, "execution aborted (timeout = 5s)"),
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
	fn token_code_failures_are_final_unless_the_node_could_not_serve() {
		let call = |err: TransportError| is_transient(&untrusted_call_error(err.into()));
		let answer = |code: i64, message: &str| {
			TransportError::ErrorResp(
				serde_json::from_value(json!({ "code": code, "message": message })).unwrap(),
			)
		};
		for (code, message) in [
			(3, "execution reverted"),
			// A revert reason is the token's own text.
			(-32000, "execution reverted: rate limit exceeded"),
			// A precompile called as a token, and wording no list knows.
			(-32000, "invalid input length"),
			(-32000, "failed to run contract: ContractTrapped"),
			// Polkadot Hub's answer for a precompile called as a token.
			(-32000, "failed to run contract: Other(\"invalid input length\")"),
			(-32000, "execution reverted: pool not found"),
		] {
			assert!(!call(answer(code, message)), "{code} {message}");
		}
		let reverted =
			r#"{"jsonrpc":"2.0","id":1,"error":{"code":3,"message":"execution reverted"}}"#;
		assert!(!call(TransportErrorKind::http_error(400, reverted.into())));

		for (code, message) in [
			(-32000, "header not found"),
			(-32000, "Header not found"),
			(-32000, "block 0x1f2e not found"),
			(-32000, "missing trie node 1a2b (path )"),
			(-32000, "required historical state unavailable"),
			(-32603, "upstream connect error"),
			(-32000, "request timed out"),
			(-32000, "execution aborted (timeout = 5s)"),
			(-32603, "internal error"),
			(429, "Too Many Requests"),
		] {
			assert!(call(answer(code, message)), "{code} {message}");
		}
		assert!(call(TransportErrorKind::http_error(503, "unavailable".into())));
		assert!(call(TransportErrorKind::http_error(400, "<html>bad gateway</html>".into())));
		assert!(call(TransportErrorKind::custom_str("request timed out")));
	}

	#[test]
	fn missing_code_and_bad_answers_are_not_transient() {
		let zero_data =
			ContractError::ZeroData("symbol".into(), AbiError::Other("empty".into()).into());
		assert!(!is_transient(&anyhow::Error::new(zero_data).context("token symbol()")));
		let undecodable: anyhow::Error = ContractError::from(AbiError::Other("bad".into())).into();
		assert!(!is_transient(&undecodable));
	}
}
