//! `handler.helpers.ts`: the ISMP messages a delivery transaction's calldata carries, so the
//! transaction's transfers can be attributed to the modules those messages concern.

use alloy::sol_types::SolInterface;

alloy::sol!(
	#[sol(all_derives)]
	HandlerV2,
	"abis/HandlerV2.abi.json"
);

use HandlerV2::HandlerV2Calls;

/** The handler functions whose messages name modules. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageFunction {
	/** The recipients of delivered POST requests. */
	HandlePostRequests,
	/** The senders of answered GET requests. */
	HandleGetResponses,
	/** The senders of timed-out POST requests. */
	HandlePostRequestTimeouts,
	/** The senders of timed-out GET requests. */
	HandleGetRequestTimeouts,
}

/**
 * `decodeHandlerCalls`: the handler calls in `calldata`, in order, with every `batchCall`
 * unwrapped. Calldata that is no handler call is no calls.
 */
pub fn decode_handler_calls(calldata: &[u8]) -> Vec<HandlerV2Calls> {
	match HandlerV2Calls::abi_decode(calldata) {
		Ok(HandlerV2Calls::batchCall(batch)) =>
			batch.calls.iter().flat_map(|inner| decode_handler_calls(inner)).collect(),
		Ok(call) => vec![call],
		Err(_) => vec![],
	}
}

/**
 * `getHandlerMessageModules`: the modules every `function` call names, as lowercase hex.
 * Never fails; unreadable calldata names none.
 */
pub fn handler_message_modules(calldata: &[u8], function: MessageFunction) -> Vec<String> {
	if calldata.is_empty() {
		return vec![];
	}
	let calls = decode_handler_calls(calldata);
	if calls.is_empty() {
		let selector = alloy::primitives::hex::encode_prefixed(&calldata[..calldata.len().min(4)]);
		tracing::warn!("[handler] Calldata with selector {selector} is not a known handler call");
	}
	let mut modules = vec![];
	for call in calls {
		match (function, call) {
			(MessageFunction::HandlePostRequests, HandlerV2Calls::handlePostRequests(call)) =>
				modules.extend(call.request.requests.iter().map(|leaf| leaf.request.to.to_string())),
			(MessageFunction::HandleGetResponses, HandlerV2Calls::handleGetResponses(call)) =>
				modules.extend(
					call.message
						.responses
						.iter()
						.map(|leaf| leaf.response.request.from.to_string()),
				),
			(
				MessageFunction::HandlePostRequestTimeouts,
				HandlerV2Calls::handlePostRequestTimeouts(call),
			) =>
				modules.extend(call.message.timeouts.iter().map(|request| request.from.to_string())),
			(
				MessageFunction::HandleGetRequestTimeouts,
				HandlerV2Calls::handleGetRequestTimeouts(call),
			) =>
				modules.extend(call.message.timeouts.iter().map(|request| request.from.to_string())),
			_ => {},
		}
	}
	modules
}

#[cfg(test)]
mod tests {
	use super::{HandlerV2::*, *};
	use alloy::{
		primitives::{Address, Bytes, B256, U256},
		sol_types::SolCall,
	};

	const HOST: &str = "0x620128e2b19193d6bd244a3ac8d3bba0541b19c3";
	const GATEWAY: &str = "0xae041f7b0cb581876832830baeb6a2aa2a3c9716";
	const OTHER_MODULE: &str = "0x1111111111111111111111111111111111111111";

	fn bytes(hex: &str) -> Bytes {
		hex.parse().unwrap()
	}

	fn height() -> StateMachineHeight {
		StateMachineHeight { stateMachineId: U256::from(3367), height: U256::from(100) }
	}

	fn proof() -> Proof {
		Proof {
			height: height(),
			multiproof: vec![B256::repeat_byte(0xab)],
			leafCount: U256::from(9),
		}
	}

	fn post_request(to: &str, from: &str) -> PostRequest {
		PostRequest {
			source: bytes("0x504f4c4b41444f542d33333637"),
			dest: bytes("0x45564d2d313337"),
			nonce: 7,
			from: bytes(from),
			to: bytes(to),
			timeoutTimestamp: 0,
			body: bytes("0x1234"),
		}
	}

	fn get_request(from: &str) -> GetRequest {
		GetRequest {
			source: bytes("0x45564d2d313337"),
			dest: bytes("0x504f4c4b41444f542d33333637"),
			nonce: 7,
			from: bytes(from),
			timeoutTimestamp: 0,
			keys: vec![bytes("0x01")],
			height: 5,
			context: bytes("0x"),
		}
	}

	fn host() -> Address {
		HOST.parse().unwrap()
	}

	fn consensus() -> Vec<u8> {
		handleConsensusCall { host: host(), proof: bytes("0xdeadbeef") }.abi_encode()
	}

	fn post_requests(to: &[&str]) -> Vec<u8> {
		handlePostRequestsCall {
			host: host(),
			request: PostRequestMessage {
				proof: proof(),
				requests: to
					.iter()
					.enumerate()
					.map(|(index, module)| PostRequestLeaf {
						request: post_request(module, OTHER_MODULE),
						index: U256::from(index),
					})
					.collect(),
			},
		}
		.abi_encode()
	}

	fn batch(calls: Vec<Vec<u8>>) -> Vec<u8> {
		batchCallCall { calls: calls.into_iter().map(Bytes::from).collect() }.abi_encode()
	}

	fn names(calldata: &[u8]) -> Vec<&'static str> {
		decode_handler_calls(calldata)
			.iter()
			.map(|call| match call {
				HandlerV2Calls::handleConsensus(_) => "handleConsensus",
				HandlerV2Calls::handlePostRequests(_) => "handlePostRequests",
				HandlerV2Calls::handleGetResponses(_) => "handleGetResponses",
				HandlerV2Calls::handlePostRequestTimeouts(_) => "handlePostRequestTimeouts",
				HandlerV2Calls::handleGetRequestTimeouts(_) => "handleGetRequestTimeouts",
				_ => "other",
			})
			.collect()
	}

	fn transfer_calldata() -> Vec<u8> {
		let mut data = bytes("0xa9059cbb").to_vec();
		data.extend([0u8; 64]);
		data
	}

	#[test]
	fn reads_a_direct_call() {
		assert_eq!(names(&consensus()), ["handleConsensus"]);
	}

	#[test]
	fn unwraps_a_batch_in_order() {
		let batch = batch(vec![consensus(), post_requests(&[GATEWAY])]);
		assert_eq!(alloy::primitives::hex::encode_prefixed(&batch[..4]), "0x68be3cf2");
		assert_eq!(names(&batch), ["handleConsensus", "handlePostRequests"]);
	}

	#[test]
	fn unwraps_nested_batches() {
		let inner = batch(vec![post_requests(&[GATEWAY])]);
		let outer = batch(vec![consensus(), inner]);
		assert_eq!(names(&outer), ["handleConsensus", "handlePostRequests"]);
	}

	#[test]
	fn other_calldata_is_no_calls() {
		assert!(names(&[]).is_empty());
		assert!(names(&transfer_calldata()).is_empty());
	}

	#[test]
	fn skips_an_unreadable_call_in_a_batch() {
		let batch = batch(vec![bytes("0xa9059cbb").to_vec(), post_requests(&[GATEWAY])]);
		assert_eq!(names(&batch), ["handlePostRequests"]);
	}

	#[test]
	fn names_post_recipients_batched_direct_and_across_calls() {
		let batched = batch(vec![consensus(), post_requests(&[GATEWAY, OTHER_MODULE])]);
		assert_eq!(
			handler_message_modules(&batched, MessageFunction::HandlePostRequests),
			[GATEWAY, OTHER_MODULE]
		);
		let across = batch(vec![post_requests(&[GATEWAY]), post_requests(&[OTHER_MODULE])]);
		assert_eq!(
			handler_message_modules(&across, MessageFunction::HandlePostRequests),
			[GATEWAY, OTHER_MODULE]
		);
		let direct = post_requests(&[GATEWAY]);
		assert_eq!(alloy::primitives::hex::encode_prefixed(&direct[..4]), "0x698b1267");
		assert_eq!(
			handler_message_modules(&direct, MessageFunction::HandlePostRequests),
			[GATEWAY]
		);
	}

	#[test]
	fn names_get_response_senders() {
		let responses = handleGetResponsesCall {
			host: host(),
			message: GetResponseMessage {
				proof: proof(),
				responses: vec![GetResponseLeaf {
					response: GetResponse {
						request: get_request(GATEWAY),
						values: vec![StorageValue { key: bytes("0x01"), value: bytes("0x02") }],
					},
					index: U256::ZERO,
				}],
			},
		}
		.abi_encode();
		let batched = batch(vec![consensus(), responses.clone()]);
		for calldata in [responses, batched] {
			assert_eq!(
				handler_message_modules(&calldata, MessageFunction::HandleGetResponses),
				[GATEWAY]
			);
		}
	}

	#[test]
	fn names_timed_out_senders() {
		let post_timeouts = handlePostRequestTimeoutsCall {
			host: host(),
			message: PostRequestTimeoutMessage {
				timeouts: vec![post_request(OTHER_MODULE, GATEWAY)],
				height: height(),
				proof: vec![bytes("0x01")],
			},
		}
		.abi_encode();
		let get_timeouts = handleGetRequestTimeoutsCall {
			host: host(),
			message: GetTimeoutMessage {
				timeouts: vec![get_request(GATEWAY)],
				height: height(),
				proof: vec![bytes("0x01")],
			},
		}
		.abi_encode();
		let batched = batch(vec![consensus(), post_timeouts, get_timeouts]);
		assert_eq!(
			handler_message_modules(&batched, MessageFunction::HandlePostRequestTimeouts),
			[GATEWAY]
		);
		assert_eq!(
			handler_message_modules(&batched, MessageFunction::HandleGetRequestTimeouts),
			[GATEWAY]
		);
	}

	#[test]
	fn other_kinds_and_unknown_calldata_name_nothing() {
		assert!(
			handler_message_modules(&consensus(), MessageFunction::HandlePostRequests).is_empty()
		);
		assert!(handler_message_modules(
			&post_requests(&[GATEWAY]),
			MessageFunction::HandleGetResponses
		)
		.is_empty());
		assert!(handler_message_modules(&transfer_calldata(), MessageFunction::HandlePostRequests)
			.is_empty());
		assert!(handler_message_modules(&[], MessageFunction::HandlePostRequests).is_empty());
	}
}
