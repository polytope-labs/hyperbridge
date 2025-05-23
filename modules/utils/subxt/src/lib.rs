use anyhow::anyhow;
use codec::Encode;
use derivative::Derivative;
use ismp::{consensus::StateMachineHeight, host::StateMachine};
use polkadot_sdk::*;
use sp_crypto_hashing::{blake2_128, keccak_256, twox_128, twox_64};
use subxt::{
	config::{
		extrinsic_params::{BaseExtrinsicParams, BaseExtrinsicParamsBuilder},
		polkadot::PlainTip,
		substrate::{BlakeTwo256, SubstrateHeader},
		ExtrinsicParams, Hasher,
	},
	tx::TxPayload,
	utils::{AccountId32, MultiAddress, H256},
	Metadata,
};

pub mod client;

#[cfg(feature = "std")]
pub use signer::*;

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
	type ExtrinsicParams = PolkadotExtrinsicParams<Self>;
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

#[derive(Derivative)]
#[derivative(Debug(bound = "Tip: core::fmt::Debug"))]
pub struct BasicExtrinsicParamWithCheckMetadata<T: subxt::Config, Tip: core::fmt::Debug>(
	BaseExtrinsicParams<T, Tip>,
);

impl<T: subxt::Config, Tip: core::fmt::Debug + Encode + 'static> ExtrinsicParams<T::Hash>
	for BasicExtrinsicParamWithCheckMetadata<T, Tip>
{
	type OtherParams = BaseExtrinsicParamsBuilder<T, Tip>;

	fn new(
		// Provided from subxt client:
		spec_version: u32,
		transaction_version: u32,
		nonce: u64,
		genesis_hash: T::Hash,
		// Provided externally:
		other_params: Self::OtherParams,
	) -> Self {
		Self(BaseExtrinsicParams::new(
			spec_version,
			transaction_version,
			nonce,
			genesis_hash,
			other_params,
		))
	}

	fn encode_extra_to(&self, v: &mut Vec<u8>) {
		self.0.encode_extra_to(v);
		// frame_metadata_hash_extension::CheckMetadataHash::encode_to_extra
		// reference https://github.com/paritytech/subxt/blob/90b47faad85c34382f086e2cc886da8574453c36/core/src/config/signed_extensions.rs#L58
		// Mode `0` means that the metadata hash is not added
		0u8.encode_to(v);
	}

	fn encode_additional_to(&self, v: &mut Vec<u8>) {
		self.0.encode_additional_to(v);
		// frame_metadata_hash_extension::CheckMetadataHash::encode_additional_to
		// https://github.com/paritytech/polkadot-sdk/blob/743dc632fd6115b408376a6e4efe815bd804cd52/substrate/frame/metadata-hash-extension/src/lib.rs#L142
		// We don't use metadata hash in subxt so the it should be encoded as None
		None::<()>.encode_to(v);
	}
}

pub type PolkadotExtrinsicParams<T> = BasicExtrinsicParamWithCheckMetadata<T, PlainTip>;

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
		tip: Option<PlainTip>,
	) -> Result<T::Hash, anyhow::Error>
	where
		<T::ExtrinsicParams as ExtrinsicParams<T::Hash>>::OtherParams:
			Default + Send + Sync + From<BaseExtrinsicParamsBuilder<T, PlainTip>>,
		T::Signature: From<MultiSignature> + Send + Sync,
	{
		let other_params = BaseExtrinsicParamsBuilder::new().tip(tip.unwrap_or_default());
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
		Ok(extrinsic.block_hash())
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

pub fn state_machine_update_time_storage_key(height: StateMachineHeight) -> Vec<u8> {
	let pallet_prefix = twox_128(b"Ismp").to_vec();

	let storage_prefix = twox_128(b"StateMachineUpdateTime").to_vec();
	let key_1 = twox_64(&height.encode()).to_vec();

	[pallet_prefix, storage_prefix, key_1, height.encode()].concat()
}

pub fn state_machine_commitment_storage_key(height: StateMachineHeight) -> Vec<u8> {
	let pallet_prefix = twox_128(b"Ismp").to_vec();

	let storage_prefix = twox_128(b"StateCommitments").to_vec();
	let key_1 = blake2_128(&height.encode()).to_vec();

	[pallet_prefix, storage_prefix, key_1, height.encode()].concat()
}

pub fn host_params_storage_key(state_machine: StateMachine) -> Vec<u8> {
	let pallet_prefix = twox_128(b"HostExecutive").to_vec();

	let storage_prefix = twox_128(b"HostParams").to_vec();
	let key_1 = twox_64(&state_machine.encode()).to_vec();

	[pallet_prefix, storage_prefix, key_1, state_machine.encode()].concat()
}

pub fn fisherman_storage_key(address: Vec<u8>) -> Vec<u8> {
	let address = {
		let mut dest = [0u8; 32];
		dest.copy_from_slice(&address);
		dest
	};
	let pallet_prefix = twox_128(b"Fishermen").to_vec();

	let storage_prefix = twox_128(b"Fishermen").to_vec();
	let key_1 = twox_64(&address.encode()).to_vec();

	[pallet_prefix, storage_prefix, key_1, address.encode()].concat()
}
