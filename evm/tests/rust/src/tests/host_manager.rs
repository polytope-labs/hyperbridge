use super::utils::*;
use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolCall, SolValue};
use ismp::{host::StateMachine, router};
use ismp_abi::evm_host::EvmHost::PostRequest as EvmPostRequest;
use pallet_ismp_host_executive::{encode_host_params, EvmHostParamsAbi, WithdrawalParams};
use polkadot_sdk::*;
use primitive_types::{H160, U256 as SubstrateU256};

alloy_sol_macro::sol! {
	struct HostParams {
		address feeToken;
		address admin;
		address handler;
		address hostManager;
		address uniswapV2;
		uint256 unStakingPeriod;
		uint256 challengePeriod;
		address consensusClient;
		uint256[] stateMachines;
		bytes hyperbridge;
	}

	struct IncomingPostRequestLocal {
		PostRequestLocal request;
		address relayer;
	}

	struct PostRequestLocal {
		bytes source;
		bytes dest;
		uint64 nonce;
		bytes from;
		bytes to;
		uint64 timeoutTimestamp;
		bytes body;
	}

	function onAccept(IncomingPostRequestLocal incoming) external;

	function balanceOf(address account) external view returns (uint256);

	function mint(address to, uint256 amount) external;

	function hostParams() external view returns (HostParams);
}

/// Build calldata for HostManager.onAccept(IncomingPostRequest) from an EvmPostRequest
fn onaccept_calldata(request: EvmPostRequest, relayer: Address) -> Vec<u8> {
	// Re-encode via ABI round-trip into the local sol! struct
	let encoded = SolValue::abi_encode(&request);
	let local = <PostRequestLocal as SolValue>::abi_decode(&encoded).unwrap();
	onAcceptCall { incoming: IncomingPostRequestLocal { request: local, relayer } }.abi_encode()
}

fn host_manager_of(env: &mut TestEnv) -> Address {
	let result = env.call(env.host, hostParamsCall {}.abi_encode());
	let params = <hostParamsCall as SolCall>::abi_decode_returns(&result).unwrap();
	params.hostManager
}

fn host_params(env: &mut TestEnv) -> HostParams {
	let result = env.call(env.host, hostParamsCall {}.abi_encode());
	<hostParamsCall as SolCall>::abi_decode_returns(&result).unwrap()
}

fn host_balance(env: &mut TestEnv) -> U256 {
	let result = env.call(env.fee_token, balanceOfCall { account: env.host }.abi_encode());
	<balanceOfCall as SolCall>::abi_decode_returns(&result).unwrap()
}

#[test]
fn test_host_manager_withdraw() {
	let mut env = TestEnv::new();
	let manager = host_manager_of(&mut env);

	// Mint 1000e18 fee tokens to the host
	let amount_to_mint = U256::from(1000u128) * U256::from(10u128.pow(18));
	env.call(env.fee_token, mintCall { to: env.host, amount: amount_to_mint }.abi_encode());
	assert_eq!(host_balance(&mut env), amount_to_mint);

	// Build a withdraw request (body = [0] + abi.encode(WithdrawParams)).
	// Withdraw the fee token (non-zero `token`) — the zero address would be
	// the native-ETH path which this test isn't exercising.
	let params = WithdrawalParams {
		beneficiary_address: H160::random().as_bytes().to_vec(),
		amount: SubstrateU256::from(500_000_000_000_000_000_000u128),
		token: H160::from_slice(env.fee_token.as_slice()),
	};

	let post = router::PostRequest {
		source: StateMachine::Kusama(2000),
		dest: StateMachine::Evm(1),
		nonce: 0,
		from: env.sender.as_slice().to_vec(),
		to: vec![],
		timeout_timestamp: 100,
		body: params.abi_encode().expect("20-byte beneficiary"),
	};
	let evm_request: EvmPostRequest = post.into();

	// HostManager.onAccept is `restrict(_params.host)` — must call AS the host
	let host_addr = env.host;
	let calldata = onaccept_calldata(evm_request, env.sender);
	env.call_as(host_addr, manager, calldata);

	let withdraw_amount = U256::from(500u128) * U256::from(10u128.pow(18));
	assert_eq!(host_balance(&mut env), amount_to_mint - withdraw_amount);
}

#[test]
fn test_host_manager_unauthorized_request() {
	let mut env = TestEnv::new();
	let manager = host_manager_of(&mut env);

	let params = WithdrawalParams {
		beneficiary_address: H160::random().as_bytes().to_vec(),
		amount: SubstrateU256::from(500_000_000_000_000_000_000u128),
		token: H160::zero(),
	};

	// Wrong source — not kusama-2000, expected to revert with UnauthorizedAction()
	let post = router::PostRequest {
		source: StateMachine::Polkadot(1000),
		dest: StateMachine::Evm(1),
		nonce: 0,
		from: env.sender.as_slice().to_vec(),
		to: vec![],
		timeout_timestamp: 100,
		body: params.abi_encode().expect("20-byte beneficiary"),
	};
	let evm_request: EvmPostRequest = post.into();

	let host_addr = env.host;
	let calldata = onaccept_calldata(evm_request, env.sender);
	let err = env
		.call_as_may_revert(host_addr, manager, calldata)
		.expect_err("expected revert");
	// UnauthorizedAction() selector = 0x843800fa
	assert_eq!(&err[..4], &[0x84, 0x38, 0x00, 0xfa]);
}

#[test]
fn test_host_manager_insufficient_balance() {
	let mut env = TestEnv::new();
	let manager = host_manager_of(&mut env);

	// Host has no fee tokens; withdraw attempt should fail on SafeERC20 transfer
	let params = WithdrawalParams {
		beneficiary_address: H160::random().as_bytes().to_vec(),
		amount: SubstrateU256::from(500_000_000_000_000_000_000u128),
		token: H160::from_slice(env.fee_token.as_slice()),
	};

	let post = router::PostRequest {
		source: StateMachine::Kusama(2000),
		dest: StateMachine::Evm(1),
		nonce: 0,
		from: env.sender.as_slice().to_vec(),
		to: vec![],
		timeout_timestamp: 100,
		body: params.abi_encode().expect("20-byte beneficiary"),
	};
	let evm_request: EvmPostRequest = post.into();

	let host_addr = env.host;
	let calldata = onaccept_calldata(evm_request, env.sender);
	let err = env
		.call_as_may_revert(host_addr, manager, calldata)
		.expect_err("expected revert");
	assert!(!err.is_empty(), "expected non-empty revert data");
}

#[test]
fn test_host_manager_set_host_params() {
	let mut env = TestEnv::new();
	let manager = host_manager_of(&mut env);

	let value = host_params(&mut env);
	let new_challenge_period = U256::from(5_000_000u128);

	let params = EvmHostParamsAbi {
		feeToken: value.feeToken,
		admin: value.admin,
		handler: value.handler,
		hostManager: value.hostManager,
		uniswapV2: value.uniswapV2,
		unStakingPeriod: value.unStakingPeriod,
		challengePeriod: new_challenge_period,
		consensusClient: value.consensusClient,
		stateMachines: value.stateMachines.clone(),
		hyperbridge: value.hyperbridge.to_vec().into(),
	};
	// encode_host_params prepends action byte (1 = SetHostParam)
	let body = encode_host_params(&params);

	let post = router::PostRequest {
		source: StateMachine::Kusama(2000),
		dest: StateMachine::Evm(1),
		nonce: 0,
		from: env.sender.as_slice().to_vec(),
		to: vec![],
		timeout_timestamp: 100,
		body,
	};
	let evm_request: EvmPostRequest = post.into();

	let host_addr = env.host;
	let calldata = onaccept_calldata(evm_request, env.sender);
	env.call_as(host_addr, manager, calldata);

	let updated = host_params(&mut env);
	assert_eq!(updated.challengePeriod, new_challenge_period);
}
