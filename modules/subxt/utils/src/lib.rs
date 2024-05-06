use codec::Encode;
use sp_core_hashing::keccak_256;
use subxt::{
	config::{polkadot::PolkadotExtrinsicParams, substrate::SubstrateHeader, Hasher},
	utils::{AccountId32, MultiAddress, MultiSignature, H256},
};

pub mod client;
pub mod gargantua;

mod gargantua_conversion {
	use crate::gargantua::api::runtime_types::pallet_hyperbridge::VersionedHostParams;

	use super::gargantua::api::runtime_types;
	use ismp::{
		consensus::{StateCommitment, StateMachineHeight, StateMachineId},
		host::{Ethereum, StateMachine},
	};
	use pallet_ismp_host_executive::{EvmHostParam, HostParam};

	impl From<runtime_types::ismp::consensus::StateCommitment> for StateCommitment {
		fn from(commitment: runtime_types::ismp::consensus::StateCommitment) -> Self {
			StateCommitment {
				timestamp: commitment.timestamp,
				overlay_root: commitment.overlay_root,
				state_root: commitment.state_root,
			}
		}
	}

	impl From<runtime_types::ismp::consensus::StateMachineHeight> for StateMachineHeight {
		fn from(state_machine_height: runtime_types::ismp::consensus::StateMachineHeight) -> Self {
			StateMachineHeight {
				id: state_machine_height.id.into(),
				height: state_machine_height.height,
			}
		}
	}

	impl From<runtime_types::ismp::consensus::StateMachineId> for StateMachineId {
		fn from(state_machine_id: runtime_types::ismp::consensus::StateMachineId) -> Self {
			StateMachineId {
				state_id: state_machine_id.state_id.into(),
				consensus_state_id: state_machine_id.consensus_state_id,
			}
		}
	}

	impl From<runtime_types::ismp::host::StateMachine> for StateMachine {
		fn from(state_machine_id: runtime_types::ismp::host::StateMachine) -> Self {
			match state_machine_id {
				runtime_types::ismp::host::StateMachine::Ethereum(ethereum) => match ethereum {
					runtime_types::ismp::host::Ethereum::ExecutionLayer =>
						StateMachine::Ethereum(Ethereum::ExecutionLayer),
					runtime_types::ismp::host::Ethereum::Optimism =>
						StateMachine::Ethereum(Ethereum::Optimism),
					runtime_types::ismp::host::Ethereum::Arbitrum =>
						StateMachine::Ethereum(Ethereum::Arbitrum),
					runtime_types::ismp::host::Ethereum::Base =>
						StateMachine::Ethereum(Ethereum::Base),
				},
				runtime_types::ismp::host::StateMachine::Polkadot(id) => StateMachine::Polkadot(id),
				runtime_types::ismp::host::StateMachine::Kusama(id) => StateMachine::Kusama(id),
				runtime_types::ismp::host::StateMachine::Grandpa(consensus_state_id) =>
					StateMachine::Grandpa(consensus_state_id),
				runtime_types::ismp::host::StateMachine::Beefy(consensus_state_id) =>
					StateMachine::Beefy(consensus_state_id),
				runtime_types::ismp::host::StateMachine::Polygon => StateMachine::Polygon,
				runtime_types::ismp::host::StateMachine::Bsc => StateMachine::Bsc,
			}
		}
	}

	impl From<StateMachineHeight> for runtime_types::ismp::consensus::StateMachineHeight {
		fn from(state_machine_height: StateMachineHeight) -> Self {
			runtime_types::ismp::consensus::StateMachineHeight {
				id: state_machine_height.id.into(),
				height: state_machine_height.height,
			}
		}
	}

	impl From<StateMachineId> for runtime_types::ismp::consensus::StateMachineId {
		fn from(state_machine_id: StateMachineId) -> Self {
			Self {
				state_id: state_machine_id.state_id.into(),
				consensus_state_id: state_machine_id.consensus_state_id,
			}
		}
	}

	impl From<StateMachine> for runtime_types::ismp::host::StateMachine {
		fn from(state_machine_id: StateMachine) -> Self {
			match state_machine_id {
				StateMachine::Ethereum(ethereum) => match ethereum {
					Ethereum::ExecutionLayer => runtime_types::ismp::host::StateMachine::Ethereum(
						runtime_types::ismp::host::Ethereum::ExecutionLayer,
					),
					Ethereum::Optimism => runtime_types::ismp::host::StateMachine::Ethereum(
						runtime_types::ismp::host::Ethereum::Optimism,
					),
					Ethereum::Arbitrum => runtime_types::ismp::host::StateMachine::Ethereum(
						runtime_types::ismp::host::Ethereum::Arbitrum,
					),
					Ethereum::Base => runtime_types::ismp::host::StateMachine::Ethereum(
						runtime_types::ismp::host::Ethereum::Base,
					),
				},
				StateMachine::Polkadot(id) => runtime_types::ismp::host::StateMachine::Polkadot(id),
				StateMachine::Kusama(id) => runtime_types::ismp::host::StateMachine::Kusama(id),
				StateMachine::Grandpa(consensus_state_id) =>
					runtime_types::ismp::host::StateMachine::Grandpa(consensus_state_id),
				StateMachine::Beefy(consensus_state_id) =>
					runtime_types::ismp::host::StateMachine::Beefy(consensus_state_id),

				StateMachine::Polygon => runtime_types::ismp::host::StateMachine::Polygon,
				StateMachine::Bsc => runtime_types::ismp::host::StateMachine::Bsc,
			}
		}
	}

	impl From<ismp::router::Post> for runtime_types::ismp::router::Post {
		fn from(post: ismp::router::Post) -> Self {
			Self {
				source: post.source.into(),
				dest: post.dest.into(),
				nonce: post.nonce,
				from: post.from,
				to: post.to,
				timeout_timestamp: post.timeout_timestamp,
				data: post.data,
			}
		}
	}

	impl From<runtime_types::pallet_ismp_host_executive::params::HostParam<u128>> for HostParam<u128> {
		fn from(value: runtime_types::pallet_ismp_host_executive::params::HostParam<u128>) -> Self {
			match value {
                runtime_types::pallet_ismp_host_executive::params::HostParam::EvmHostParam(params) => {
                    let evm = EvmHostParam {
                        default_timeout: params.default_timeout,
                        per_byte_fee: params.per_byte_fee,
                        fee_token: params.fee_token,
                        admin: params.admin,
                        handler: params.handler,
                        host_manager: params.host_manager,
                        un_staking_period: params.un_staking_period,
                        challenge_period: params.challenge_period,
                        consensus_client: params.consensus_client,
                        consensus_state: params
                            .consensus_state
                            .0
                            .try_into().expect("Runtime will always provide bounded vec"),
                        consensus_update_timestamp: params.consensus_update_timestamp,
                        state_machine_whitelist: params
                            .state_machine_whitelist
                            .0
                            .try_into()
                            .expect("Runtime will always provide bounded vec"),
                        fishermen: params
                            .fishermen
                            .0
                            .try_into()
                            .expect("Runtime will always provide bounded vec"),
                        hyperbridge: params
                            .hyperbridge
                            .0
                            .try_into()
                            .expect("Runtime will always provide bounded vec"),
                    };
                    HostParam::EvmHostParam(evm)
                }
                runtime_types::pallet_ismp_host_executive::params::HostParam::SubstrateHostParam(VersionedHostParams::V1(value)) => {
                    HostParam::SubstrateHostParam(pallet_hyperbridge::VersionedHostParams::V1(value))
                }
            }
		}
	}
}

/// Implements [`subxt::Config`] for substrate chains with keccak as their hashing algorithm
#[derive(Clone)]
pub struct Hyperbridge;

/// A type that can hash values using the keccak_256 algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode)]
pub struct RuntimeHasher;

impl Hasher for RuntimeHasher {
	type Output = H256;
	fn hash(s: &[u8]) -> Self::Output {
		keccak_256(s).into()
	}
}

impl subxt::Config for Hyperbridge {
	type Hash = H256;
	type AccountId = AccountId32;
	type Address = MultiAddress<Self::AccountId, u32>;
	type Signature = MultiSignature;
	type Hasher = RuntimeHasher;
	type Header = SubstrateHeader<u32, RuntimeHasher>;
	type ExtrinsicParams = PolkadotExtrinsicParams<Self>;
}
