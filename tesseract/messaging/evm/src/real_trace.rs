//! Live mainnet reproduction of issue #1436.
//!
//! The request was delivered on Base and proved on Hyperbridge, but the
//! `IntentGateway` at the destination reverts in `onAccept` because the order
//! was already cancelled. BSC has no receipt for it, and the old retry loop
//! re-simulated it every five minutes forever.
//!
//! Run with the BSC endpoint from `evm/.env.mainnet`:
//!
//! ```text
//! cargo test -p tesseract-evm --lib real_trace -- --ignored --nocapture
//! ```

use alloy::{
	eips::BlockId,
	primitives::{Address, U256 as AlloyU256},
	providers::ext::DebugApi,
};
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::{hash_request, Message, Proof, RequestMessage},
	router::{PostRequest, Request},
};
use primitive_types::H256;
use sp_core::H160;
use tesseract_primitives::{Hasher, IsmpProvider, SimulationFailure};

use crate::{abi::EvmHostInstance, EvmClient, EvmConfig};

alloy_sol_types::sol! {
	#[sol(rpc)]
	interface IIntentGateway {
		function relayer() external view returns (address);
	}
}

const BSC_HOST: H160 = H160(hex_literal::hex!("620128E2B19193d6Bd244a3AC8D3bBa0541B19c3"));
const HYPERBRIDGE_STATE_MACHINE: StateMachine = StateMachine::Polkadot(3367);
const HYPERBRIDGE_LATEST_HEIGHT_QUERY: u64 = 3367;
const DEFAULT_HYPERBRIDGE_RPC: &str = "https://nexus.rpc.polytope.technology";

/// Nexus indexer message
/// `0xcc973faa2e1cb735db8bf7e2eca61cab973eb20d119546e1e72535e8db0a733a`
/// (#1436): an `IntentGateway` cancellation on Base→BSC.
const MESSAGE_COMMITMENT: &str =
	"0xcc973faa2e1cb735db8bf7e2eca61cab973eb20d119546e1e72535e8db0a733a";
const REQUEST_NONCE: u64 = 369;
const INTENT_GATEWAY: &str = "Ae041F7B0CB581876832830baeB6a2Aa2a3C9716";
const REQUEST_BODY: &str = "0x0400000000000000000000000000000000000000000000000000000000000000202fdf0561d605175ffa70908042834a48ea3519567a5c879468aec4b29c757ec1000000000000000000000000f2d68da20aae040ff844c59aa13b9b2d7402faca0000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000000100000000000000000000000055d398326f99059ff775485246999027b3197955000000000000000000000000000000000000000000000010296b7c6deddeea00";

fn request() -> PostRequest {
	let gateway = hex::decode(INTENT_GATEWAY).expect("valid intent gateway address");
	PostRequest {
		source: StateMachine::Evm(8453),
		dest: StateMachine::Evm(56),
		nonce: REQUEST_NONCE,
		from: gateway.clone(),
		to: gateway,
		timeout_timestamp: 0,
		body: hex::decode(REQUEST_BODY.trim_start_matches("0x")).expect("valid request body"),
	}
}

fn commitment() -> H256 {
	H256::from_slice(
		&hex::decode(MESSAGE_COMMITMENT.trim_start_matches("0x")).expect("valid commitment"),
	)
}

#[tokio::test]
#[ignore = "live BSC + Hyperbridge RPC"]
async fn intent_gateway_cancellation_is_a_module_revert() {
	let bsc_url =
		std::env::var("BSC_RPC_URL").expect("BSC_RPC_URL must be set (source evm/.env.mainnet)");
	let hyperbridge_url =
		std::env::var("HYPERBRIDGE_RPC_URL").unwrap_or_else(|_| DEFAULT_HYPERBRIDGE_RPC.into());

	let post = request();
	// A typo in the body or address would change the commitment, so fail before
	// spending a trace on the wrong message.
	assert_eq!(
		hash_request::<Hasher>(&Request::Post(post.clone())),
		commitment(),
		"request does not match the explorer message"
	);

	let config = EvmConfig {
		rpc_urls: vec![bsc_url],
		state_machine: Some(StateMachine::Evm(56)),
		consensus_state_id: Some("BSC0".into()),
		ismp_host: Some(BSC_HOST),
		..Default::default()
	};
	let mut client = EvmClient::new(config).await.expect("BSC client");

	// `onAccept` is `onlyRelayer(incoming.relayer)`. A throwaway signer would
	// revert `Unauthorized()` at that gate and hide the real module revert, so
	// trace from the gateway's own authorized relayer. Only the calldata sender
	// matters for `debug_traceCall`; no signature is needed.
	let gateway = Address::from_slice(&post.to);
	let relayer = IIntentGateway::new(gateway, client.client.clone())
		.relayer()
		.call()
		.await
		.expect("IntentGateway relayer on BSC");
	assert_ne!(
		relayer,
		Address::ZERO,
		"gateway has no authorized relayer, so the test cannot reproduce the gated path"
	);
	client.address = relayer.to_vec();

	// Nobody else delivered it either, which is why the relayer kept retrying.
	let receipt = client
		.query_request_receipt(commitment())
		.await
		.expect("request receipt on BSC");
	assert_eq!(
		receipt,
		vec![0u8; 20],
		"BSC unexpectedly has a receipt for the message: {}",
		hex::encode(&receipt)
	);

	// Read the Hyperbridge height BSC has a commitment for. The proof is built
	// at that height, not at the chain tip.
	let host_addr = Address::from_slice(&client.ismp_host.0);
	let host = EvmHostInstance::new(host_addr, client.client.clone());
	let height = host
		.latestStateMachineHeight(AlloyU256::from(HYPERBRIDGE_LATEST_HEIGHT_QUERY))
		.block(BlockId::latest())
		.call()
		.await
		.expect("latestStateMachineHeight for Hyperbridge on BSC");
	let height: u64 = height.try_into().expect("height fits in u64");

	let rpc_response: serde_json::Value = reqwest::Client::new()
		.post(&hyperbridge_url)
		.json(&serde_json::json!({
			"jsonrpc": "2.0",
			"id": 1,
			"method": "mmr_queryProof",
			"params": [height, { "Requests": [format!("{:#x}", commitment())] }],
		}))
		.send()
		.await
		.expect("Hyperbridge RPC request")
		.error_for_status()
		.expect("Hyperbridge RPC status")
		.json()
		.await
		.expect("Hyperbridge RPC body");
	let result = rpc_response
		.get("result")
		.unwrap_or_else(|| panic!("mmr_queryProof failed at {height}: {rpc_response}"));
	let proof: Vec<u8> =
		serde_json::from_value(result["proof"].clone()).expect("proof bytes in RPC response");
	let proof_height: u64 = result["height"].as_u64().expect("proof height in RPC response");

	let message = Message::Request(RequestMessage {
		requests: vec![post],
		proof: Proof {
			height: StateMachineHeight {
				id: StateMachineId {
					state_id: HYPERBRIDGE_STATE_MACHINE,
					consensus_state_id: *b"BEEF",
				},
				height: proof_height,
			},
			proof,
		},
		signer: vec![],
	});

	// Trace the delivery at the chain gas limit, as the submission path does.
	let (mut txs, _) = crate::tx::generate_contract_calls(&client, &[message], true)
		.await
		.expect("delivery calldata");
	let tx = txs.pop().expect("one transaction");
	let chain_gas_limit = crate::tx::get_chain_gas_limit(client.state_machine);
	let trace = client
		.client
		.debug_trace_call(
			tx.gas_limit(chain_gas_limit),
			BlockId::latest(),
			crate::provider::call_tracer_options().expect("trace options"),
		)
		.await
		.expect("debug_traceCall");
	let alloy::rpc::types::trace::geth::GethTrace::CallTracer(frame) = trace else {
		panic!("unexpected trace variant: {trace:?}");
	};
	let handler = client.handler().await.expect("handler address");
	let handler = Address::from_slice(&handler.0);
	let delivered = crate::provider::check_trace_for_event(
		&frame,
		crate::provider::CheckTraceForEventParams::Request,
	);
	let failure = crate::provider::simulation_failure(
		&frame,
		Address::from_slice(&client.ismp_host.0),
		handler,
	);
	eprintln!("delivery trace at chain gas limit {chain_gas_limit}: {failure:?}");
	assert!(!delivered, "unexpectedly delivered: {failure:?}");

	match &failure {
		SimulationFailure::ModuleReverted { reason } => {
			// `IntentGatewayV2.onAccept` delegates to its `ExtrinsicModule`, so the
			// deepest reverting frame is that module (`0x5f925b88…`) rather than the
			// gateway address in the request. The order was already cancelled, so
			// `RefundEscrow` hits `Filled()`.
			let reason = reason.to_ascii_lowercase();
			assert!(
				reason.contains(&INTENT_GATEWAY.to_ascii_lowercase()) ||
					reason.contains("5f925b8870f730b3feb8bddaed7ad088c62a32c2"),
				"revert reason names neither the IntentGateway {INTENT_GATEWAY} nor the \
				 ExtrinsicModule it delegates to: {reason}"
			);
			let filled = hex::encode(&alloy::primitives::keccak256("Filled()")[..4]);
			assert!(
				reason.contains(&filled),
				"revert reason is not Filled() (0x{filled}): {reason}"
			);
		},
		other => panic!("expected a module revert, got {other:?}"),
	}
}
