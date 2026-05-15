//! ABI encoding and type conversions for ISMP types.
//!
//! Uses the generated Solidity ABI types from `ismp-abi` to produce
//! byte arrays identical to Solidity's `abi.encode(...)`. This prevents hash
//! malleability vulnerabilities that exist with `abi.encodePacked`.
//!
//! Also contains conversions between ISMP router types and the generated
//! Solidity ABI types.

use crate::{
	consensus,
	events::{StateCommitmentVetoed, StateMachineUpdated, TimeoutHandled},
	host::StateMachine,
	prelude::Vec,
	router,
};
use alloc::string::{String, ToString};
use alloy_primitives::{Bytes, FixedBytes};
use alloy_sol_types::SolValue;
use anyhow::anyhow;
use core::str::FromStr;
use ismp_abi::{
	conversions::ToU256,
	evm_host::EvmHost::{
		GetRequest, GetRequestEvent, GetRequestHandled, GetRequestTimeoutHandled, GetResponse,
		HostFrozen, HostParamsUpdated, HostWithdrawal, PostRequest, PostRequestEvent,
		PostRequestHandled, PostRequestTimeoutHandled, RequestFunded, StateCommitment,
		StateCommitmentRead, StateCommitmentVetoed as EvmStateCommitmentVetoed,
		StateMachineHeight, StateMachineUpdated as EvmStateMachineUpdated,
	},
};
use primitive_types::{H160, H256};

// ── Encoding ──────────────────────────────────────────────────────────

/// Encode a [`router::PostRequest`] identically to Solidity's
/// `abi.encode(req.source, req.dest, req.nonce, req.timeoutTimestamp, req.from, req.to, req.body)`.
pub fn encode_post_request(req: &router::PostRequest) -> Vec<u8> {
	let sol: PostRequest = req.clone().into();
	sol.abi_encode_params()
}

/// Encode a [`router::GetRequest`] identically to Solidity's
/// `abi.encode(req.source, req.dest, req.nonce, req.height, req.timeoutTimestamp,
///             abi.encodePacked(req.from), req.keys, req.context)`.
pub fn encode_get_request(req: &router::GetRequest) -> Vec<u8> {
	let sol: GetRequest = req.clone().into();
	sol.abi_encode_params()
}

/// Encode a [`router::GetResponse`] identically to Solidity's
/// `abi.encode(encode(res.request), res.values)`.
///
/// The Solidity hash function passes the pre-encoded request bytes (not the struct),
/// so this encodes as `(bytes, StorageValue[])`.
pub fn encode_get_response(
	request_encoding: &[u8],
	values: &[router::StorageValue],
) -> Vec<u8> {
	let sol_values: Vec<ismp_abi::evm_host::StorageValue> = values
		.iter()
		.map(|sv| ismp_abi::evm_host::StorageValue {
			key: sv.key.clone().into(),
			value: sv.value.as_ref().cloned().unwrap_or_default().into(),
		})
		.collect();
	let request_bytes = Bytes::from(request_encoding.to_vec());
	(request_bytes, sol_values).abi_encode_params()
}

// ── Router type conversions (moved from ismp-abi) ───────────

impl From<router::PostRequest> for PostRequest {
	fn from(value: router::PostRequest) -> Self {
		PostRequest {
			source: value.source.to_string().into_bytes().into(),
			dest: value.dest.to_string().into_bytes().into(),
			nonce: value.nonce,
			from: value.from.into(),
			to: value.to.into(),
			timeoutTimestamp: value.timeout_timestamp,
			body: value.body.into(),
		}
	}
}

impl From<router::PostRequest>
	for ismp_abi::handler::Handler::PostRequest
{
	fn from(value: router::PostRequest) -> Self {
		Self {
			source: value.source.to_string().into_bytes().into(),
			dest: value.dest.to_string().into_bytes().into(),
			nonce: value.nonce,
			from: value.from.into(),
			to: value.to.into(),
			timeoutTimestamp: value.timeout_timestamp,
			body: value.body.into(),
		}
	}
}

impl From<router::PostRequest>
	for ismp_abi::handler::handler_v2::HandlerV2::PostRequest
{
	fn from(value: router::PostRequest) -> Self {
		Self {
			source: value.source.to_string().into_bytes().into(),
			dest: value.dest.to_string().into_bytes().into(),
			nonce: value.nonce,
			from: value.from.into(),
			to: value.to.into(),
			timeoutTimestamp: value.timeout_timestamp,
			body: value.body.into(),
		}
	}
}

impl TryFrom<PostRequest> for router::PostRequest {
	type Error = anyhow::Error;
	fn try_from(value: PostRequest) -> Result<Self, Self::Error> {
		Ok(router::PostRequest {
			source: StateMachine::from_str(
				&String::from_utf8(value.source.to_vec()).map_err(|e| anyhow!("{e}"))?,
			)
			.map_err(|err| anyhow!("{err}"))?,
			dest: StateMachine::from_str(
				&String::from_utf8(value.dest.to_vec()).map_err(|e| anyhow!("{e}"))?,
			)
			.map_err(|err| anyhow!("{err}"))?,
			nonce: value.nonce.try_into().map_err(|e| anyhow!("{e}"))?,
			from: value.from.to_vec(),
			to: value.to.to_vec(),
			timeout_timestamp: value.timeoutTimestamp.try_into().map_err(|e| anyhow!("{e}"))?,
			body: value.body.to_vec(),
		})
	}
}

impl From<router::GetRequest> for GetRequest {
	fn from(value: router::GetRequest) -> Self {
		GetRequest {
			source: value.source.to_string().into_bytes().into(),
			dest: value.dest.to_string().into_bytes().into(),
			nonce: value.nonce,
			keys: value.keys.into_iter().map(Into::into).collect(),
			from: {
				let mut address = H160::default();
				address.0.copy_from_slice(&value.from);
				alloy_primitives::Address::from(address.0)
			},
			context: value.context.into(),
			timeoutTimestamp: value.timeout_timestamp,
			height: value.height,
		}
	}
}

impl From<router::GetResponse> for GetResponse {
	fn from(value: router::GetResponse) -> Self {
		GetResponse {
			request: value.get.into(),
			values: value
				.values
				.into_iter()
				.map(|storage_value| ismp_abi::evm_host::StorageValue {
					key: storage_value.key.into(),
					value: storage_value.value.unwrap_or_default().into(),
				})
				.collect(),
		}
	}
}

// ── Consensus type conversions (moved from ismp-abi) ────────

impl TryFrom<consensus::StateMachineHeight> for StateMachineHeight {
	type Error = anyhow::Error;
	fn try_from(value: consensus::StateMachineHeight) -> Result<Self, anyhow::Error> {
		Ok(StateMachineHeight {
			stateMachineId: match value.id.state_id {
				StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.to_u256(),
				state_machine => Err(anyhow!("Unsupported state machine {state_machine:?}"))?,
			},
			height: value.height.to_u256(),
		})
	}
}

impl From<consensus::StateCommitment> for StateCommitment {
	fn from(value: consensus::StateCommitment) -> Self {
		StateCommitment {
			timestamp: value.timestamp.to_u256(),
			stateRoot: FixedBytes::from(value.state_root.0),
			overlayRoot: FixedBytes::from(value.overlay_root.unwrap_or_default().0),
		}
	}
}

// ── Event conversions (moved from ismp-abi) ─────────────────

/// Enum representing all EvmHost events for conversion
pub enum EvmHostEvents {
	/// A get request event
	GetRequestEvent(GetRequestEvent),
	/// A post request event
	PostRequestEvent(PostRequestEvent),
	/// A post request handled event
	PostRequestHandled(PostRequestHandled),
	/// A get request handled event
	GetRequestHandled(GetRequestHandled),
	/// A state machine updated event
	StateMachineUpdated(EvmStateMachineUpdated),
	/// A post request timeout handled event
	PostRequestTimeoutHandled(PostRequestTimeoutHandled),
	/// A get request timeout handled event
	GetRequestTimeoutHandled(GetRequestTimeoutHandled),
	/// A state commitment vetoed event
	StateCommitmentVetoed(EvmStateCommitmentVetoed),
	/// A state commitment read event
	StateCommitmentRead(StateCommitmentRead),
	/// A host frozen event
	HostFrozen(HostFrozen),
	/// A host withdrawal event
	HostWithdrawal(HostWithdrawal),
	/// A host params updated event
	HostParamsUpdated(HostParamsUpdated),
	/// A request funded event
	RequestFunded(RequestFunded),
}

impl TryFrom<EvmHostEvents> for crate::events::Event {
	type Error = anyhow::Error;
	fn try_from(event: EvmHostEvents) -> Result<Self, Self::Error> {
		match event {
			EvmHostEvents::GetRequestEvent(get) =>
				Ok(crate::events::Event::GetRequest(get.try_into()?)),
			EvmHostEvents::PostRequestEvent(post) =>
				Ok(crate::events::Event::PostRequest(post.try_into()?)),
			EvmHostEvents::PostRequestHandled(handled) =>
				Ok(crate::events::Event::PostRequestHandled(
					crate::events::RequestResponseHandled {
						commitment: H256(handled.commitment.0),
						relayer: handled.relayer.0.to_vec(),
					},
				)),
			EvmHostEvents::GetRequestHandled(handled) =>
				Ok(crate::events::Event::GetRequestHandled(
					crate::events::RequestResponseHandled {
						commitment: H256(handled.commitment.0),
						relayer: handled.relayer.0.to_vec(),
					},
				)),
			EvmHostEvents::StateMachineUpdated(filter) =>
				Ok(crate::events::Event::StateMachineUpdated(StateMachineUpdated {
					state_machine_id: consensus::StateMachineId {
						state_id: StateMachine::from_str(&filter.stateMachineId)
							.map_err(|e| anyhow!("{}", e))?,
						consensus_state_id: Default::default(),
					},
					latest_height: filter.height.try_into().map_err(|e| anyhow!("{e}"))?,
				})),
			EvmHostEvents::PostRequestTimeoutHandled(handled) => {
				let dest =
					StateMachine::from_str(&handled.dest).map_err(|e| anyhow!("{}", e))?;
				Ok(crate::events::Event::PostRequestTimeoutHandled(TimeoutHandled {
					commitment: H256(handled.commitment.0),
					dest: dest.clone(),
					source: dest.clone(),
				}))
			},
			EvmHostEvents::GetRequestTimeoutHandled(handled) => {
				let dest =
					StateMachine::from_str(&handled.dest).map_err(|e| anyhow!("{}", e))?;
				Ok(crate::events::Event::GetRequestTimeoutHandled(TimeoutHandled {
					commitment: H256(handled.commitment.0),
					dest: dest.clone(),
					source: dest.clone(),
				}))
			},
			EvmHostEvents::StateCommitmentVetoed(vetoed) =>
				Ok(crate::events::Event::StateCommitmentVetoed(StateCommitmentVetoed {
					height: consensus::StateMachineHeight {
						id: consensus::StateMachineId {
							state_id: StateMachine::from_str(&vetoed.stateMachineId)
								.map_err(|e| anyhow!("{}", e))?,
							consensus_state_id: Default::default(),
						},
						height: vetoed.height.try_into().map_err(|e| anyhow!("{e}"))?,
					},
					fisherman: vetoed.fisherman.0.to_vec(),
				})),
			EvmHostEvents::StateCommitmentRead(_) |
			EvmHostEvents::HostFrozen(_) |
			EvmHostEvents::HostWithdrawal(_) |
			EvmHostEvents::HostParamsUpdated(_) |
			EvmHostEvents::RequestFunded(_) => Err(anyhow!("Unsupported Event!"))?,
		}
	}
}

impl TryFrom<PostRequestEvent> for router::PostRequest {
	type Error = anyhow::Error;

	fn try_from(post: PostRequestEvent) -> Result<Self, Self::Error> {
		Ok(router::PostRequest {
			source: StateMachine::from_str(&post.source).map_err(|e| anyhow!("{}", e))?,
			dest: StateMachine::from_str(&post.dest).map_err(|e| anyhow!("{}", e))?,
			nonce: post.nonce.try_into().map_err(|e| anyhow!("{e}"))?,
			from: post.from.0.to_vec(),
			to: post.to.0.to_vec(),
			timeout_timestamp: post.timeoutTimestamp.try_into().map_err(|e| anyhow!("{e}"))?,
			body: post.body.to_vec(),
		})
	}
}

impl TryFrom<GetRequestEvent> for router::GetRequest {
	type Error = anyhow::Error;

	fn try_from(get: GetRequestEvent) -> Result<Self, Self::Error> {
		Ok(router::GetRequest {
			source: StateMachine::from_str(&get.source).map_err(|e| anyhow!("{}", e))?,
			dest: StateMachine::from_str(&get.dest).map_err(|e| anyhow!("{}", e))?,
			nonce: get.nonce.try_into().map_err(|e| anyhow!("{e}"))?,
			from: get.from.0.to_vec(),
			keys: get.keys.into_iter().map(|key| key.to_vec()).collect(),
			height: get.height.try_into().map_err(|e| anyhow!("{e}"))?,
			context: get.context.to_vec(),
			timeout_timestamp: get.timeoutTimestamp.try_into().map_err(|e| anyhow!("{e}"))?,
		})
	}
}
