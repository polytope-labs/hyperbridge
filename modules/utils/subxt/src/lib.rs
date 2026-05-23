#![allow(dead_code)]

use anyhow::anyhow;
use codec::Encode;
use polkadot_sdk::*;
use sp_crypto_hashing::{blake2_128, keccak_256, twox_128, twox_64};
use subxt::{
	config::{
		substrate::{BlakeTwo256, SubstrateExtrinsicParams, SubstrateHeader},
		Hasher,
	},
	tx::Payload,
	utils::{AccountId32, MultiAddress, H256},
};

use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
};
#[cfg(feature = "std")]
pub use signer::*;

pub mod client;
pub mod values;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Hyperbridge;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode)]
pub struct RuntimeHasher;

impl Hasher for RuntimeHasher {
	type Output = H256;

	fn new(_metadata: &subxt::metadata::types::Metadata) -> Self {
		Self
	}

	fn hash(&self, s: &[u8]) -> Self::Output {
		keccak_256(s).into()
	}
}

impl subxt::Config for Hyperbridge {
	type AccountId = AccountId32;
	type Address = MultiAddress<Self::AccountId, u32>;
	type Signature = subxt::utils::MultiSignature;
	type Hasher = RuntimeHasher;
	type Header = SubstrateHeader<u32, RuntimeHasher>;
	type ExtrinsicParams = SubstrateExtrinsicParams<Self>;
	type AssetId = ();
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlakeSubstrateChain;

impl subxt::Config for BlakeSubstrateChain {
	type AccountId = AccountId32;
	type Address = MultiAddress<Self::AccountId, ()>;
	type Signature = subxt::utils::MultiSignature;
	type Hasher = BlakeTwo256;
	type Header = SubstrateHeader<u32, BlakeTwo256>;
	type ExtrinsicParams = SubstrateExtrinsicParams<Self>;
	type AssetId = ();
}

#[cfg(feature = "std")]
pub mod signer {
	use anyhow::Context;
	use polkadot_sdk::sp_core::{sr25519, Pair};
	use subxt::{
		config::{DefaultExtrinsicParamsBuilder, ExtrinsicParams, HashFor},
		tx::{DefaultParams, Signer, TxInBlock, TxProgress, TxStatus},
		utils::{AccountId32, MultiSignature},
		OnlineClient,
	};

	use super::*;

	#[derive(Clone)]
	pub struct InMemorySigner<T: subxt::Config> {
		pub account_id: T::AccountId,
		pub signer: sr25519::Pair,
	}

	impl<T: subxt::Config> InMemorySigner<T>
	where
		T::Signature: From<MultiSignature>,
		T::AccountId: From<AccountId32> + Into<T::Address> + Clone + 'static + Send + Sync,
	{
		pub fn new(pair: sr25519::Pair) -> Self {
			let binding = pair.public();
			let public_key_slice: &[u8] = binding.as_ref();

			let public_key_array: [u8; 32] =
				public_key_slice.try_into().expect("sr25519 public key should be 32 bytes");

			let account_id = AccountId32::from(public_key_array);
			InMemorySigner { account_id: account_id.into(), signer: pair }
		}
	}

	impl<T: subxt::Config> Signer<T> for InMemorySigner<T>
	where
		T::AccountId: Into<T::Address> + Clone + 'static,
		T::Signature: From<MultiSignature>,
	{
		fn account_id(&self) -> T::AccountId {
			self.account_id.clone()
		}

		fn sign(&self, payload: &[u8]) -> T::Signature {
			MultiSignature::Sr25519(<[u8; 64]>::from(self.signer.sign(payload))).into()
		}
	}

	pub async fn send_extrinsic<T: subxt::Config, Tx: Payload>(
		client: &OnlineClient<T>,
		signer: &InMemorySigner<T>,
		payload: &Tx,
		_tip: Option<u128>,
		wait_for_finalization: bool,
	) -> Result<HashFor<T>, anyhow::Error>
	where
		T::AccountId: Into<T::Address> + Clone + 'static,
		T::Signature: From<MultiSignature> + Send + Sync,
		<T::ExtrinsicParams as ExtrinsicParams<T>>::Params: Send + Sync + DefaultParams,
	{
		let params = DefaultParams::default_params();
		let ext = client.tx().create_signed(payload, signer, params).await?;
		let progress = ext.submit_and_watch().await.context("Failed to submit signed extrinsic")?;
		await_extrinsic::<T>(progress, wait_for_finalization).await
	}

	/// Like [`send_extrinsic`], but submits with an explicit `nonce` rather than letting subxt
	/// fetch one from the chain.
	///
	/// subxt sources the auto nonce from the latest *finalized* block (via its internal
	/// `inject_account_nonce_and_block`). On a parachain, finality trails the best chain by
	/// several blocks, so a submitter that only waits for in-block (not finalization) before its
	/// next submission reuses an already-spent nonce and the node rejects it as
	/// `InvalidTransaction::Stale`. Passing a pool-aware nonce (e.g. from `system_accountNextIndex`)
	/// avoids that. This uses `create_partial_offline`, which — unlike `create_signed` — does not
	/// overwrite the nonce we set.
	pub async fn send_extrinsic_with_nonce<T, Tx: Payload>(
		client: &OnlineClient<T>,
		signer: &InMemorySigner<T>,
		payload: &Tx,
		nonce: u64,
		wait_for_finalization: bool,
	) -> Result<HashFor<T>, anyhow::Error>
	where
		T: subxt::Config<ExtrinsicParams = SubstrateExtrinsicParams<T>>,
		T::AccountId: Into<T::Address> + Clone + 'static,
		T::Signature: From<MultiSignature> + Send + Sync,
	{
		let params = DefaultExtrinsicParamsBuilder::<T>::new().nonce(nonce).build();
		let mut partial = client.tx().create_partial_offline(payload, params)?;
		let ext = partial.sign(signer);
		let progress = ext.submit_and_watch().await.context("Failed to submit signed extrinsic")?;
		await_extrinsic::<T>(progress, wait_for_finalization).await
	}

	/// Drive a submitted extrinsic to inclusion (or finalization), assert it executed
	/// successfully, and return the hash of the block it landed in.
	async fn await_extrinsic<T: subxt::Config>(
		progress: TxProgress<T, OnlineClient<T>>,
		wait_for_finalization: bool,
	) -> Result<HashFor<T>, anyhow::Error> {
		let ext_hash = progress.extrinsic_hash();

		let extrinsic = if wait_for_finalization {
			match progress.wait_for_finalized().await {
				Ok(p) => p,
				Err(err) => Err(refine_subxt_error(err)).context(format!(
					"Error waiting for signed extrinsic in block with hash {ext_hash:?}"
				))?,
			}
		} else {
			wait_for_inblock::<T>(progress).await?
		};

		match extrinsic.wait_for_success().await {
			Ok(p) => p,
			Err(err) =>
				Err(err).context(format!("Error executing signed extrinsic {ext_hash:?}"))?,
		};
		Ok(extrinsic.block_hash())
	}

	/// Resolve once the extrinsic appears in a (best) block, without waiting for finality.
	async fn wait_for_inblock<T: subxt::Config>(
		mut progress: TxProgress<T, OnlineClient<T>>,
	) -> Result<TxInBlock<T, OnlineClient<T>>, anyhow::Error> {
		let ext_hash = progress.extrinsic_hash();
		while let Some(status) = progress.next().await {
			match status? {
				TxStatus::InFinalizedBlock(s) | TxStatus::InBestBlock(s) => return Ok(s),
				TxStatus::Error { .. } | TxStatus::Invalid { .. } | TxStatus::Dropped { .. } =>
					return Err(anyhow!(
						"signed extrinsic {ext_hash:?} failed before reaching a block"
					)),
				_ => {},
			}
		}
		Err(anyhow!("signed extrinsic {ext_hash:?} stream ended without in-block status"))
	}
}

pub fn refine_subxt_error(err: subxt::Error) -> anyhow::Error {
	match err {
		subxt::Error::Runtime(subxt::error::DispatchError::Module(ref err)) => {
			anyhow!(err.to_string())
		},
		_ => anyhow!(err),
	}
}

pub fn relayer_account_balance_storage_key(
	state_machine: StateMachine,
	address: Vec<u8>,
) -> Vec<u8> {
	let pallet_prefix = twox_128(b"Relayer").to_vec();

	let storage_prefix = twox_128(b"Fees").to_vec();
	let key_1 = blake2_128(&state_machine.encode()).to_vec();
	let key_2 = blake2_128(&address.encode()).to_vec();

	[pallet_prefix, storage_prefix, key_1, state_machine.encode(), key_2, address.encode()].concat()
}

pub fn relayer_nonce_storage_key(address: Vec<u8>, state_machine: StateMachine) -> Vec<u8> {
	let pallet_prefix = twox_128(b"Relayer").to_vec();

	let storage_prefix = twox_128(b"Nonce").to_vec();
	let key_1 = blake2_128(&address.encode()).to_vec();
	let key_2 = blake2_128(&state_machine.encode()).to_vec();

	[pallet_prefix, storage_prefix, key_1, address.encode(), key_2, state_machine.encode()].concat()
}

/// Storage key for `pallet_ismp_relayer::OutboundConsensusRotationsClaimed[(destination, set_id)]`.
/// Both keys hash with `Blake2_128Concat` so the layout is
/// `twox_128("Relayer") || twox_128("OutboundConsensusRotationsClaimed")
/// || blake2_128(destination) || destination || blake2_128(set_id) || set_id`.
///
/// Used by the outbound-claim task to short-circuit claims that some
/// other relayer already redeemed: the storage value is `()`, so a
/// non-empty raw fetch at this key means the `(destination, set_id)`
/// is closed and the local row should be dropped instead of submitted.
pub fn outbound_consensus_rotations_claimed_storage_key(
	destination: StateMachine,
	set_id: u64,
) -> Vec<u8> {
	let pallet_prefix = twox_128(b"Relayer").to_vec();
	let storage_prefix = twox_128(b"OutboundConsensusRotationsClaimed").to_vec();
	let key_1 = blake2_128(&destination.encode()).to_vec();
	let key_2 = blake2_128(&set_id.encode()).to_vec();

	[pallet_prefix, storage_prefix, key_1, destination.encode(), key_2, set_id.encode()].concat()
}

pub fn state_machine_update_time_storage_key(height: StateMachineHeight) -> Vec<u8> {
	let pallet_prefix = twox_128(b"Ismp").to_vec();
	let storage_prefix = twox_128(b"BoundedStateMachineUpdateTime").to_vec();
	let key_1 = blake2_128(&height.id.encode()).to_vec();
	let key_2 = blake2_128(&height.height.encode()).to_vec();

	[pallet_prefix, storage_prefix, key_1, height.id.encode(), key_2, height.height.encode()]
		.concat()
}

/// Storage key for `pallet_ismp_optimism::StateMachinesDisputeGameFactoriesTypes` at
/// `state_machine_id`. The map uses `Blake2_128Concat` hashing.
pub fn optimism_game_type_configs_storage_key(state_machine_id: StateMachineId) -> Vec<u8> {
	let pallet_prefix = twox_128(b"IsmpOptimism").to_vec();
	let storage_prefix = twox_128(b"StateMachinesDisputeGameFactoriesTypes").to_vec();
	let encoded = state_machine_id.encode();
	let hashed = blake2_128(&encoded).to_vec();

	[pallet_prefix, storage_prefix, hashed, encoded].concat()
}

pub fn state_machine_commitment_storage_key(height: StateMachineHeight) -> Vec<u8> {
	let pallet_prefix = twox_128(b"Ismp").to_vec();
	let storage_prefix = twox_128(b"BoundedStateCommitments").to_vec();
	let key_1 = blake2_128(&height.id.encode()).to_vec();
	let key_2 = blake2_128(&height.height.encode()).to_vec();

	[pallet_prefix, storage_prefix, key_1, height.id.encode(), key_2, height.height.encode()]
		.concat()
}

pub fn host_params_storage_key(state_machine: StateMachine) -> Vec<u8> {
	let pallet_prefix = twox_128(b"HostExecutive").to_vec();

	let storage_prefix = twox_128(b"HostParams").to_vec();
	let key_1 = twox_64(&state_machine.encode()).to_vec();

	[pallet_prefix, storage_prefix, key_1, state_machine.encode()].concat()
}
