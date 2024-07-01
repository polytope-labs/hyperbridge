use anyhow::anyhow;
use codec::Encode;
use sp_core_hashing::keccak_256;
use subxt::{
	config::{
		polkadot::PolkadotExtrinsicParams,
		substrate::{BlakeTwo256, SubstrateExtrinsicParams, SubstrateHeader},
		Hasher,
	},
	tx::TxPayload,
	utils::{AccountId32, MultiAddress, H256},
	Metadata,
};

pub mod client;
pub mod gargantua;

#[cfg(feature = "std")]
pub use signer::*;

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
                        state_machines: params
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
	type Signature = subxt::utils::MultiSignature;
	type Hasher = RuntimeHasher;
	type Header = SubstrateHeader<u32, RuntimeHasher>;
	type ExtrinsicParams = PolkadotExtrinsicParams<Self>;
}

/// Implements [`subxt::Config`] for substrate chains with blake2b as their hashing algorithm
#[derive(Clone)]
pub struct BlakeSubstrateChain;

impl subxt::Config for BlakeSubstrateChain {
	type Hash = H256;
	type AccountId = AccountId32;
	type Address = MultiAddress<Self::AccountId, u32>;
	type Signature = subxt::utils::MultiSignature;
	type Hasher = BlakeTwo256;
	type Header = SubstrateHeader<u32, BlakeTwo256>;
	type ExtrinsicParams = SubstrateExtrinsicParams<Self>;
}

/// Implements [`TxPayload`] for extrinsic encoding
pub struct Extrinsic {
	/// The pallet name, used to query the metadata
	pallet_name: String,
	/// The call name
	call_name: String,
	/// The encoded pallet call. Note that this should be the pallet call. Not runtime call
	encoded: Vec<u8>,
}

impl Extrinsic {
	/// Creates a new extrinsic ready to be sent with subxt.
	pub fn new(
		pallet_name: impl Into<String>,
		call_name: impl Into<String>,
		encoded_call: Vec<u8>,
	) -> Self {
		Extrinsic {
			pallet_name: pallet_name.into(),
			call_name: call_name.into(),
			encoded: encoded_call,
		}
	}
}

impl TxPayload for Extrinsic {
	fn encode_call_data_to(
		&self,
		metadata: &Metadata,
		out: &mut Vec<u8>,
	) -> Result<(), subxt::error::Error> {
		// encode the pallet index
		let pallet = metadata.pallet_by_name_err(&self.pallet_name)?;
		let call_index = pallet
			.call_variant_by_name(&self.call_name)
			.ok_or_else(|| {
				subxt::error::Error::Other(format!(
					"Can't find {} in pallet {} metadata",
					self.call_name, self.pallet_name
				))
			})?
			.index;
		let pallet_index = pallet.index();
		pallet_index.encode_to(out);
		call_index.encode_to(out);

		// copy the encoded call to out
		out.extend_from_slice(&self.encoded);

		Ok(())
	}
}

#[cfg(feature = "std")]
pub mod signer {
	use super::*;
	use anyhow::Context;
	use subxt::{
		config::{
			extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams,
		},
		ext::{
			sp_core::{crypto, sr25519, Pair},
			sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
		},
		tx::Signer,
		OnlineClient,
	};

	#[derive(Clone)]
	pub struct InMemorySigner<T: subxt::Config> {
		pub account_id: T::AccountId,
		pub signer: sr25519::Pair,
	}

	impl<T: subxt::Config> InMemorySigner<T>
	where
		T::Signature: From<MultiSignature> + Send + Sync,
		T::AccountId: From<crypto::AccountId32> + Into<T::Address> + Clone + 'static + Send + Sync,
	{
		pub fn new(pair: sr25519::Pair) -> Self {
			InMemorySigner {
				account_id: MultiSigner::Sr25519(pair.public()).into_account().into(),
				signer: pair,
			}
		}
	}

	impl<T: subxt::Config> Signer<T> for InMemorySigner<T>
	where
		T::AccountId: Into<T::Address> + Clone + 'static,
		T::Signature: From<MultiSignature> + Send + Sync,
	{
		fn account_id(&self) -> T::AccountId {
			self.account_id.clone()
		}

		fn address(&self) -> T::Address {
			self.account_id.clone().into()
		}

		fn sign(&self, payload: &[u8]) -> T::Signature {
			MultiSignature::Sr25519(self.signer.sign(&payload)).into()
		}
	}

	/// Send a transaction
	pub async fn send_extrinsic<T: subxt::Config, Tx: TxPayload>(
		client: &OnlineClient<T>,
		signer: InMemorySigner<T>,
		payload: Tx,
	) -> Result<(), anyhow::Error>
	where
		<T::ExtrinsicParams as ExtrinsicParams<T::Hash>>::OtherParams:
			Default + Send + Sync + From<BaseExtrinsicParamsBuilder<T, PlainTip>>,
		T::Signature: From<MultiSignature> + Send + Sync,
	{
		let other_params = BaseExtrinsicParamsBuilder::new();
		let ext = client.tx().create_signed(&payload, &signer, other_params.into()).await?;
		let progress = ext.submit_and_watch().await.context("Failed to submit signed extrinsic")?;
		let ext_hash = progress.extrinsic_hash();

		let extrinsic = match progress.wait_for_in_block().await {
			Ok(p) => p,
			Err(err) => Err(refine_subxt_error(err)).context(format!(
				"Error waiting for signed extrinsic in block with hash {ext_hash:?}"
			))?,
		};

		match extrinsic.wait_for_success().await {
			Ok(p) => p,
			Err(err) =>
				Err(err).context(format!("Error executing signed extrinsic {ext_hash:?}"))?,
		};
		Ok(())
	}
}

/// This prevents the runtime metadata from being displayed when module errors are encountered
pub fn refine_subxt_error(err: subxt::Error) -> anyhow::Error {
	match err {
		subxt::Error::Runtime(subxt::error::DispatchError::Module(ref err)) => {
			anyhow!(err.to_string())
		},
		_ => anyhow!(err),
	}
}
