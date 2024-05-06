#![cfg(test)]

use std::collections::HashMap;

use anyhow::{anyhow, Context};
use codec::Encode;
use futures::StreamExt;
use ismp::host::StateMachine;
use pallet_ismp_rpc::BlockNumberOrHash;
use sp_runtime::MultiAddress;
use staging_xcm::{
	v3::{Junction, Junctions, MultiLocation, NetworkId, WeightLimit},
	VersionedMultiAssets, VersionedMultiLocation,
};
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder,
		polkadot::{PlainTip, PolkadotExtrinsicParams},
		substrate::SubstrateHeader,
		ExtrinsicParams, Hasher, Header,
	},
	ext::{
		sp_core::{self, bytes::from_hex, crypto::AccountId32, keccak_256, sr25519, Pair, H256},
		sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
	},
	rpc_params,
	tx::{Signer, TxPayload},
	Error, Metadata, OnlineClient, PolkadotConfig,
};

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
	fn encode_call_data_to(&self, metadata: &Metadata, out: &mut Vec<u8>) -> Result<(), Error> {
		// encode the pallet index
		let pallet = metadata.pallet_by_name_err(&self.pallet_name)?;
		let call_index = pallet
			.call_variant_by_name(&self.call_name)
			.ok_or_else(|| {
				Error::Other(format!(
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

#[derive(Clone)]
pub struct InMemorySigner<T: subxt::Config> {
	pub account_id: T::AccountId,
	pub signer: sr25519::Pair,
}

impl<T: subxt::Config> InMemorySigner<T>
where
	T::Signature: From<MultiSignature> + Send + Sync,
	T::AccountId:
		From<sp_core::crypto::AccountId32> + Into<T::Address> + Clone + 'static + Send + Sync,
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
		Err(err) => Err(err).context(format!(
			"Error waiting for signed extrinsic in block with hash {ext_hash:?}"
		))?,
	};

	match extrinsic.wait_for_success().await {
		Ok(p) => p,
		Err(err) => Err(err).context(format!("Error executing signed extrinsic {ext_hash:?}"))?,
	};
	Ok(())
}

const SEND_AMOUNT: u128 = 2_000_000_000_000;

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

#[ignore]
#[tokio::test]
async fn should_dispatch_ismp_request_when_xcm_is_received() -> anyhow::Result<()> {
	dotenv::dotenv().ok();
	let private_key = std::env::var("SUBSTRATE_SIGNING_KEY").ok().unwrap_or(
		"0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string(),
	);
	let seed = from_hex(&private_key)?;
	let pair = sr25519::Pair::from_seed_slice(&seed)?;
	let signer = InMemorySigner::<PolkadotConfig>::new(pair.clone());
	let url = std::env::var("ROCOCO_LOCAL_URL")
		.ok()
		.unwrap_or("ws://127.0.0.1:9922".to_string());
	let client = OnlineClient::<PolkadotConfig>::from_url(&url).await?;

	let para_url = std::env::var("PARA_LOCAL_URL")
		.ok()
		.unwrap_or("ws://127.0.0.1:9990".to_string());
	let para_client = OnlineClient::<Hyperbridge>::from_url(&para_url).await?;

	// Wait for parachain block production

	let sub = para_client.rpc().subscribe_all_block_headers().await?;
	let _block = sub
		.take(1)
		.collect::<Vec<_>>()
		.await
		.into_iter()
		.collect::<Result<Vec<_>, _>>()?;
	let beneficiary: MultiLocation = Junctions::X3(
		Junction::AccountId32 { network: None, id: pair.public().into() },
		Junction::AccountKey20 {
			network: Some(NetworkId::Ethereum { chain_id: 1 }),
			key: [1u8; 20],
		},
		Junction::GeneralIndex(60 * 60),
	)
	.into_location();
	let weight_limit = WeightLimit::Unlimited;

	let dest: MultiLocation = Junction::Parachain(2000).into();

	let call = (
		Box::<VersionedMultiLocation>::new(dest.clone().into()),
		Box::<VersionedMultiLocation>::new(beneficiary.clone().into()),
		Box::<VersionedMultiAssets>::new((Junctions::Here, SEND_AMOUNT).into()),
		0,
		weight_limit,
	);

	{
		let signer = InMemorySigner::<PolkadotConfig>::new(pair.clone());
		// Force set the xcm version to our supported version
		let encoded_call =
			Extrinsic::new("XcmPallet", "force_xcm_version", (Box::new(dest.clone()), 3).encode())
				.encode_call_data(&client.metadata())?;
		let tx = Extrinsic::new("Sudo", "sudo", encoded_call);
		send_extrinsic(&client, signer, tx).await?;
	}

	let ext = Extrinsic {
		pallet_name: "XcmPallet".to_string(),
		call_name: "limited_reserve_transfer_assets".to_string(),
		encoded: call.encode(),
	};

	send_extrinsic(&client, signer, ext).await?;

	let sub = para_client.rpc().subscribe_finalized_block_headers().await?;
	// Give enough time for the message to be processed
	let block = sub
		.take(8)
		.collect::<Vec<_>>()
		.await
		.into_iter()
		.collect::<Result<Vec<_>, _>>()?;
	let current_block = block
		.last()
		.cloned()
		.ok_or_else(|| anyhow!("Finalized heads missing"))?
		.number();

	let params = rpc_params![
		BlockNumberOrHash::<H256>::Number(1),
		BlockNumberOrHash::<H256>::Number(current_block)
	];
	let response: HashMap<String, Vec<ismp::events::Event>> =
		para_client.rpc().request("ismp_queryEvents", params).await?;

	let events = response.values().into_iter().cloned().flatten().collect::<Vec<_>>();
	let post = match events.get(0).cloned().ok_or_else(|| anyhow!("Ismp Event should exist"))? {
		ismp::events::Event::PostRequest(post) => post,
		_ => Err(anyhow!("Unexpected event"))?,
	};

	dbg!(&post);

	// Assert that this is the post we sent
	assert_eq!(post.nonce, 0);
	assert_eq!(post.dest, StateMachine::Ethereum(ismp::host::Ethereum::ExecutionLayer));
	assert_eq!(post.source, StateMachine::Kusama(2000));
	Ok(())
}
