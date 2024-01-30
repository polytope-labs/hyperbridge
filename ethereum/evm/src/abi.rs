#![allow(clippy::all, ambiguous_glob_reexports)]

use anyhow::anyhow;
use ismp::{
	consensus::StateMachineId,
	events::{Event, StateMachineUpdated},
	host::StateMachine,
	router,
};

use std::str::FromStr;

pub mod arb_gas_info;
pub mod i_rollup;
pub mod l2_output_oracle;
pub mod ovm_gas_price_oracle;

pub use ismp_solidity_abi::{beefy::*, evm_host::*, handler::*, ping_module::*};

pub fn to_ismp_event(event: EvmHostEvents) -> Result<Event, anyhow::Error> {
	match event {
		EvmHostEvents::GetRequestEventFilter(get) => Ok(Event::GetRequest(router::Get {
			source: StateMachine::from_str(&String::from_utf8(get.source.0.into())?)
				.map_err(|e| anyhow!("{}", e))?,
			dest: StateMachine::from_str(&String::from_utf8(get.dest.0.into())?)
				.map_err(|e| anyhow!("{}", e))?,
			nonce: get.nonce.low_u64(),
			from: get.from.0.into(),
			keys: get.keys.into_iter().map(|key| key.0.into()).collect(),
			height: get.height.low_u64(),
			timeout_timestamp: get.timeout_timestamp.low_u64(),
			gas_limit: get.gaslimit.low_u64(),
		})),
		EvmHostEvents::PostRequestEventFilter(post) => Ok(Event::PostRequest(router::Post {
			source: StateMachine::from_str(&String::from_utf8(post.source.0.into())?)
				.map_err(|e| anyhow!("{}", e))?,
			dest: StateMachine::from_str(&String::from_utf8(post.dest.0.into())?)
				.map_err(|e| anyhow!("{}", e))?,
			nonce: post.nonce.low_u64(),
			from: post.from.0.into(),
			to: post.to.0.into(),
			timeout_timestamp: post.timeout_timestamp.low_u64(),
			data: post.data.0.into(),
			gas_limit: post.gaslimit.low_u64(),
		})),
		EvmHostEvents::PostResponseEventFilter(resp) =>
			Ok(Event::PostResponse(router::PostResponse {
				post: router::Post {
					source: StateMachine::from_str(&String::from_utf8(resp.source.0.into())?)
						.map_err(|e| anyhow!("{}", e))?,
					dest: StateMachine::from_str(&String::from_utf8(resp.dest.0.into())?)
						.map_err(|e| anyhow!("{}", e))?,
					nonce: resp.nonce.low_u64(),
					from: resp.from.0.into(),
					to: resp.to.0.into(),
					timeout_timestamp: resp.timeout_timestamp.low_u64(),
					data: resp.data.0.into(),
					gas_limit: resp.gaslimit.low_u64(),
				},
				response: resp.response.0.into(),
				timeout_timestamp: resp.timeout_timestamp.low_u64(),
				gas_limit: resp.res_gaslimit.low_u64(),
			})),
		_ => Err(anyhow!("Unknown event")),
	}
}

pub fn to_state_machine_updated(event: StateMachineUpdatedFilter) -> StateMachineUpdated {
	StateMachineUpdated {
		state_machine_id: StateMachineId {
			state_id: StateMachine::Kusama(event.state_machine_id.low_u64() as u32),
			consensus_state_id: Default::default(),
		},
		latest_height: event.height.low_u64(),
	}
}
