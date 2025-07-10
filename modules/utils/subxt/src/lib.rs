use anyhow::anyhow;
use codec::Encode;
use derivative::Derivative;
use ismp::{consensus::StateMachineHeight, host::StateMachine};
use polkadot_sdk::*;
use sp_crypto_hashing::{blake2_128, keccak_256, twox_128, twox_64};
use subxt::{
	config::{
		substrate::{
			BlakeTwo256, SubstrateExtrinsicParams, SubstrateExtrinsicParamsBuilder,
			SubstrateHeader,
		},
		Hasher,
	},
	tx::Payload,
	utils::{AccountId32, MultiAddress, H256},
	Metadata,
};

pub mod client;

#[cfg(feature = "std")]
pub use signer::*;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Hyperbridge;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode)]
pub struct RuntimeHasher;

impl Hasher for RuntimeHasher {
	type Output = H256;

	fn new(metadata: &subxt::metadata::types::Metadata) -> Self {
		Self
	}

	fn hash(&self, s: &[u8]) -> Self::Output {
		keccak_256(s).into()
	}
}

impl subxt::Config for Hyperbridge {
	type AccountId = AccountId32;
	type Address = MultiAddress<Self::AccountId, ()>;
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
	use super::*;
	use anyhow::Context;
	use subxt::{
		tx::Signer,
		OnlineClient,
	};
	use polkadot_sdk::sp_core::{crypto, sr25519, Pair};
	use polkadot_sdk::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner};
	use subxt::config::{DefaultExtrinsicParamsBuilder, ExtrinsicParams, HashFor};
	use subxt::tx::DefaultParams;

	#[derive(Clone)]
	pub struct InMemorySigner<T: subxt::Config> {
		pub account_id: T::AccountId,
		pub signer: sr25519::Pair,
	}

	impl<T: subxt::Config> InMemorySigner<T>
	where
		T::Signature: From<MultiSignature>,
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
		T::Signature: From<MultiSignature>,
	{
		fn account_id(&self) ->  T::AccountId {
			self.account_id.clone()
		}

		fn sign(&self, payload: &[u8]) -> T::Signature {
			MultiSignature::Sr25519(self.signer.sign(payload)).into()
		}
	}

	pub async fn send_extrinsic<T: subxt::Config, Tx: Payload>(
		client: &OnlineClient<T>,
		signer: &InMemorySigner<T>,
		payload: &Tx,
	) -> Result<HashFor<T>, anyhow::Error>
	where
		T::AccountId: Into<T::Address> + Clone + 'static,
		T::Signature: From<MultiSignature> + Send + Sync,
		<T::ExtrinsicParams as ExtrinsicParams<T>>::Params: DefaultParams,
	{
		let params = DefaultParams::default_params();
		let ext = client.tx().create_signed(payload, signer, params).await?;
		let progress = ext.submit_and_watch().await.context("Failed to submit signed extrinsic")?;
		let ext_hash = progress.extrinsic_hash();

		let extrinsic = match progress.wait_for_finalized().await {
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

pub fn refine_subxt_error(err: subxt::Error) -> anyhow::Error {
	match err {
		subxt::Error::Runtime(subxt::error::DispatchError::Module(ref err)) => {
			anyhow!(err.to_string())
		},
		_ => anyhow!(err),
	}
}
