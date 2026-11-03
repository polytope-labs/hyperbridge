//! What the gateway's logs leave out: the placed or filled order from the transaction's calldata
//! (directly or through a call trace), and the receipt's user-operation and transfer logs.

use alloy::{
	primitives::{keccak256, Address, Bytes, B256, U256},
	providers::Provider,
	sol,
	sol_types::{SolCall, SolValue},
};
use anyhow::{anyhow, Context};
use serde_json::{json, Value};

use crate::chains::Chain;

sol! {
	#[sol(all_derives)]
	/** The gateway's order and the calls that carry one, including fill shapes deployed before
	 * `FillOptions` gained `validUntil` and `inputs`. Selectors are pinned by the tests. */
	interface Calls {
		struct TokenInfo { bytes32 token; uint256 amount; }
		struct DispatchInfo { TokenInfo[] assets; bytes call; }
		struct PaymentInfo { bytes32 beneficiary; TokenInfo[] assets; bytes call; }
		struct Order {
			bytes32 user;
			bytes source;
			bytes destination;
			uint256 deadline;
			uint256 nonce;
			uint256 fees;
			address session;
			DispatchInfo predispatch;
			TokenInfo[] inputs;
			PaymentInfo output;
		}
		struct FillOptions { uint256 relayerFee; uint256 nativeDispatchFee; uint256 validUntil; TokenInfo[] outputs; TokenInfo[] inputs; }
		struct FillOptionsWithoutInputs { uint256 relayerFee; uint256 nativeDispatchFee; uint256 validUntil; TokenInfo[] outputs; }
		struct FillOptionsWithoutValidUntil { uint256 relayerFee; uint256 nativeDispatchFee; TokenInfo[] outputs; }

		function placeOrder(Order order, bytes32 graffiti) external payable;
		function fillOrder(Order order, FillOptions options) external payable;
		function fillOrder(Order order, FillOptionsWithoutInputs options) external payable;
		function fillOrder(Order order, FillOptionsWithoutValidUntil options) external payable;
	}
}

pub use Calls::{Order as SolOrder, TokenInfo};

/** One `(token, amount)` of an order or event, `token` as bytes32. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenAmount {
	pub token: B256,
	pub amount: U256,
}

impl TokenAmount {
	/** The token's 20-byte address: the last 20 bytes of its bytes32 form. */
	pub fn address(&self) -> Address {
		Address::from_slice(&self.token[12..])
	}
}

/** An order as the TypeScript built it from an `OrderPlaced` log. */
#[derive(Debug, Clone)]
pub struct Order {
	pub user: B256,
	pub source: String,
	pub destination: String,
	pub deadline: U256,
	pub nonce: U256,
	pub fees: U256,
	pub session: Address,
	pub predispatch: Vec<TokenAmount>,
	pub predispatch_call: Bytes,
	pub inputs: Vec<TokenAmount>,
	pub beneficiary: B256,
	pub outputs: Vec<TokenAmount>,
	pub output_call: Bytes,
}

fn sol_tokens(tokens: &[TokenAmount]) -> Vec<TokenInfo> {
	tokens.iter().map(|t| TokenInfo { token: t.token, amount: t.amount }).collect()
}

/** `computeOrderCommitment`'s `toBytes`: hex stays hex, anything else is its UTF-8 bytes. */
fn chain_bytes(value: &str) -> anyhow::Result<Bytes> {
	match value.strip_prefix("0x") {
		Some(hex) => Ok(hex::decode(hex).with_context(|| format!("chain id {value:?}"))?.into()),
		None => Ok(Bytes::copy_from_slice(value.as_bytes())),
	}
}

impl Order {
	pub fn to_sol(&self) -> anyhow::Result<SolOrder> {
		Ok(SolOrder {
			user: self.user,
			source: chain_bytes(&self.source)?,
			destination: chain_bytes(&self.destination)?,
			deadline: self.deadline,
			nonce: self.nonce,
			fees: self.fees,
			session: self.session,
			predispatch: Calls::DispatchInfo {
				assets: sol_tokens(&self.predispatch),
				call: self.predispatch_call.clone(),
			},
			inputs: sol_tokens(&self.inputs),
			output: Calls::PaymentInfo {
				beneficiary: self.beneficiary,
				assets: sol_tokens(&self.outputs),
				call: self.output_call.clone(),
			},
		})
	}

	/** `keccak256(abi.encode(order))`, as the gateway computes it. */
	pub fn commitment(&self) -> anyhow::Result<B256> {
		Ok(commitment(&self.to_sol()?))
	}
}

pub fn commitment(order: &SolOrder) -> B256 {
	keccak256(order.abi_encode())
}

/** `decodePlaceOrder`: the order and graffiti of a `placeOrder` call, or `None`. */
pub fn decode_place_order(calldata: &[u8]) -> Option<(SolOrder, B256)> {
	let call = Calls::placeOrderCall::abi_decode(calldata).ok()?;
	Some((call.order, call.graffiti))
}

/** `tryDecodeFillOrder`: the order of a `fillOrder` call in any deployed shape, or `None`. */
pub fn decode_fill_order(calldata: &[u8]) -> Option<SolOrder> {
	let selector: [u8; 4] = calldata.get(..4)?.try_into().ok()?;
	match selector {
		Calls::fillOrder_0Call::SELECTOR =>
			Calls::fillOrder_0Call::abi_decode(calldata).ok().map(|c| c.order),
		Calls::fillOrder_1Call::SELECTOR =>
			Calls::fillOrder_1Call::abi_decode(calldata).ok().map(|c| c.order),
		Calls::fillOrder_2Call::SELECTOR =>
			Calls::fillOrder_2Call::abi_decode(calldata).ok().map(|c| c.order),
		_ => None,
	}
}

/** Hex calldata as a trace or transaction reports it. Invalid hex decodes as nothing. */
pub fn hex_bytes(value: &str) -> Option<Vec<u8>> {
	hex::decode(value.strip_prefix("0x").unwrap_or(value)).ok()
}

/** A receipt log, with the fields the matching below reads. */
#[derive(Debug, Clone)]
pub struct ReceiptLog {
	pub address: Address,
	pub topics: Vec<B256>,
	/** `None` when the data isn't valid hex. */
	pub data: Option<Bytes>,
	pub log_index: u64,
}

impl ReceiptLog {
	fn from_json(log: &Value) -> Option<Self> {
		let address = log.get("address")?.as_str()?.parse().ok()?;
		let topics = log
			.get("topics")?
			.as_array()?
			.iter()
			.map(|t| t.as_str()?.parse().ok())
			.collect::<Option<Vec<B256>>>()?;
		let data = log.get("data").and_then(Value::as_str).and_then(hex_bytes).map(Bytes::from);
		let index = log.get("logIndex")?.as_str()?;
		let log_index = u64::from_str_radix(index.trim_start_matches("0x"), 16).ok()?;
		Some(Self { address, topics, data, log_index })
	}
}

/** The logs of a transaction's receipt. Fails when the receipt is unavailable. */
pub async fn receipt_logs(chain: &Chain, transaction: B256) -> anyhow::Result<Vec<ReceiptLog>> {
	let receipt: Value = chain
		.provider
		.raw_request("eth_getTransactionReceipt".into(), (transaction,))
		.await?;
	let logs = receipt
		.get("logs")
		.and_then(Value::as_array)
		.ok_or_else(|| anyhow!("no receipt for {transaction}"))?;
	Ok(logs.iter().filter_map(ReceiptLog::from_json).collect())
}

/** The input of a transaction, if the RPC returns it. */
pub async fn transaction_input(chain: &Chain, transaction: B256) -> anyhow::Result<Option<String>> {
	let tx: Value = chain
		.provider
		.raw_request("eth_getTransactionByHash".into(), (transaction,))
		.await?;
	Ok(tx.get("input").and_then(Value::as_str).map(str::to_string))
}

/**
 * `getContractCallInputs`: the inputs of every successful nested call to `target`, in call
 * order, from `debug_traceTransaction`. Empty when the transaction calls `target` directly.
 */
pub async fn nested_call_inputs(
	chain: &Chain,
	transaction: B256,
	target: Address,
) -> anyhow::Result<Vec<String>> {
	let trace: Value = chain
		.provider
		.raw_request(
			"debug_traceTransaction".into(),
			(
				transaction,
				json!({ "tracer": "callTracer", "tracerConfig": { "disableCode": true } }),
			),
		)
		.await?;
	if trace.is_null() {
		anyhow::bail!("no trace for {transaction}");
	}
	Ok(trace_call_inputs(&trace, target))
}

/** The trace walk behind [`nested_call_inputs`], on a `callTracer` result. */
pub fn trace_call_inputs(trace: &Value, target: Address) -> Vec<String> {
	if call_target(trace) == Some(target) {
		return vec![];
	}
	let mut inputs = vec![];
	collect_inputs(trace, target, &mut inputs);
	inputs
}

fn call_target(call: &Value) -> Option<Address> {
	call.get("to")?.as_str()?.parse().ok()
}

fn collect_inputs(call: &Value, target: Address, inputs: &mut Vec<String>) {
	let Some(calls) = call.get("calls").and_then(Value::as_array) else { return };
	for nested in calls {
		// A reverted subtree cannot have produced a persisted log.
		if nested.get("error").is_some_and(|e| !e.is_null()) {
			continue;
		}
		if call_target(nested) == Some(target) {
			if let Some(input) = nested.get("input").and_then(Value::as_str) {
				inputs.push(input.to_string());
			}
		}
		collect_inputs(nested, target, inputs);
	}
}

const USER_OPERATION_EVENT: B256 =
	alloy::primitives::b256!("49628fd1471006c1482da88028e9ce4dbb080b815c9b0344d39e5a8e6ec1419f");
const BEFORE_EXECUTION: B256 =
	alloy::primitives::b256!("bb47ee3e183a558b1a2ff0874b079f3fc5478b7454eacf2bfc5af2ff5878f972");
const TRANSFER: B256 =
	alloy::primitives::b256!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");
pub const ORDER_FILLED: B256 =
	alloy::primitives::b256!("dd5ba16ce7d9636800f5875b2a5572176a96f47947d3218b2a2ac4c5cc11f9bf");
pub const PARTIAL_FILL: B256 =
	alloy::primitives::b256!("a71fc5b4fbaf5f5f0846475fec0d0c1d6c93100f2326ddb75c117244f45bbe85");

/** Canonical EntryPoints v0.6 to v0.9. */
const ENTRY_POINTS: [Address; 4] = [
	alloy::primitives::address!("5ff137d4b0fdcd49dca30c7cf57e578a026d2789"),
	alloy::primitives::address!("0000000071727de22e5e9d8baf0edac6f37da032"),
	alloy::primitives::address!("4337084d9e255ff0702461cf8895ce9e3b5ff108"),
	alloy::primitives::address!("433709009b8330fda32311df1c2afa402ed8d009"),
];

fn topic_address(topic: &B256) -> Address {
	Address::from_slice(&topic[12..])
}

/** The first user operation after `log_index`, if `sender` sent it. Never skips to a later one. */
fn find_user_op_event(logs: &[ReceiptLog], sender: Address, log_index: u64) -> Option<&ReceiptLog> {
	let next = logs
		.iter()
		.filter(|log| {
			log.topics.first() == Some(&USER_OPERATION_EVENT) &&
				log.topics.len() == 4 &&
				ENTRY_POINTS.contains(&log.address) &&
				log.log_index > log_index
		})
		.min_by_key(|log| log.log_index)?;
	(topic_address(&next.topics[2]) == sender).then_some(next)
}

/** `findUserOpHash`: the user operation a fill at `log_index` ran in. */
pub fn find_user_op_hash(logs: &[ReceiptLog], sender: Address, log_index: u64) -> Option<B256> {
	find_user_op_event(logs, sender, log_index).map(|log| log.topics[1])
}

/** `resolvePlacementUserOpHash`: the user operation a placement ran in, only if it ran during
 * that operation's execution (after its `BeforeExecution`). */
pub fn placement_user_op_hash(
	logs: &[ReceiptLog],
	sender: Address,
	log_index: u64,
) -> Option<B256> {
	let event = find_user_op_event(logs, sender, log_index)?;
	let start = logs
		.iter()
		.filter(|log| {
			log.address == event.address &&
				log.topics.first() == Some(&BEFORE_EXECUTION) &&
				log.log_index < event.log_index
		})
		.map(|log| log.log_index)
		.max()?;
	(start < log_index).then_some(event.topics[1])
}

/**
 * `matchDeliveryTransfers`: the amount each output's Transfer from the filler to the
 * beneficiary carried, between the previous fill event in the transaction and this one. A
 * native output, or an ambiguous match, is `None`; a zero output is 0.
 */
pub fn match_delivery_transfers(
	logs: &[ReceiptLog],
	gateway: Address,
	log_index: u64,
	filler: Address,
	beneficiary: Address,
	outputs: &[TokenAmount],
) -> Vec<Option<U256>> {
	let boundary = logs
		.iter()
		.filter(|log| {
			log.address == gateway &&
				matches!(log.topics.first(), Some(t) if *t == ORDER_FILLED || *t == PARTIAL_FILL) &&
				log.log_index < log_index
		})
		.map(|log| log.log_index as i64)
		.max()
		.unwrap_or(-1);
	let mut transfers: Vec<&ReceiptLog> = logs
		.iter()
		.filter(|log| {
			log.topics.first() == Some(&TRANSFER) &&
				log.topics.len() == 3 &&
				log.data.as_ref().is_some_and(|d| d.len() == 32) &&
				(log.log_index as i64) > boundary &&
				log.log_index < log_index &&
				topic_address(&log.topics[1]) == filler &&
				topic_address(&log.topics[2]) == beneficiary
		})
		.collect();
	transfers.sort_by_key(|log| log.log_index);

	let expected = |token: Address| {
		outputs.iter().filter(|o| !o.amount.is_zero() && o.address() == token).count()
	};
	let mut consumed: Vec<(Address, usize)> = vec![];
	outputs
		.iter()
		.map(|output| {
			let token = output.address();
			if token == Address::ZERO {
				return None;
			}
			if output.amount.is_zero() {
				return Some(U256::ZERO);
			}
			let group: Vec<&&ReceiptLog> =
				transfers.iter().filter(|t| t.address == token).collect();
			if group.len() != expected(token) {
				return None;
			}
			let next = match consumed.iter_mut().find(|(t, _)| *t == token) {
				Some((_, next)) => next,
				None => {
					consumed.push((token, 0));
					&mut consumed.last_mut().unwrap().1
				},
			};
			let log = group[*next];
			*next += 1;
			Some(U256::from_be_slice(log.data.as_ref().unwrap()))
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::handlers::abi::IntentGatewayV3;

	const GATEWAY: Address =
		alloy::primitives::address!("1111111111111111111111111111111111111111");
	const ENTRY_POINT: Address = ENTRY_POINTS[1];
	const FILLER: Address = alloy::primitives::address!("2222222222222222222222222222222222222222");
	const BENEFICIARY: Address =
		alloy::primitives::address!("3333333333333333333333333333333333333333");
	const TOKEN_A: Address =
		alloy::primitives::address!("4444444444444444444444444444444444444444");
	const TOKEN_B: Address =
		alloy::primitives::address!("5555555555555555555555555555555555555555");
	const USER_OP_HASH: B256 = B256::repeat_byte(0xaa);

	fn pad(address: Address) -> B256 {
		address.into_word()
	}

	fn transfer(
		log_index: u64,
		token: Address,
		from: Address,
		to: Address,
		amount: u64,
	) -> ReceiptLog {
		ReceiptLog {
			address: token,
			topics: vec![TRANSFER, pad(from), pad(to)],
			data: Some(U256::from(amount).to_be_bytes::<32>().to_vec().into()),
			log_index,
		}
	}

	fn user_op(log_index: u64, hash: B256, sender: Address) -> ReceiptLog {
		user_op_at(log_index, hash, sender, ENTRY_POINT)
	}

	fn user_op_at(log_index: u64, hash: B256, sender: Address, entry_point: Address) -> ReceiptLog {
		ReceiptLog {
			address: entry_point,
			topics: vec![USER_OPERATION_EVENT, hash, pad(sender), B256::ZERO],
			data: Some(Bytes::new()),
			log_index,
		}
	}

	fn before_execution(log_index: u64, entry_point: Address) -> ReceiptLog {
		ReceiptLog {
			address: entry_point,
			topics: vec![BEFORE_EXECUTION],
			data: Some(Bytes::new()),
			log_index,
		}
	}

	fn fill_event(log_index: u64) -> ReceiptLog {
		ReceiptLog {
			address: GATEWAY,
			topics: vec![ORDER_FILLED, B256::repeat_byte(0xcc)],
			data: Some(Bytes::new()),
			log_index,
		}
	}

	fn outputs(tokens: &[Address]) -> Vec<TokenAmount> {
		tokens
			.iter()
			.map(|t| TokenAmount { token: pad(*t), amount: U256::from(100) })
			.collect()
	}

	fn amounts(values: &[Option<u64>]) -> Vec<Option<U256>> {
		values.iter().map(|v| v.map(U256::from)).collect()
	}

	fn deliveries(logs: &[ReceiptLog], log_index: u64, outs: &[TokenAmount]) -> Vec<Option<U256>> {
		match_delivery_transfers(logs, GATEWAY, log_index, FILLER, BENEFICIARY, outs)
	}

	#[test]
	fn fill_user_op_is_the_next_operation_by_the_filler() {
		let logs = [
			user_op(2, B256::repeat_byte(0x11), FILLER),
			fill_event(5),
			user_op(9, USER_OP_HASH, FILLER),
			user_op(12, B256::repeat_byte(0x33), FILLER),
		];
		assert_eq!(find_user_op_hash(&logs, FILLER, 5), Some(USER_OP_HASH));
		// A different sender's operation comes first: no attribution.
		let other = [user_op(7, USER_OP_HASH, BENEFICIARY), user_op(9, USER_OP_HASH, FILLER)];
		assert_eq!(find_user_op_hash(&other, FILLER, 5), None);
		// Spoofed events from other contracts are ignored.
		let spoof = user_op_at(6, USER_OP_HASH, FILLER, TOKEN_A);
		assert_eq!(find_user_op_hash(&[spoof.clone()], FILLER, 5), None);
		assert_eq!(
			find_user_op_hash(&[spoof, user_op(9, USER_OP_HASH, FILLER)], FILLER, 5),
			Some(USER_OP_HASH)
		);
		// Plain EOA fills and operations before the fill have none.
		let eoa = [fill_event(5), transfer(3, TOKEN_A, FILLER, BENEFICIARY, 100)];
		assert_eq!(find_user_op_hash(&eoa, FILLER, 5), None);
		assert_eq!(
			find_user_op_hash(&[user_op(2, USER_OP_HASH, FILLER), fill_event(5)], FILLER, 5),
			None
		);
	}

	#[test]
	fn placement_user_op_needs_the_execution_boundary() {
		let account = FILLER;
		let other = BENEFICIARY;
		let hash = B256::repeat_byte(0xab);
		let later = B256::repeat_byte(0xcd);
		let start = |i| before_execution(i, ENTRY_POINTS[2]);
		let op = |i, sender, hash| user_op_at(i, hash, sender, ENTRY_POINTS[2]);
		for entry_point in ENTRY_POINTS {
			let logs =
				[before_execution(0, entry_point), user_op_at(6, hash, account, entry_point)];
			assert_eq!(placement_user_op_hash(&logs, account, 5), Some(hash));
		}
		let bundled = [op(12, account, later), op(6, account, hash), start(0)];
		assert_eq!(placement_user_op_hash(&bundled, account, 5), Some(hash));
		assert_eq!(placement_user_op_hash(&bundled, account, 10), Some(later));
		let one = [start(0), op(6, account, hash)];
		assert_eq!(placement_user_op_hash(&one, account, 3), Some(hash));
		assert_eq!(placement_user_op_hash(&one, account, 5), Some(hash));
		let skip = [start(0), op(6, other, hash), op(10, account, hash)];
		assert_eq!(placement_user_op_hash(&skip, account, 5), None);
		let spoofed = [start(0), user_op_at(6, later, account, other), op(10, account, hash)];
		assert_eq!(placement_user_op_hash(&spoofed, account, 5), Some(hash));
		for logs in [
			vec![],
			vec![start(0), op(4, account, hash)],
			vec![op(6, account, hash)],
			vec![before_execution(0, other), op(6, account, hash)],
			vec![start(7), op(10, account, hash)],
			vec![start(0), op(2, account, hash), start(7), op(10, account, hash)],
		] {
			assert_eq!(placement_user_op_hash(&logs, account, 5), None, "{logs:?}");
		}
	}

	#[test]
	fn delivery_amounts_per_output() {
		let logs = [
			transfer(2, TOKEN_A, FILLER, BENEFICIARY, 105),
			transfer(3, TOKEN_B, FILLER, BENEFICIARY, 200),
			fill_event(5),
		];
		assert_eq!(
			deliveries(&logs, 5, &outputs(&[TOKEN_A, TOKEN_B])),
			amounts(&[Some(105), Some(200)])
		);

		let repeated = [
			transfer(2, TOKEN_A, FILLER, BENEFICIARY, 50),
			transfer(3, TOKEN_A, FILLER, BENEFICIARY, 60),
			fill_event(5),
		];
		assert_eq!(
			deliveries(&repeated, 5, &outputs(&[TOKEN_A, TOKEN_A])),
			amounts(&[Some(50), Some(60)])
		);

		let partial = [transfer(2, TOKEN_A, FILLER, BENEFICIARY, 60), fill_event(5)];
		let partial_outputs = [
			TokenAmount { token: pad(TOKEN_A), amount: U256::ZERO },
			TokenAmount { token: pad(TOKEN_A), amount: U256::from(60) },
		];
		assert_eq!(deliveries(&partial, 5, &partial_outputs), amounts(&[Some(0), Some(60)]));

		let native = [transfer(2, TOKEN_A, FILLER, BENEFICIARY, 100), fill_event(5)];
		assert_eq!(
			deliveries(&native, 5, &outputs(&[Address::ZERO, TOKEN_A])),
			amounts(&[None, Some(100)])
		);
	}

	#[test]
	fn delivery_matching_ignores_unrelated_transfers() {
		let logs = [
			transfer(1, TOKEN_A, FILLER, GATEWAY, 5),
			transfer(2, TOKEN_A, BENEFICIARY, FILLER, 7),
			transfer(3, TOKEN_A, FILLER, BENEFICIARY, 100),
			fill_event(5),
			transfer(6, TOKEN_A, FILLER, BENEFICIARY, 9),
		];
		assert_eq!(deliveries(&logs, 5, &outputs(&[TOKEN_A])), amounts(&[Some(100)]));

		let batched = [
			transfer(2, TOKEN_A, FILLER, BENEFICIARY, 100),
			fill_event(3),
			transfer(6, TOKEN_A, FILLER, BENEFICIARY, 250),
			fill_event(8),
		];
		assert_eq!(deliveries(&batched, 8, &outputs(&[TOKEN_A])), amounts(&[Some(250)]));
		assert_eq!(deliveries(&batched, 3, &outputs(&[TOKEN_A])), amounts(&[Some(100)]));

		let ambiguous = [
			transfer(1, TOKEN_A, FILLER, BENEFICIARY, 7),
			transfer(2, TOKEN_A, FILLER, BENEFICIARY, 105),
		];
		assert_eq!(deliveries(&ambiguous, 5, &outputs(&[TOKEN_A])), amounts(&[None]));

		let missing = [transfer(2, TOKEN_A, FILLER, BENEFICIARY, 50)];
		assert_eq!(deliveries(&missing, 5, &outputs(&[TOKEN_A, TOKEN_A])), amounts(&[None, None]));

		let mut malformed = transfer(2, TOKEN_A, FILLER, BENEFICIARY, 100);
		malformed.data = None;
		assert_eq!(deliveries(&[malformed], 5, &outputs(&[TOKEN_A])), amounts(&[None]));

		let mut boundary = fill_event(3);
		boundary.topics = vec![PARTIAL_FILL];
		let partial_boundary = [
			transfer(2, TOKEN_A, FILLER, BENEFICIARY, 50),
			boundary,
			transfer(4, TOKEN_A, FILLER, BENEFICIARY, 60),
		];
		assert_eq!(deliveries(&partial_boundary, 5, &outputs(&[TOKEN_A])), amounts(&[Some(60)]));
		assert_eq!(deliveries(&[fill_event(5)], 5, &outputs(&[TOKEN_A])), amounts(&[None]));
	}

	fn sample_order() -> SolOrder {
		SolOrder {
			user: pad(BENEFICIARY),
			source: Bytes::from_static(b"EVM-97"),
			destination: Bytes::from_static(b"EVM-56"),
			deadline: U256::from(1000),
			nonce: U256::from(1),
			fees: U256::from(5),
			session: Address::ZERO,
			predispatch: Calls::DispatchInfo { assets: vec![], call: Bytes::new() },
			inputs: vec![TokenInfo { token: pad(TOKEN_A), amount: U256::from(100) }],
			output: Calls::PaymentInfo {
				beneficiary: pad(BENEFICIARY),
				assets: vec![TokenInfo { token: pad(TOKEN_B), amount: U256::from(200) }],
				call: Bytes::new(),
			},
		}
	}

	#[test]
	fn selectors_match_the_deployed_gateways() {
		assert_eq!(Calls::placeOrderCall::SELECTOR, IntentGatewayV3::placeOrderCall::SELECTOR);
		assert_eq!(hex::encode(Calls::placeOrderCall::SELECTOR), "958b00ed");
		assert_eq!(Calls::fillOrder_0Call::SELECTOR, IntentGatewayV3::fillOrderCall::SELECTOR);
		assert_eq!(hex::encode(Calls::fillOrder_0Call::SELECTOR), "68ddf058");
		assert_eq!(hex::encode(Calls::fillOrder_1Call::SELECTOR), "a5470064");
		assert_eq!(hex::encode(Calls::fillOrder_2Call::SELECTOR), "5cfb1ea5");
	}

	#[test]
	fn fill_orders_decode_in_every_deployed_shape() {
		let order = sample_order();
		let tokens = order.output.assets.clone();
		let current = Calls::fillOrder_0Call {
			order: order.clone(),
			options: Calls::FillOptions {
				relayerFee: U256::ZERO,
				nativeDispatchFee: U256::ZERO,
				validUntil: U256::ZERO,
				outputs: tokens.clone(),
				inputs: order.inputs.clone(),
			},
		}
		.abi_encode();
		let decoded = decode_fill_order(&current).unwrap();
		assert_eq!(decoded, order);
		assert_eq!(commitment(&decoded), commitment(&order));

		let without_inputs = Calls::fillOrder_1Call {
			order: order.clone(),
			options: Calls::FillOptionsWithoutInputs {
				relayerFee: U256::ZERO,
				nativeDispatchFee: U256::ZERO,
				validUntil: U256::ZERO,
				outputs: tokens.clone(),
			},
		}
		.abi_encode();
		assert_eq!(
			decode_fill_order(&without_inputs).unwrap().output.beneficiary,
			pad(BENEFICIARY)
		);
		let oldest = Calls::fillOrder_2Call {
			order: order.clone(),
			options: Calls::FillOptionsWithoutValidUntil {
				relayerFee: U256::ZERO,
				nativeDispatchFee: U256::ZERO,
				outputs: tokens,
			},
		}
		.abi_encode();
		assert_eq!(decode_fill_order(&oldest).unwrap().output.beneficiary, pad(BENEFICIARY));

		let place =
			Calls::placeOrderCall { order: order.clone(), graffiti: pad(FILLER) }.abi_encode();
		assert_eq!(decode_fill_order(&place), None);
		assert_eq!(decode_place_order(&place), Some((order, pad(FILLER))));
		assert_eq!(decode_fill_order(&[0x12, 0x34, 0x56, 0x78]), None);
		assert_eq!(decode_place_order(&current), None);
	}

	#[test]
	fn event_orders_hash_like_calldata_orders() {
		let sol = sample_order();
		let order = Order {
			user: sol.user,
			source: "EVM-97".into(),
			destination: "0x45564d2d3536".into(),
			deadline: sol.deadline,
			nonce: sol.nonce,
			fees: sol.fees,
			session: sol.session,
			predispatch: vec![],
			predispatch_call: Bytes::new(),
			inputs: vec![TokenAmount { token: pad(TOKEN_A), amount: U256::from(100) }],
			beneficiary: pad(BENEFICIARY),
			outputs: vec![TokenAmount { token: pad(TOKEN_B), amount: U256::from(200) }],
			output_call: Bytes::new(),
		};
		assert_eq!(order.commitment().unwrap(), commitment(&sol));
	}

	#[test]
	fn commitment_matches_a_mainnet_order() {
		// Base order 0x1e5aac55…, as the live indexer has it.
		let word = |hex: &str| pad(hex.parse().unwrap());
		let order = Order {
			user: word("0x3c43ad0f71bb11ba2cc9466c2c8614bbb2589d3d"),
			source: "EVM-8453".into(),
			destination: "EVM-8453".into(),
			deadline: U256::from(53459566u64),
			nonce: U256::from(1287),
			fees: U256::from(15086),
			session: "0xF162C33f96613888C86c1F95ab66F93A3Ed72303".parse().unwrap(),
			predispatch: vec![],
			predispatch_call: Bytes::new(),
			inputs: vec![TokenAmount {
				token: word("0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"),
				amount: U256::from(8256869500u64),
			}],
			beneficiary: word("0xda64d52fa817be25a7e492bb470b621adc3d40c2"),
			outputs: vec![TokenAmount {
				token: word("0x46c85152bfe9f96829aa94755d9f915f9b10ef5f"),
				amount: U256::from(11311911214988u64),
			}],
			output_call: Bytes::new(),
		};
		assert_eq!(
			order.commitment().unwrap().to_string(),
			"0x1e5aac55b5069b58d6a053f332b737a26d4b7d3837cdcecc5a4485f460927307"
		);
	}

	#[test]
	fn trace_walk_collects_successful_nested_calls() {
		let call = |to: Option<&str>, input: &str, extra: Value| {
			let mut call = json!({ "input": input });
			if let Some(to) = to {
				call["to"] = json!(to);
			}
			if let Value::Object(extra) = extra {
				call.as_object_mut().unwrap().extend(extra);
			}
			call
		};
		let gateway = "0x1111111111111111111111111111111111111111";
		let trace = call(
			Some("0x9999999999999999999999999999999999999999"),
			"0x",
			json!({ "calls": [
				call(Some(gateway), "0xaaaa", json!({})),
				call(Some("0x8888888888888888888888888888888888888888"), "0x", json!({
					"error": "execution reverted",
					"calls": [call(Some(gateway), "0xffff", json!({}))],
				})),
				call(None, "0x1234", json!({})),
				call(Some("0x7777777777777777777777777777777777777777"), "0x", json!({
					"calls": [call(Some(gateway), "0xbbbb", json!({}))],
				})),
			]}),
		);
		assert_eq!(trace_call_inputs(&trace, GATEWAY), vec!["0xaaaa", "0xbbbb"]);
		// A direct call to the target is not a nested one.
		assert!(trace_call_inputs(&call(Some(gateway), "0x1234", json!({})), GATEWAY).is_empty());
	}
}
