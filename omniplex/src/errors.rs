//! Which errors a batch should fail on. rindexer retries a failed batch forever, so only an
//! error that can go away on a retry may fail one; anything else would stop the chain.

use alloy::{
	rpc::json_rpc::ErrorPayload,
	transports::{HttpError, RpcError, TransportError, TransportErrorKind},
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

/**
 * Messages of a node that couldn't serve a call: it is busy or ran out of time, or lacks the
 * block or state. In lower case, as providers vary the case and alloy's own list matches it
 * exactly.
 */
const NOT_SERVED: &[&str] = &[
	"rate limit",
	"rate exceeded",
	"too many requests",
	"credits limited",
	"request limit",
	"concurrent requests",
	"quota",
	"busy",
	"deadline exceeded",
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
	// The ISMP RPC's answers when the node's runtime call for the lookup failed.
	"error fetching requests",
	"error fetching responses",
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
 * Whether a node's error answer says it doesn't serve the method. Another node may, but this
 * one answers the same on every retry.
 */
pub fn lacks_method(answer: &ErrorPayload) -> bool {
	answer.code == METHOD_NOT_FOUND
}

/** Whether a transport error is a node's answer that it [`lacks_method`]. */
pub fn is_missing_method(err: &TransportError) -> bool {
	error_answer(err).is_some_and(|answer| lacks_method(&answer))
}

/**
 * Whether a node's error answer says it couldn't serve the call: it is rate-limited or busy, or
 * lacks the block or state. Another RPC, or a retry, can answer it. A failed execution never
 * counts, as its revert reason is the contract's own text.
 */
pub fn not_served(answer: &ErrorPayload) -> bool {
	let message = answer.message.to_lowercase();
	!failed_execution(answer) &&
		!lacks_method(answer) &&
		(answer.is_retry_err() ||
			answer.code == INTERNAL_ERROR ||
			NOT_SERVED.iter().any(|reason| message.contains(reason)))
}

/** Whether the node answered, with a JSON-RPC error or an HTTP error, rather than not at all. */
pub fn is_answer(err: &TransportError) -> bool {
	matches!(err, RpcError::ErrorResp(_) | RpcError::Transport(TransportErrorKind::HttpError(_)))
}

/**
 * SQLSTATE classes whose errors repeat for the same values: data exceptions (22), such as a NUL
 * in text or a number out of range; integrity constraint violations (23), such as a NOT NULL
 * or unique violation; and program limits (54), such as a value too long for an index.
 */
const LASTING_DATABASE_ERRORS: &[&str] = &["22", "23", "54"];

/**
 * How tokio-postgres starts the errors it raises itself, with no SQLSTATE, when a statement
 * doesn't fit its values or its rows: a parameter or column of another type, a missing column,
 * the wrong number of parameters or rows, or a value too large to send. The same statement
 * fails the same way again.
 */
const LASTING_CLIENT_ERRORS: &[&str] = &[
	"error serializing parameter",
	"error deserializing column",
	"invalid column",
	"query returned an unexpected number of",
	// "expected 2 parameters but got 1"
	"expected ",
	"error encoding message to server",
];

/** Whether a database error repeats on a retry, by its SQLSTATE or, lacking one, its wording. */
fn is_lasting_database_error(err: &tokio_postgres::Error) -> bool {
	match err.code() {
		Some(code) => LASTING_DATABASE_ERRORS.iter().any(|class| code.code().starts_with(class)),
		None => {
			let message = err.to_string();
			LASTING_CLIENT_ERRORS.iter().any(|start| message.starts_with(start))
		},
	}
}

/**
 * Whether an error is transient: a database error, or an RPC that failed to answer. A failed
 * execution, a method the node lacks, a contract's answer that doesn't ABI-decode or bad data
 * is not: retrying gives the same answer. Nor is a database error that
 * [`is_lasting_database_error`], since the same statement fails again.
 */
pub fn is_transient(err: &anyhow::Error) -> bool {
	fn transport(err: &TransportError) -> bool {
		match error_answer(err) {
			Some(answer) => !(failed_execution(&answer) || lacks_method(&answer)),
			None => !matches!(err, RpcError::UnsupportedFeature(_) | RpcError::LocalUsageError(_)),
		}
	}
	err.chain().any(|cause| {
		if let Some(db) = cause.downcast_ref::<tokio_postgres::Error>() {
			return !is_lasting_database_error(db);
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

/** Whether `err` came from the database. A failed statement aborts its transaction. */
pub fn is_database_error(err: &anyhow::Error) -> bool {
	err.chain().any(|cause| cause.is::<tokio_postgres::Error>())
}

/**
 * Whether `err` is Postgres refusing a statement because an earlier one in its transaction
 * failed (25P02). It says nothing of that failure.
 */
pub fn is_aborted_transaction(err: &anyhow::Error) -> bool {
	err.chain().any(|cause| {
		let code = cause.downcast_ref::<tokio_postgres::Error>().and_then(|err| err.code());
		code.is_some_and(|code| code.code() == "25P02")
	})
}

/**
 * The outcomes of work that ran concurrently on one transaction, or its first error that isn't
 * [`is_aborted_transaction`]. Once a statement fails, the others only fail with 25P02, which a
 * retry can't fix even when it counts as transient.
 */
pub fn first_failure<T>(outcomes: Vec<anyhow::Result<T>>) -> anyhow::Result<Vec<T>> {
	let mut values = Vec::with_capacity(outcomes.len());
	let mut failure: Option<anyhow::Error> = None;
	for outcome in outcomes {
		match outcome {
			Ok(value) => values.push(value),
			Err(err) => {
				let keep = match &failure {
					None => true,
					Some(kept) => is_aborted_transaction(kept) && !is_aborted_transaction(&err),
				};
				if keep {
					failure = Some(err);
				}
			},
		}
	}
	failure.map_or(Ok(values), Err)
}

/**
 * The HTTP error in `err` when its status refuses the request itself: a 4xx other than a
 * timeout (408) or a rate limit (429).
 */
fn refusal(err: &TransportError) -> Option<&HttpError> {
	match err {
		RpcError::Transport(TransportErrorKind::HttpError(http))
			if (400..500).contains(&http.status) && !matches!(http.status, 408 | 429) =>
			Some(http),
		_ => None,
	}
}

/** The node's error answer in `err`: a JSON-RPC error, or one sent with a [`refusal`]. */
fn error_answer(err: &TransportError) -> Option<ErrorPayload> {
	#[derive(Deserialize)]
	struct Response {
		error: ErrorPayload,
	}
	match err {
		RpcError::ErrorResp(answer) => Some(answer.clone()),
		_ => serde_json::from_str::<Response>(&refusal(err)?.body)
			.ok()
			.map(|response| response.error),
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
	match answer(&err) {
		Some(answer) if !not_served(&answer) => anyhow!("{err}"),
		_ => err.into(),
	}
}

/**
 * The error of a request many RPCs refuse, each in its own words and code, such as a trace on a
 * plan without it. As for [`untrusted_call_error`], an answer is final unless it says the node
 * couldn't serve the request. So is a [`refusal`] with no answer in it, such as a plain 403.
 */
pub fn refused_request_error(err: TransportError) -> anyhow::Error {
	let refused = match error_answer(&err) {
		Some(answer) => !not_served(&answer),
		None => refusal(&err).is_some(),
	};
	if refused {
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
		// The same answers are final for a trusted contract's call too.
		let trusted = |status, body: &str| {
			let err = TransportErrorKind::http_error(status, body.into());
			is_transient(&ContractError::TransportError(err).into())
		};
		assert!(!trusted(400, reverted));
		let lacking =
			r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
		assert!(!trusted(403, lacking));
		assert!(trusted(429, reverted));

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
			// Rate limits in wording or case alloy's own list doesn't match.
			(-32000, "Too Many Requests"),
			(-32000, "Rate limit exceeded"),
			(-32000, "daily quota exceeded"),
			(-32009, "Server is busy, try again later"),
			(-32000, "context deadline exceeded"),
		] {
			assert!(call(answer(code, message)), "{code} {message}");
		}
		assert!(call(TransportErrorKind::http_error(503, "unavailable".into())));
		assert!(call(TransportErrorKind::http_error(400, "<html>bad gateway</html>".into())));
		assert!(call(TransportErrorKind::custom_str("request timed out")));
	}

	#[test]
	fn a_refused_request_is_final_unless_the_node_could_not_serve() {
		let request = |err: TransportError| is_transient(&refused_request_error(err));
		let answer = |code: i64, message: &str| {
			TransportError::ErrorResp(
				serde_json::from_value(json!({ "code": code, "message": message })).unwrap(),
			)
		};
		// A plan without traces, in the words of three providers, and a tracer the node lacks.
		for (code, message) in [
			(-32600, "debug_traceTransaction is not available on the Free tier"),
			(35, "method is not available on freetier"),
			(-32601, "the method debug_traceTransaction does not exist/is not available"),
			(-32000, "tracer not registered: callTracer"),
		] {
			assert!(!request(answer(code, message)), "{code} {message}");
		}
		let refused = r#"{"error":{"code":-32053,"message":"API key is not allowed to trace"}}"#;
		assert!(!request(TransportErrorKind::http_error(403, refused.into())));
		assert!(!request(TransportErrorKind::http_error(403, "Forbidden".into())));

		for (code, message) in [
			(-32000, "transaction not found"),
			(-32000, "historical state unavailable"),
			(-32005, "rate limit exceeded"),
			(-32603, "internal error"),
		] {
			assert!(request(answer(code, message)), "{code} {message}");
		}
		assert!(request(TransportErrorKind::http_error(429, "Too Many Requests".into())));
		assert!(request(TransportErrorKind::http_error(503, "unavailable".into())));
		assert!(request(TransportErrorKind::custom_str("request timed out")));
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
